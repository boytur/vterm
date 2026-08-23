use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use gpui::Context;

pub struct PtyTerminal {
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub child_pid: Option<u32>,
}

impl PtyTerminal {

    pub fn new_with_cwd(cwd: Option<String>, cx: &mut Context<Self>) -> Self {
        let pty_system = NativePtySystem::default();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }).unwrap();

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string());
        let mut cmd = CommandBuilder::new(shell);
        cmd.args(["-l"]);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(c) = cwd {
            cmd.cwd(c);
        }
        
        let child = pair.slave.spawn_command(cmd).unwrap();
        let child_pid = child.process_id();
        
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let master = pair.master;

        let parser = Arc::new(Mutex::new(vt100::Parser::new(24, 80, 0)));
        let parser_clone = parser.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        
        std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 { break; }
                let mut v = Vec::with_capacity(n);
                v.extend_from_slice(&buf[..n]);
                if tx.send(v).is_err() {
                    break;
                }
            }
        });

        cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    let mut received = false;
                    while let Ok(bytes) = rx.try_recv() {
                        parser_clone.lock().unwrap().process(&bytes);
                        received = true;
                    }
                    
                    if received {
                        if this.update(&mut cx, |_, cx| cx.notify()).is_err() {
                            break;
                        }
                    }
                    
                    cx.background_executor().timer(std::time::Duration::from_millis(16)).await;
                }
            }
        }).detach();

        Self {
            parser,
            writer,
            master,
            child_pid,
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        self.writer.write_all(data).ok();
    }
    
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }).ok();
        self.parser.lock().unwrap().screen_mut().set_size(rows, cols);
    }
}
