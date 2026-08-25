use std::io::Write;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_channel::{Receiver, Sender};
use gpui::{Context, WeakEntity};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use vt100::Parser;

const SCROLLBACK_LINES: usize = 10000;

const ZSH_CODEPOINT_BACKSPACE: &str = r#"
vterm_backward_delete_codepoint() {
    if (( CURSOR > 0 )); then
        LBUFFER="${LBUFFER[1,$((CURSOR - 1))]}${LBUFFER[$((CURSOR + 1)),-1]}"
    fi
}
zle -N vterm-backward-delete-codepoint vterm_backward_delete_codepoint
bindkey '^?' vterm-backward-delete-codepoint
bindkey -M emacs '^?' vterm-backward-delete-codepoint
bindkey -M viins '^?' vterm-backward-delete-codepoint
"#;

/// Re-exported so downstream UI crates can interpret cell colors without
/// depending on the emulator directly.
pub use vt100;

pub struct PtyTerminal {
    pub parser: Arc<Mutex<Parser>>,
    writer: Box<dyn Write + Send>,
    master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn Child + Send>>,
    pub child_pid: Option<u32>,
    pub session_name: Option<String>,
    /// Accumulates fractional scroll lines from trackpad events so that small
    /// deltas aren't silently truncated to zero.
    scroll_accumulator: f32,
}

impl PtyTerminal {
    pub fn new_with_cwd(
        cwd: Option<String>,
        requested_session: Option<String>,
        rows: u16,
        cols: u16,
        cx: &mut Context<Self>,
    ) -> Self {
        let parser = Arc::new(Mutex::new(Parser::new(rows, cols, SCROLLBACK_LINES)));
        let parser_clone = parser.clone();

        let pty_system = native_pty_system();
        let pair = match pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("vterm: failed to open pty: {e}");
                return dead_terminal(parser, None);
            }
        };

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string());
        let zsh_integration = (shell.rsplit('/').next() == Some("zsh"))
            .then(prepare_zsh_integration)
            .flatten();
        let mut session_name = None;
        let mut cmd = CommandBuilder::new(&shell);
        cmd.args(["-l"]);
        if let Some(cwd) = cwd.as_deref() {
            cmd.cwd(cwd);
        }

        #[cfg(target_os = "macos")]
        if screen_available() {
            let name = requested_session
                .filter(|name| valid_session_name(name))
                .unwrap_or_else(new_session_name);
            session_name = Some(name.clone());
            cmd = CommandBuilder::new("screen");
            if let Some(screen_target) = screen_socket(&name) {
                cmd.args(["-A", "-xRR", &screen_target]);
            } else {
                cmd.args(["-S", &name, &shell, "-l"]);
            }
            if let Some(cwd) = cwd.as_deref() {
                cmd.cwd(cwd);
            }
        }

        // Set AFTER the screen wrap above: replacing the CommandBuilder
        // discards earlier env() calls. Without these, children inherit
        // vterm's own launch env (empty TERM/COLORTERM when started from
        // Finder/Dock), and CLIs downgrade to a reduced color palette.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // Children inherit vterm's own process env, which — when vterm was
        // launched from another terminal (e.g. `cargo run` inside Zed) —
        // contains that terminal's identity vars. CLIs key integrations off
        // them, so replace them with vterm's own identity.
        cmd.env(
            "TERM_PROGRAM",
            std::env::var("VTERM_APP_NAME").unwrap_or_else(|_| "vterm".to_string()),
        );
        cmd.env(
            "TERM_PROGRAM_VERSION",
            std::env::var("VTERM_DEV_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
        );
        for foreign in [
            "ZED_TERM",
            "ITERM_SESSION_ID",
            "ITERM_PROFILE",
            "WEZTERM_EXECUTABLE",
            "WEZTERM_PANE",
            "KITTY_WINDOW_ID",
            "KITTY_PID",
        ] {
            cmd.env_remove(foreign);
        }
        // The terminal pane is always dark (see workspace terminal.rs), so
        // advertise that: CLIs that skip color queries (codex, vim, …) use
        // COLORFGBG to pick their dark palette. Note opencode ignores it.
        cmd.env("COLORFGBG", "15;0");
        if let Some((config_dir, original_zdotdir)) = zsh_integration {
            cmd.env("ZDOTDIR", config_dir.to_string_lossy().as_ref());
            cmd.env("_VTERM_ORIGINAL_ZDOTDIR", original_zdotdir);
        }

        let child = match pair.slave.spawn_command(cmd) {
            Ok(child) => child,
            Err(e) => {
                eprintln!("vterm: failed to spawn shell: {e}");
                return dead_terminal(parser, session_name);
            }
        };

        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(e) => {
                eprintln!("vterm: failed to take reader: {e}");
                return dead_terminal(parser, session_name);
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(e) => {
                eprintln!("vterm: failed to take writer: {e}");
                return dead_terminal(parser, session_name);
            }
        };

        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = async_channel::unbounded();

        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.try_send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        cx.spawn(|this: WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            let mut filter = AltScreenFilter::new();
            async move {
                while let Ok(bytes) = rx.recv().await {
                    // Coalesce the whole queued burst into one parse pass so
                    // heavy output costs a single lock + repaint instead of
                    // one per 8KB chunk. Keeps typing/scrolling smooth while
                    // commands stream data.
                    let mut batch = bytes;
                    while let Ok(next) = rx.try_recv() {
                        batch.extend_from_slice(&next);
                    }
                    {
                        let mut guard = parser_clone.lock().unwrap();
                        for (segment, clear_after) in filter.feed(&batch) {
                            guard.process(&segment);
                            if clear_after {
                                let fresh = cleared_like(&mut guard);
                                *guard = fresh;
                            }
                        }
                    }
                    if this.update(&mut cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        let child_pid = child.process_id();

        Self {
            parser,
            writer: Box::new(writer),
            master: Some(pair.master),
            child: Some(child),
            child_pid,
            session_name,
            scroll_accumulator: 0.0,
        }
    }

    pub fn shutdown(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(name) = self.session_name.as_deref() {
            let _ = std::process::Command::new("screen")
                .args(["-S", name, "-X", "quit"])
                .status();
        }

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        // Any user input snaps the view back to the bottom, like real
        // terminals do when you start typing while scrolled up.
        self.parser.lock().unwrap().screen_mut().set_scrollback(0);
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        if let Some(master) = self.master.as_ref() {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
        self.parser
            .lock()
            .unwrap()
            .screen_mut()
            .set_size(rows, cols);
    }

    /// Scrolls by `delta_lines`. Positive values move toward history (up),
    /// negative back to the bottom — matching the GPUI wheel delta and
    /// alacritty/Zed conventions.
    pub fn scroll(&mut self, delta_lines: f32) {
        // Accumulate fractional scroll so that small trackpad deltas aren't lost.
        self.scroll_accumulator += delta_lines;

        // Extract whole lines from the accumulator.
        let whole_lines = self.scroll_accumulator.trunc() as i32;
        if whole_lines == 0 {
            return;
        }
        self.scroll_accumulator -= whole_lines as f32;

        let mut parser = self.parser.lock().unwrap();
        let current_offset = parser.screen().scrollback();

        let new_offset = if whole_lines > 0 {
            current_offset.saturating_add(whole_lines as usize)
        } else {
            current_offset.saturating_sub((-whole_lines) as usize)
        };

        parser.screen_mut().set_scrollback(new_offset);
    }

    pub fn scroll_info(&self) -> (usize, usize) {
        let mut parser = self.parser.lock().unwrap();
        let current = parser.screen().scrollback();
        parser.screen_mut().set_scrollback(usize::MAX);
        let max = parser.screen().scrollback();
        parser.screen_mut().set_scrollback(current);
        (current, max)
    }
}

impl Drop for PtyTerminal {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

fn prepare_zsh_integration() -> Option<(PathBuf, String)> {
    let original_zdotdir = std::env::var("ZDOTDIR")
        .ok()
        .filter(|path| !path.is_empty())
        .or_else(|| std::env::var("HOME").ok())?;

    let base = std::env::temp_dir();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let config_dir = base.join(format!("vterm-zsh-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&config_dir).ok()?;

    let source = |file: &str| {
        format!(
            "if [[ -r \"$_VTERM_ORIGINAL_ZDOTDIR/{file}\" ]]; then\n    source \"$_VTERM_ORIGINAL_ZDOTDIR/{file}\"\nfi\n"
        )
    };
    let files = [
        (".zshenv", source(".zshenv")),
        (".zprofile", source(".zprofile")),
        (
            ".zshrc",
            format!("{}{}", source(".zshrc"), ZSH_CODEPOINT_BACKSPACE),
        ),
        (
            ".zlogin",
            format!("{}{}", source(".zlogin"), ZSH_CODEPOINT_BACKSPACE),
        ),
    ];

    for (name, contents) in files {
        if std::fs::write(config_dir.join(name), contents).is_err() {
            let _ = std::fs::remove_dir_all(&config_dir);
            return None;
        }
    }

    Some((config_dir, original_zdotdir))
}

fn dead_terminal(parser: Arc<Mutex<Parser>>, session_name: Option<String>) -> PtyTerminal {
    PtyTerminal {
        parser,
        writer: Box::new(std::io::sink()),
        master: None,
        child: None,
        child_pid: None,
        session_name,
        scroll_accumulator: 0.0,
    }
}

#[cfg(target_os = "macos")]
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
fn screen_available() -> bool {
    std::process::Command::new("screen")
        .arg("-v")
        .output()
        .is_ok()
}

#[cfg(target_os = "macos")]
fn valid_session_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

#[cfg(target_os = "macos")]
fn new_session_name() -> String {
    loop {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let name = format!(
            "vterm-{}-{}-{}",
            std::process::id(),
            SESSION_COUNTER.fetch_add(1, Ordering::Relaxed),
            nonce
        );
        if screen_socket(&name).is_none() {
            return name;
        }
    }
}

#[cfg(target_os = "macos")]
fn screen_socket(name: &str) -> Option<String> {
    let output = std::process::Command::new("screen")
        .arg("-ls")
        .output()
        .ok()?;
    let suffix = format!(".{name}");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .find(|socket| socket.ends_with(&suffix))
        .map(str::to_owned)
}

/// Removes alternate-screen switching sequences (`CSI ?47h/l`, `?1047h/l`,
/// `?1049h/l`) from the PTY byte stream. Screen sends those on startup and
/// detach, and our vt100 parser records no scrollback for the alternate grid
/// — so without this, history is unscrollable while a session is wrapped.
///
/// It also detects `CSI 3J` ("erase saved lines"), which the parser ignores,
/// so callers can drop scrollback when programs run `clear`.
struct AltScreenFilter {
    /// Bytes of an escape sequence split across chunk boundaries.
    pending: Vec<u8>,
    /// Previous kept sequence was a cursor-home, awaiting an adjacent 2J.
    saw_cup_home: bool,
}

struct CsiScan {
    keep: bool,
    /// Sequence is `CSI 3J` / `CSI ?3J` (clear scrollback).
    clear_scrollback: bool,
    /// Sequence is `CSI H`-style cursor home.
    cup_home: bool,
    /// Sequence is `CSI 2J` / `CSI J`.
    erase_all: bool,
    len: usize,
}

fn scan_csi(data: &[u8]) -> Result<CsiScan, bool> {
    // Ok(scan) = full sequence; Err(consume_esc_only) = not a CSI we track.
    debug_assert!(!data.is_empty() && data[0] == 0x1b);
    if data.len() < 2 || data[1] != b'[' {
        return Err(true);
    }
    let mut i = 2;
    let dec_private = i < data.len() && data[i] == b'?';
    if dec_private {
        i += 1;
    }
    let params_start = i;
    while i < data.len() && (data[i].is_ascii_digit() || data[i] == b';') {
        i += 1;
    }
    if i >= data.len() {
        return Err(false); // incomplete; wait for more bytes
    }
    let final_byte = data[i];
    let len = i + 1;
    // CSI final bytes occupy 0x40..=0x7E.
    if !(0x40..=0x7e).contains(&final_byte) {
        return Err(true);
    }
    let params: Vec<&[u8]> = data[params_start..i].split(|&b| b == b';').collect();
    // Only DEC-private set/reset can be an alt-screen switch. Screen sends
    // its own at startup and swallows child apps' ones, so these are simply
    // stripped — they carry no usable signal for us.
    let alt_switch = dec_private
        && matches!(final_byte, b'h' | b'l')
        && params
            .iter()
            .any(|p| matches!(*p, b"47" | b"1047" | b"1049"));
    let clear_scrollback =
        final_byte == b'J' && params.iter().any(|p| matches!(*p, b"3"));
    // `CSI H` / `CSI 1;1H` — cursor home, first half of the classic clear.
    let cup_home = final_byte == b'H' && {
        let non_empty: Vec<_> = params.iter().filter(|p| !p.is_empty()).collect();
        non_empty.is_empty() || non_empty == [&&b"1"[..]] || non_empty == [&&b"1"[..], &&b"1"[..]]
            || non_empty == [&&b""[..], &&b"1"[..]] || non_empty == [&&b"1"[..], &&b""[..]]
    };
    // `CSI 2J` — erase everything, second half of the classic clear.
    let erase_all = final_byte == b'J'
        && params.iter().all(|p| p.is_empty() || *p == b"2");
    Ok(CsiScan {
        keep: !alt_switch,
        clear_scrollback,
        cup_home,
        erase_all,
        len,
    })
}

impl AltScreenFilter {
    fn new() -> Self {
        Self {
            pending: Vec::new(),
            saw_cup_home: false,
        }
    }

    /// Returns byte segments in order; `true` marks that a scrollback-clear
    /// fires right after that segment — either an explicit `3J` or the
    /// classic `home + 2J` clear pair (macOS terminfo ships no E3, so
    /// `clear` never sends 3J). Full-screen apps repaint with the same pair,
    /// so launching one also flushes history — same as tmux's behavior.
    fn feed(&mut self, chunk: &[u8]) -> Vec<(Vec<u8>, bool)> {
        let mut data = std::mem::take(&mut self.pending);
        data.extend_from_slice(chunk);

        let mut segments = Vec::new();
        let mut cur: Vec<u8> = Vec::new();
        let mut i = 0;
        while i < data.len() {
            if data[i] != 0x1b {
                cur.push(data[i]);
                i += 1;
                self.saw_cup_home = false;
                continue;
            }
            match scan_csi(&data[i..]) {
                Ok(scan) => {
                    if scan.keep {
                        cur.extend_from_slice(&data[i..i + scan.len]);
                    } else {
                        self.saw_cup_home = false;
                        i += scan.len;
                        continue;
                    }
                    let purge =
                        scan.clear_scrollback || (scan.erase_all && self.saw_cup_home);
                    self.saw_cup_home = scan.cup_home;
                    if purge {
                        segments.push((std::mem::take(&mut cur), true));
                    }
                    i += scan.len;
                }
                Err(consume_esc_only) => {
                    if consume_esc_only {
                        cur.push(data[i]);
                        i += 1;
                        self.saw_cup_home = false;
                    } else {
                        self.pending = data[i..].to_vec();
                        segments.push((cur, false));
                        return segments;
                    }
                }
            }
        }
        segments.push((cur, false));
        segments
    }
}

/// A fresh parser equal to `old` but with empty scrollback: vt100's own
/// serialization round-trips visible cells, cursor and modes, and drops
/// accumulated history — the effect terminals give `CSI 3J` after `clear`.
fn cleared_like(old: &mut Parser) -> Parser {
    // Serialize from the live view; a stale scrollback offset must not leak
    // into the snapshot.
    old.screen_mut().set_scrollback(0);
    let (rows, cols) = old.screen().size();
    let mut fresh = Parser::new(rows, cols, SCROLLBACK_LINES);
    fresh.process(&old.screen().state_formatted());
    fresh.process(&old.screen().input_mode_formatted());
    fresh.process(&old.screen().cursor_state_formatted());
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(segments: Vec<(Vec<u8>, bool)>) -> (Vec<u8>, usize) {
        let mut bytes = Vec::new();
        let mut clears = 0;
        for (seg, clear_after) in segments {
            bytes.extend_from_slice(&seg);
            if clear_after {
                clears += 1;
            }
        }
        (bytes, clears)
    }

    #[test]
    fn filters_alt_screen_switches_but_keeps_other_escapes() {
        let mut f = AltScreenFilter::new();
        let input = b"\x1b[?1049h\x1b[2Jhello\x1b[?1049l\x1b[31mred";
        assert_eq!(
            flat(f.feed(input)),
            (b"\x1b[2Jhello\x1b[31mred".to_vec(), 0)
        );
    }

    #[test]
    fn filters_47_and_1047_variants_and_keeps_other_modes() {
        let mut f = AltScreenFilter::new();
        let input = b"a\x1b[?47hb\x1b[?1047lc\x1b[?1000l\x1b[?1006hd";
        assert_eq!(
            flat(f.feed(input)),
            (
                b"abc\x1b[?1000l\x1b[?1006hd".to_vec(),
                0
            )
        );
    }

    #[test]
    fn reassembles_sequences_split_across_chunks() {
        let mut f = AltScreenFilter::new();
        let first = flat(f.feed(b"x\x1b[?10"));
        let second = flat(f.feed(b"49h y\x1b[H!"));
        assert_eq!(first, (b"x".to_vec(), 0));
        assert_eq!(second, (b" y\x1b[H!".to_vec(), 0));
    }

    #[test]
    fn lone_escape_bytes_pass_through() {
        let mut f = AltScreenFilter::new();
        assert_eq!(flat(f.feed(b"a\x1bb\x1bc")), (b"a\x1bb\x1bc".to_vec(), 0));
    }

    #[test]
    fn detects_clear_scrollback_in_both_variants() {
        // The 3J bytes themselves pass through (the parser no-ops them);
        // the flag tells the consumer to swap in a cleared parser.
        let mut f = AltScreenFilter::new();
        let (_, clears) = flat(f.feed(b"\x1b[3J"));
        assert_eq!(clears, 1);

        let mut f = AltScreenFilter::new();
        let (bytes, clears) = flat(f.feed(b"hi\x1b[?3Jbye"));
        assert_eq!(bytes, b"hi\x1b[?3Jbye".to_vec());
        assert_eq!(clears, 1);
    }

    #[test]
    fn home_plus_erase_all_purges_outside_fullscreen_apps() {
        let mut f = AltScreenFilter::new();
        let (bytes, clears) = flat(f.feed(b"out \x1b[H\x1b[2Jdone"));
        assert_eq!(bytes, b"out \x1b[H\x1b[2Jdone".to_vec());
        assert_eq!(clears, 1);
    }

    #[test]
    fn fullscreen_start_flushes_like_a_clear() {
        let mut f = AltScreenFilter::new();
        // Screen swallows child apps' alt-screen switches, so vim's first
        // paint is indistinguishable from `clear` — both flush history.
        let (bytes, clears) = flat(f.feed(b"\x1b[?1049h\x1b[H\x1b[2Jvim draws\x1b[?1049l"));
        assert_eq!(bytes, b"\x1b[H\x1b[2Jvim draws".to_vec());
        assert_eq!(clears, 1);
    }

    #[test]
    fn lone_2j_does_not_purge_without_adjacent_home() {
        let mut f = AltScreenFilter::new();
        let (_, clears) = flat(f.feed(b"x y \x1b[2J more"));
        assert_eq!(clears, 0);
    }

    #[test]
    fn cleared_parser_keeps_contents_and_drops_history() {
        fn deepest(p: &mut Parser) -> usize {
            p.screen_mut().set_scrollback(usize::MAX);
            p.screen().scrollback()
        }

        let mut old = Parser::new(4, 20, SCROLLBACK_LINES);
        // Fill history, then emulate `clear` at that boundary.
        old.process(&vec![b'\n'; 40]);
        old.process(b"visible");
        assert!(deepest(&mut old) > 0, "history exists pre-clear");

        let mut fresh = cleared_like(&mut old);
        assert_eq!(deepest(&mut fresh), 0, "history purged");
        // The text sits where the cursor was left; verify it survived.
        let (crow, ccol) = fresh.screen().cursor_position();
        let ccol = ccol as usize;
        let start = ccol.saturating_sub(7);
        let row: String = (start..ccol)
            .map(|c| {
                fresh
                    .screen()
                    .cell(crow, c as u16)                    .map(|cell| cell.contents())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(row, "visible", "live contents survive the swap");
    }
}
