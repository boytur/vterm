use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{ClearMode, Handler, NamedColor, Processor, StdSyncHandler};
use alacritty_terminal::vte::ansi::Color as AnsiColor;
use async_channel::{Receiver, Sender};
use gpui::{Context, WeakEntity};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

const SCROLLBACK_LINES: usize = 10000;

// The fixed xterm-standard palette the terminal pane renders with (the
// workspace renderer consumes these same values). Deliberately NOT derived
// from the app theme: recoloring these per-theme makes CLI tools (codex,
// opencode, vim, etc.) render with mismatched colors whenever the user
// switches themes.
pub const STANDARD_ANSI: [u32; 16] = [
    0x000000, 0xcd0000, 0x00cd00, 0xcdcd00, 0x0000ee, 0xcd00cd, 0x00cdcd, 0xe5e5e5, //
    0x7f7f7f, 0xff0000, 0x00ff00, 0xffff00, 0x5c5cff, 0xff00ff, 0x00ffff, 0xffffff,
];

// The terminal surface stays dark regardless of the app theme. CLI default
// palettes assume a dark background.
pub const TERMINAL_BG: u32 = 0x0d0d0d;
pub const TERMINAL_FG: u32 = 0xe8e8e8;

/// RGB channels for any xterm palette index (0–15 named, 16–231 color cube,
/// 232–255 grayscale) exactly as the renderer displays it. Returns `None`
/// outside 0..=256.
pub fn palette_rgb(index: usize) -> Option<[u8; 3]> {
    let channels = |hex: u32| [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8];
    match index {
        0..=15 => Some(channels(STANDARD_ANSI[index])),
        16..=231 => {
            let mut i = index - 16;
            let b = ((i % 6) * 51) as u8;
            i /= 6;
            let g = ((i % 6) * 51) as u8;
            i /= 6;
            let r = ((i % 6) * 51) as u8;
            Some([r, g, b])
        }
        232..=255 => {
            // xterm grayscale ramp: 8..=238 in steps of 10.
            let v = ((index - 232) * 10 + 8) as u8;
            Some([v, v, v])
        }
        _ => None,
    }
}

const ZSH_CODEPOINT_BACKSPACE: &str = r#"
zle_highlight+=('paste:none')
vterm_backward_delete_codepoint() {
    if (( CURSOR > 0 )); then
        LBUFFER="${LBUFFER[1,-2]}"
    fi
}
zle -N vterm-backward-delete-codepoint vterm_backward_delete_codepoint
bindkey '^?' vterm-backward-delete-codepoint
bindkey -M emacs '^?' vterm-backward-delete-codepoint
bindkey -M viins '^?' vterm-backward-delete-codepoint
"#;

/// Re-exported so downstream UI crates can interpret cell colors without
/// depending on the emulator directly.
pub use alacritty_terminal;

/// Collects emulator events (`ColorRequest`, `PtyWrite`, …) raised while
/// parsing PTY output. Drained by the read loop, which answers them in
/// order — the same lazy pattern Zed uses for its embedded alacritty core.
#[derive(Clone, Default)]
pub struct EventProxy(Arc<Mutex<Vec<Event>>>);

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }
}

/// Grid dimensions handed to alacritty on construction and resize.
#[derive(Clone, Copy)]
struct TermBounds {
    rows: u16,
    cols: u16,
}

impl Dimensions for TermBounds {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.rows as usize
    }

    fn columns(&self) -> usize {
        self.cols as usize
    }
}

/// The default-foreground/background and ANSI 0–15 palette that child apps
/// should see — shared by the OSC query responder and the renderer so they
/// can never disagree. The workspace injects the active app theme into this;
/// switches apply live to terminals that are already running (Zed's model:
/// colors resolve at query/paint time, never baked in at spawn).
#[derive(Clone)]
pub struct TerminalColors {
    inner: Arc<Mutex<ColorInner>>,
}

#[derive(Clone, Copy)]
struct ColorInner {
    fg: [u8; 3],
    bg: [u8; 3],
    ansi: [[u8; 3]; 16],
}

impl TerminalColors {
    pub fn new(fg: [u8; 3], bg: [u8; 3], ansi: [[u8; 3]; 16]) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ColorInner { fg, bg, ansi })),
        }
    }

    /// The built-in fixed dark palette, used until a theme is injected.
    pub fn dark() -> Self {
        let ansi = STANDARD_ANSI.map(|hex| [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]);
        Self::new(
            [(TERMINAL_FG >> 16) as u8, (TERMINAL_FG >> 8) as u8, TERMINAL_FG as u8],
            [(TERMINAL_BG >> 16) as u8, (TERMINAL_BG >> 8) as u8, TERMINAL_BG as u8],
            ansi,
        )
    }

    pub fn set(&self, fg: [u8; 3], bg: [u8; 3], ansi: [[u8; 3]; 16]) {
        *self.inner.lock().unwrap() = ColorInner { fg, bg, ansi };
    }

    pub fn get(&self) -> ([u8; 3], [u8; 3], [[u8; 3]; 16]) {
        let inner = self.inner.lock().unwrap();
        (inner.fg, inner.bg, inner.ansi)
    }

    /// Resolves an OSC color-query index against the live theme. Cube
    /// (16–231) and grayscale (232–255) ramps stay xterm-standard.
    fn resolve(&self, index: usize) -> Option<[u8; 3]> {
        match index {
            0..=15 => Some(self.inner.lock().unwrap().ansi[index]),
            16..=255 => palette_rgb(index),
            256 | 267 => Some(self.inner.lock().unwrap().fg),
            257 | 268 => Some(self.inner.lock().unwrap().bg),
            // Dim variants: halve the base ANSI color.
            259..=266 => {
                let [r, g, b] = self.inner.lock().unwrap().ansi[index - 259];
                Some([r / 2, g / 2, b / 2])
            }
            _ => None,
        }
    }
}

/// A resolved cell color: either one of the terminal's defaults or an
/// explicit palette entry / truecolor value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellColor {
    Foreground,
    Background,
    Rgb([u8; 3]),
    Palette(u8),
}

/// One grid cell as the renderer should paint it.
#[derive(Debug, Clone)]
pub struct TermCell {
    pub ch: char,
    pub zero_width: Vec<char>,
    pub fg: CellColor,
    pub bg: CellColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    /// Continuation column of a double-width glyph; renderers must emit a
    /// blank here to keep the grid aligned.
    pub wide_spacer: bool,
}

/// An immutable view of exactly what the user sees (live screen plus any
/// scrolled-back history), taken under a single lock.
#[derive(Debug, Clone)]
pub struct TerminalSnapshot {
    pub rows: u16,
    pub cols: u16,
    /// Lines of history currently scrolled up from the live screen.
    pub offset: usize,
    /// Total lines of history that exist.
    pub history: usize,
    /// Cursor position relative to the viewport top-left, present only when
    /// the live screen is visible.
    pub cursor: Option<(u16, u16)>,
    pub hide_cursor: bool,
    /// Exactly `rows * cols` cells, row-major from the viewport top-left.
    pub cells: Vec<TermCell>,
}

impl TerminalSnapshot {
    pub fn cell(&self, row: u16, col: u16) -> Option<&TermCell> {
        self.cells.get(row as usize * self.cols as usize + col as usize)
    }
}

fn map_color(color: AnsiColor) -> CellColor {
    match color {
        AnsiColor::Named(
            NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::DimForeground,
        ) => CellColor::Foreground,
        AnsiColor::Named(NamedColor::Background) => CellColor::Background,
        // The cursor color is unused by cell rendering.
        AnsiColor::Named(NamedColor::Cursor) => CellColor::Foreground,
        // Discriminants 0..=15 are the standard + bright palette in order.
        AnsiColor::Named(named) if (named as usize) < 16 => CellColor::Palette(named as u8),
        // We render no separate dim palette; fall back to the base colors
        // (DimBlack starts at discriminant 259; the 8 dim colors follow).
        AnsiColor::Named(
            named @ (NamedColor::DimBlack
            | NamedColor::DimRed
            | NamedColor::DimGreen
            | NamedColor::DimYellow
            | NamedColor::DimBlue
            | NamedColor::DimMagenta
            | NamedColor::DimCyan
            | NamedColor::DimWhite),
        ) => CellColor::Palette((named as usize - NamedColor::DimBlack as usize) as u8),
        AnsiColor::Spec(rgb) => CellColor::Rgb([rgb.r, rgb.g, rgb.b]),
        AnsiColor::Indexed(i) => CellColor::Palette(i),
        // Any other named color has no cell-rendering meaning here.
        AnsiColor::Named(_) => CellColor::Foreground,
    }
}

/// Builds a standalone emulator with our scrollback config, shared by real
/// terminals, dead terminals, and tests.
fn make_term(rows: u16, cols: u16) -> (Arc<FairMutex<Term<EventProxy>>>, Arc<Mutex<Vec<Event>>>) {
    let proxy = EventProxy::default();
    let events = proxy.0.clone();
    let bounds = TermBounds { rows, cols };
    let config = Config {
        scrolling_history: SCROLLBACK_LINES,
        ..Default::default()
    };
    (
        Arc::new(FairMutex::new(Term::new(config, &bounds, proxy))),
        events,
    )
}

pub struct PtyTerminal {
    /// The embedded emulator core (Zed-style: alacritty under our lock,
    /// driven manually from PTY bytes).
    pub term: Arc<FairMutex<Term<EventProxy>>>,
    events: Arc<Mutex<Vec<Event>>>,
    /// Theme-injected palette shared with the renderer and OSC replies.
    pub colors: TerminalColors,
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
        _requested_session: Option<String>,
        rows: u16,
        cols: u16,
        colors: TerminalColors,
        cx: &mut Context<Self>,
    ) -> Self {
        let (term, events) = make_term(rows, cols);
        let term_clone = term.clone();

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
                return dead_terminal(None, colors.clone());
            }
        };

        let shell = detect_shell();
        let zsh_integration = if cfg!(unix) {
            (shell.rsplit('/').next() == Some("zsh"))
                .then(prepare_zsh_integration)
                .flatten()
        } else {
            None
        };
        #[allow(unused_mut)]
        let mut session_name = None;
        let mut cmd = CommandBuilder::new(&shell);
        // Login-shell flag is Unix-only; cmd.exe/powershell don't accept -l.
        #[cfg(unix)]
        cmd.args(["-l"]);
        if let Some(cwd) = cwd.as_deref() {
            cmd.cwd(cwd);
        }

        // Spawn the shell directly on the PTY unless we are reattaching to
        // an existing persisted session. GNU screen interprets the byte
        // stream and swallows OSC color queries (OSC 10/11/4), which CLIs
        // like opencode need answered to pick their dark/light theme — Zed
        // has no multiplexer between shell and emulator, and neither do we.
        #[cfg(target_os = "macos")]
        if let Some((name, screen_target)) = _requested_session
            .filter(|name| valid_session_name(name))
            .and_then(|name| screen_socket(&name).map(|socket| (name, socket)))
        {
            session_name = Some(name);
            cmd = CommandBuilder::new("screen");
            cmd.args(["-A", "-xRR", &screen_target]);
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
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().ends_with("_SHELL_INTEGRATION") {
                cmd.env_remove(key);
            }
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
                return dead_terminal(session_name, colors.clone());
            }
        };

        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(e) => {
                eprintln!("vterm: failed to take reader: {e}");
                return dead_terminal(session_name, colors.clone());
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(e) => {
                eprintln!("vterm: failed to take writer: {e}");
                return dead_terminal(session_name, colors.clone());
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
            let mut processor = Processor::<StdSyncHandler>::new();
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
                        let mut guard = term_clone.lock();
                        for (segment, clear_after) in filter.feed(&batch) {
                            processor.advance(&mut *guard, &segment);
                            if clear_after {
                                // Native CSI 3J semantics: drop saved lines,
                                // keep the visible screen.
                                guard.clear_screen(ClearMode::Saved);
                            }
                        }
                    }
                    // Color requests and PTY writes the emulator raised
                    // while parsing must be answered in order.
                    if this
                        .update(&mut cx, |term_model, _| term_model.drain_events())
                        .is_err()
                    {
                        break;
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
            term,
            events,
            colors,
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
        self.term.lock().scroll_display(Scroll::Bottom);
    }

    pub fn write_text(&mut self, text: &str) {
        let bracketed_paste = self
            .term
            .lock()
            .mode()
            .contains(TermMode::BRACKETED_PASTE);
        self.write(&encode_text_input(text, bracketed_paste));
    }

    /// Answers the events the emulator raised while parsing PTY output:
    /// color queries (OSC 10/11/4) with the exact colors this pane renders
    /// — app-set overrides first, our fixed palette as fallback, Zed-style
    /// — plus any writes the emulator itself wants on the PTY.
    fn drain_events(&mut self) {
        let pending = self.events.lock().unwrap().drain(..).collect::<Vec<_>>();
        for event in pending {
            match event {
                Event::ColorRequest(index, format) => {
                    if let Some(rgb) = self.query_color(index) {
                        // Bypass write() — a machine reply must not reset
                        // scrollback.
                        let _ = self.writer.write_all(format(rgb).as_bytes());
                    }
                }
                Event::PtyWrite(text) => {
                    let _ = self.writer.write_all(text.as_bytes());
                }
                _ => {}
            }
        }
    }

    /// Resolves a palette index an app asked about. App-modified colors (set
    /// via OSC 4 at runtime) win; otherwise the live theme palette answers —
    /// so replies always match what the pane currently renders.
    fn query_color(&self, index: usize) -> Option<alacritty_terminal::vte::ansi::Rgb> {
        use alacritty_terminal::vte::ansi::Rgb;
        let term_colors = self.term.lock().colors()[index];
        term_colors.or_else(|| self.colors.resolve(index).map(|[r, g, b]| Rgb { r, g, b }))
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
        self.term.lock().resize(TermBounds { rows, cols });
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

        self.term.lock().scroll_display(Scroll::Delta(whole_lines));
    }

    pub fn scroll_info(&self) -> (usize, usize) {
        let term = self.term.lock();
        let grid = term.grid();
        (grid.display_offset(), term.history_size())
    }

    pub fn size(&self) -> (u16, u16) {
        let term = self.term.lock();
        let grid = term.grid();
        (grid.screen_lines() as u16, grid.columns() as u16)
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        let term = self.term.lock();
        let content = term.renderable_content();
        if content.cursor.shape == alacritty_terminal::vte::ansi::CursorShape::Hidden {
            return None;
        }
        Some((
            content.cursor.point.line.0.max(0) as u16,
            content.cursor.point.column.0 as u16,
        ))
    }

    /// The text in a viewport-relative cell range (inclusive), one string
    /// per row — used for copy-to-clipboard.
    pub fn text_in_range(
        &self,
        (c1, r1): (u16, u16),
        (c2, r2): (u16, u16),
    ) -> String {
        let snapshot = self.snapshot();
        let min_c = c1.min(c2);
        let max_c = c1.max(c2);
        let mut lines = Vec::new();
        for r in r1.min(r2)..=r1.max(r2) {
            let mut line = String::new();
            for c in min_c..=max_c {
                line.push_str(&
                    snapshot
                        .cell(r, c)
                        .filter(|cell| !cell.wide_spacer)
                        .map(|cell| {
                            let mut text = String::new();
                            text.push(cell.ch);
                            text.extend(cell.zero_width.iter().copied());
                            text
                        })
                        .unwrap_or_else(|| " ".to_string()),
                );
            }
            lines.push(line);
        }
        lines.join("\n")
    }

    /// Captures exactly what the user sees — live screen plus scrolled-back
    /// history — under one lock. The renderer works only off this.
    pub fn snapshot(&self) -> TerminalSnapshot {
        let term = self.term.lock();
        let grid = term.grid();
        let rows = grid.screen_lines() as u16;
        let cols = grid.columns() as u16;
        let offset = grid.display_offset();

        let cursor = term.renderable_content().cursor;
        let hide_cursor = cursor.shape == alacritty_terminal::vte::ansi::CursorShape::Hidden;
        // The cursor belongs to the live screen; while scrolled into history
        // it would highlight an arbitrary historical cell.
        let cursor_pos = (offset == 0 && !hide_cursor).then(|| {
            (
                (cursor.point.line.0.max(0) as u16).min(rows.saturating_sub(1)),
                (cursor.point.column.0 as u16).min(cols.saturating_sub(1)),
            )
        });

        let mut cells = Vec::with_capacity(rows as usize * cols as usize);
        for row in 0..rows as i32 {
            // Negative lines reach into scrollback: viewport top is at
            // Line(-offset), matching what vt100's display offset did.
            let grid_row = &grid[Line(row - offset as i32)];
            for col in 0..cols {
                let cell = &grid_row[Column(col as usize)];
                let wide_spacer = cell.flags.contains(Flags::WIDE_CHAR_SPACER);
                cells.push(TermCell {
                    ch: cell.c,
                    zero_width: cell.zerowidth().unwrap_or_default().to_vec(),
                    fg: map_color(cell.fg),
                    bg: map_color(cell.bg),
                    bold: cell.flags.contains(Flags::BOLD),
                    italic: cell.flags.contains(Flags::ITALIC),
                    underline: cell.flags.contains(Flags::UNDERLINE),
                    inverse: cell.flags.contains(Flags::INVERSE),
                    wide_spacer,
                });
            }
        }

        TerminalSnapshot {
            rows,
            cols,
            offset,
            history: term.history_size(),
            cursor: cursor_pos,
            hide_cursor,
            cells,
        }
    }
}

impl Drop for PtyTerminal {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

/// Returns the user's preferred shell, respecting the platform convention.
fn detect_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string())
    }
    #[cfg(windows)]
    {
        // Prefer PowerShell if available, fall back to cmd.exe.
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
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

fn encode_text_input(text: &str, bracketed_paste: bool) -> Vec<u8> {
    if bracketed_paste && text.chars().any(|character| !character.is_ascii()) {
        let mut bytes = Vec::with_capacity(text.len() + 12);
        bytes.extend_from_slice(b"\x1b[200~");
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(b"\x1b[201~");
        bytes
    } else {
        text.as_bytes().to_vec()
    }
}

fn dead_terminal(session_name: Option<String>, colors: TerminalColors) -> PtyTerminal {
    let (term, events) = make_term(24, 80);
    PtyTerminal {
        term,
        events,
        colors,
        writer: Box::new(std::io::sink()),
        master: None,
        child: None,
        child_pid: None,
        session_name,
        scroll_accumulator: 0.0,
    }
}

#[cfg(target_os = "macos")]
fn valid_session_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
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

/// Resolves an OSC color-query index to the exact RGB we render, using the
/// same pseudo-indices alacritty's `Colors` table uses (256 = foreground,
/// 257 = background, 259+ = dim variants). Returns `None` for indices with
/// no meaningful answer (e.g. the cursor color).
pub fn fallback_color_rgb(index: usize) -> Option<[u8; 3]> {
    match index {
        0..=255 => palette_rgb(index),
        256 | 267 => Some([(TERMINAL_FG >> 16) as u8, (TERMINAL_FG >> 8) as u8, TERMINAL_FG as u8]),
        257 | 268 => Some([(TERMINAL_BG >> 16) as u8, (TERMINAL_BG >> 8) as u8, TERMINAL_BG as u8]),
        // Dim palette entries: halve the standard colors.
        259..=266 => {
            let [r, g, b] = palette_rgb(index - 259)?;
            Some([r / 2, g / 2, b / 2])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::process::Command;

    /// Feeds raw bytes through the same parse path as the read loop.
    fn feed(term: &Arc<FairMutex<Term<EventProxy>>>, bytes: &[u8]) {
        let mut processor = Processor::<StdSyncHandler>::new();
        let mut guard = term.lock();
        processor.advance(&mut *guard, bytes);
    }

    impl PtyTerminal {
        /// Wraps a bare emulator (no PTY) so snapshot logic is testable.
        fn for_testing(
            term: Arc<FairMutex<Term<EventProxy>>>,
            events: Arc<Mutex<Vec<Event>>>,
        ) -> Self {
            Self {
                term,
                events,
                colors: TerminalColors::dark(),
                writer: Box::new(std::io::sink()),
                master: None,
                child: None,
                child_pid: None,
                session_name: None,
                scroll_accumulator: 0.0,
            }
        }
    }

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
    fn clear_saved_history_keeps_screen_and_drops_history() {
        let (term, _events) = make_term(4, 20);
        feed(&term, &[b'\n'; 40]);
        feed(&term, b"visible");
        assert!(term.lock().history_size() > 0, "history exists pre-clear");

        term.lock().clear_screen(ClearMode::Saved);
        let term = term.lock();
        assert_eq!(term.history_size(), 0, "history purged");

        // The text sits where the cursor was left; verify it survived.
        let point = term.grid().cursor.point;
        let row = &term.grid()[point.line];
        let start = point.column.0.saturating_sub(7);
        let text: String = (start..point.column.0)
            .map(|c| row[Column(c)].c)
            .collect();
        assert_eq!(text, "visible", "live contents survive the clear");
    }

    #[test]
    fn snapshot_renders_text_attributes_and_cursor() {
        let (term, events) = make_term(4, 20);
        feed(&term, b"\x1b[1;31mre\x1b[0md");

        let snap = PtyTerminal::for_testing(term, events).snapshot();
        assert_eq!((snap.rows, snap.cols), (4, 20));
        assert_eq!(snap.offset, 0);
        assert_eq!(
            snap.cell(0, 0).map(|c| (c.ch, c.fg, c.bold)),
            Some(('r', CellColor::Palette(1), true))
        );
        assert_eq!(
            snap.cell(0, 1).map(|c| (c.ch, c.fg)),
            Some(('e', CellColor::Palette(1)))
        );
        // Reset restores default colors.
        assert_eq!(
            snap.cell(0, 2).map(|c| (c.ch, c.fg)),
            Some(('d', CellColor::Foreground))
        );
        // Cursor sits right after 'd'.
        assert_eq!(snap.cursor, Some((0, 3)));
        // Empty cells are blank spaces.
        assert_eq!(snap.cell(3, 19).map(|c| c.ch), Some(' '));
    }

    #[test]
    fn snapshot_passes_through_truecolor() {
        // codex colors its input line with 24-bit SGR; the emulator must keep
        // the exact RGB rather than collapsing to a palette entry.
        let (term, events) = make_term(1, 20);
        feed(&term, b"\x1b[38;2;205;49;49mX\x1b[48;2;10;20;30mY\x1b[0m");

        let snap = PtyTerminal::for_testing(term, events).snapshot();
        assert_eq!(
            snap.cell(0, 0).map(|c| (c.ch, c.fg)),
            Some(('X', CellColor::Rgb([205, 49, 49])))
        );
        assert_eq!(
            snap.cell(0, 1).map(|c| (c.ch, c.bg)),
            Some(('Y', CellColor::Rgb([10, 20, 30])))
        );
        // Reset restores defaults, not the last truecolor value.
        assert_eq!(
            snap.cell(0, 2).map(|c| (c.ch, c.fg, c.bg)),
            Some((' ', CellColor::Foreground, CellColor::Background))
        );
    }

    #[test]
    fn snapshot_views_scrollback_offset() {
        let (term, events) = make_term(2, 10);
        feed(&term, b"one\r\ntwo\r\nthree\r\nfour");

        let live = PtyTerminal::for_testing(term.clone(), events.clone()).snapshot();
        assert_eq!(live.history, 2);
        assert_eq!(live.offset, 0);
        assert_eq!(live.cell(0, 0).map(|c| c.ch), Some('t')); // "two"

        // User scrolling moves the viewport into history (content-preserving).
        // Screen shows [three, four], history [one, two]; scrolling 2 reveals
        // the full history.
        term.lock().scroll_display(Scroll::Delta(2));
        let scrolled = PtyTerminal::for_testing(term, events).snapshot();
        assert_eq!(scrolled.offset, 2);
        assert_eq!(scrolled.cell(0, 0).map(|c| c.ch), Some('o')); // "one"
        assert_eq!(scrolled.cell(1, 0).map(|c| c.ch), Some('t')); // "two"
        // Cursor hidden while viewing history.
        assert_eq!(scrolled.cursor, None);
    }

    #[test]
    fn color_requests_are_raised_as_events() {
        let (term, events) = make_term(4, 20);
        feed(&term, b"out \x1b]11;?\x1b\\\x1b]4;1;?\x07");
        let pending = events.lock().unwrap();
        assert_eq!(pending.len(), 2, "bg + palette queries answered");
        assert!(matches!(&pending[0], Event::ColorRequest(257, _)));
        assert!(matches!(&pending[1], Event::ColorRequest(1, _)));
    }

    #[test]
    fn theme_colors_resolve_query_indices() {
        let mut ansi = STANDARD_ANSI.map(|hex| [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]);
        // Inject a light-ish theme palette entry to prove dynamism.
        ansi[1] = [0xaa, 0x12, 0x34];
        let colors = TerminalColors::new([0x11, 0x22, 0x33], [0xdd, 0xee, 0xff], ansi);

        assert_eq!(colors.resolve(0), Some([0x00, 0x00, 0x00]));
        assert_eq!(colors.resolve(1), Some([0xaa, 0x12, 0x34]));
        assert_eq!(colors.resolve(255), Some([238, 238, 238]));
        // 256 = foreground, 257 = background pseudo-indices.
        assert_eq!(colors.resolve(256), Some([0x11, 0x22, 0x33]));
        assert_eq!(colors.resolve(257), Some([0xdd, 0xee, 0xff]));
        // Dim entries halve the live ANSI palette; cursor has no answer.
        assert_eq!(colors.resolve(259), Some([0x00, 0x00, 0x00]));
        assert_eq!(colors.resolve(260), Some([0x55, 0x09, 0x1a]));
        assert_eq!(colors.resolve(258), None);
        assert_eq!(colors.resolve(269), None);

        // Live updates apply immediately.
        colors.set(
            [0, 0, 0],
            [255, 255, 255],
            {
                let mut a = ansi;
                a[2] = [9, 9, 9];
                a
            },
        );
        assert_eq!(colors.resolve(256), Some([0, 0, 0]));
        assert_eq!(colors.resolve(257), Some([255, 255, 255]));
        assert_eq!(colors.resolve(2), Some([9, 9, 9]));
    }

    #[test]
    fn palette_rgb_matches_xterm_standards() {
        assert_eq!(palette_rgb(0), Some([0x00, 0x00, 0x00]));
        assert_eq!(palette_rgb(15), Some([0xff, 0xff, 0xff]));
        assert_eq!(palette_rgb(16), Some([0x00, 0x00, 0x00]));
        assert_eq!(palette_rgb(231), Some([0xff, 0xff, 0xff]));
        assert_eq!(palette_rgb(232), Some([8, 8, 8]));
        assert_eq!(palette_rgb(255), Some([238, 238, 238]));
        assert_eq!(palette_rgb(256), None);
    }

    #[test]
    fn encodes_composed_text_as_bracketed_paste() {
        assert_eq!(
            encode_text_input("สวัสดี", true),
            "\x1b[200~สวัสดี\x1b[201~".as_bytes()
        );
        assert_eq!(encode_text_input("สวัสดี", false), "สวัสดี".as_bytes());
        assert_eq!(encode_text_input("plain", true), b"plain");
    }

    #[cfg(unix)]
    #[test]
    fn zsh_codepoint_delete_removes_one_combining_codepoint() {
        let script = format!(
            r#"{ZSH_CODEPOINT_BACKSPACE}
LBUFFER="สวัสดี"
CURSOR=${{#LBUFFER}}
vterm_backward_delete_codepoint
print -r -- "$LBUFFER"
"#
        );
        let output = Command::new("zsh")
            .args(["-dfc", &script])
            .output()
            .expect("zsh is required for terminal input tests");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<_> = stdout.lines().collect();
        assert_eq!(lines.last().copied(), Some("สวัสด"));
    }

    #[test]
    fn snapshot_preserves_zero_width_text() {
        let (term, events) = make_term(2, 20);
        feed(&term, "สวัสดี".as_bytes());
        let snapshot = PtyTerminal::for_testing(term, events).snapshot();
        let row: String = (0..snapshot.cols)
            .filter_map(|col| snapshot.cell(0, col))
            .filter(|cell| !cell.wide_spacer)
            .flat_map(|cell| std::iter::once(cell.ch).chain(cell.zero_width.iter().copied()))
            .collect();
        assert_eq!(row.trim_end(), "สวัสดี");
        assert_eq!(snapshot.cursor, Some((0, 4)));
    }
}
