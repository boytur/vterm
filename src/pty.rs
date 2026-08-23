use std::io::Write;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_channel::{Receiver, Sender};
use gpui::{Context, WeakEntity};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use vt100::Parser;

pub struct PtyTerminal {
    pub parser: Arc<Mutex<Parser>>,
    writer: Box<dyn Write + Send>,
    master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn Child + Send>>,
    pub child_pid: Option<u32>,
    pub session_name: Option<String>,
}

impl PtyTerminal {
    pub fn new_with_cwd(
        cwd: Option<String>,
        requested_session: Option<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let parser = Arc::new(Mutex::new(Parser::new(24, 80, 10000)));
        let parser_clone = parser.clone();

        let pty_system = native_pty_system();
        let pair = match pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
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
            ensure_screen_session(&name, cwd.as_deref(), &shell);
            session_name = Some(name.clone());
            cmd = CommandBuilder::new("screen");
            cmd.args(["-xRR", &name]);
            if let Some(cwd) = cwd.as_deref() {
                cmd.cwd(cwd);
            }
        }

        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

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
            async move {
                while let Ok(bytes) = rx.recv().await {
                    parser_clone.lock().unwrap().process(&bytes);
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

    pub fn scroll(&mut self, delta_lines: f32) {
        let mut parser = self.parser.lock().unwrap();
        let current_offset = parser.screen().scrollback();

        let new_offset = if delta_lines < 0.0 {
            current_offset.saturating_add((-delta_lines) as usize)
        } else {
            current_offset.saturating_sub(delta_lines as usize)
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

fn dead_terminal(parser: Arc<Mutex<Parser>>, session_name: Option<String>) -> PtyTerminal {
    PtyTerminal {
        parser,
        writer: Box::new(std::io::sink()),
        master: None,
        child: None,
        child_pid: None,
        session_name,
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
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "vterm-{}-{}-{}",
        std::process::id(),
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed),
        nonce
    )
}

#[cfg(target_os = "macos")]
fn ensure_screen_session(name: &str, cwd: Option<&str>, shell: &str) {
    let mut command = std::process::Command::new("screen");
    command.args(["-dmS", name, shell, "-l"]);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor");
    let _ = command.status();
}
