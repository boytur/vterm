use std::io::Write;
use std::sync::{Arc, Mutex};

use async_channel::{Receiver, Sender};
use gpui::{Context, WeakEntity};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use vt100::Parser;

pub struct PtyTerminal {
    pub parser: Arc<Mutex<Parser>>,
    writer: Box<dyn Write + Send>,
    master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn Child + Send>>,
    pub child_pid: Option<u32>,
}

impl PtyTerminal {
    pub fn new_with_cwd(cwd: Option<String>, cx: &mut Context<Self>) -> Self {
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
                return dead_terminal(parser);
            }
        };

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string());
        let mut cmd = CommandBuilder::new(shell);
        cmd.args(["-l"]);
        if let Some(cwd) = cwd.as_deref() {
            cmd.cwd(cwd);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let child = match pair.slave.spawn_command(cmd) {
            Ok(child) => child,
            Err(e) => {
                eprintln!("vterm: failed to spawn shell: {e}");
                return dead_terminal(parser);
            }
        };

        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(e) => {
                eprintln!("vterm: failed to take reader: {e}");
                return dead_terminal(parser);
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(e) => {
                eprintln!("vterm: failed to take writer: {e}");
                return dead_terminal(parser);
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

fn dead_terminal(parser: Arc<Mutex<Parser>>) -> PtyTerminal {
    PtyTerminal {
        parser,
        writer: Box::new(std::io::sink()),
        master: None,
        child: None,
        child_pid: None,
    }
}
