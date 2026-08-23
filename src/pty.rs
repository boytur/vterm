use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::ptr;
use std::sync::{Arc, Mutex};

use gpui::Context;

pub struct MasterPty {
    fd: RawFd,
}

impl MasterPty {
    pub fn resize(&self, rows: u16, cols: u16) -> std::io::Result<()> {
        unsafe { set_winsize(self.fd, rows, cols) }
    }
}

impl Drop for MasterPty {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

pub struct PtyTerminal {
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub writer: Box<dyn Write + Send>,
    pub master: MasterPty,
    pub child_pid: Option<u32>,
}

unsafe fn set_winsize(fd: RawFd, rows: u16, cols: u16) -> std::io::Result<()> {
    let ws = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if libc::ioctl(fd, libc::TIOCSWINSZ, &ws as *const libc::winsize) < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

unsafe fn open_pty(rows: u16, cols: u16) -> std::io::Result<(RawFd, RawFd)> {
    let master = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
    if master < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::grantpt(master) } < 0 {
        unsafe {
            libc::close(master);
        }
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::unlockpt(master) } < 0 {
        unsafe {
            libc::close(master);
        }
        return Err(std::io::Error::last_os_error());
    }
    let name_ptr = unsafe { libc::ptsname(master) };
    if name_ptr.is_null() {
        unsafe {
            libc::close(master);
        }
        return Err(std::io::Error::last_os_error());
    }
    let slave_path = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
    let slave_path_c =
        CString::new(slave_path).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "slave name"))?;
    let slave = unsafe { libc::open(slave_path_c.as_ptr(), libc::O_RDWR | libc::O_NOCTTY) };
    if slave < 0 {
        unsafe {
            libc::close(master);
        }
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        set_winsize(master, rows, cols)?;
    }
    Ok((master, slave))
}

unsafe fn spawn_shell(
    master_fd: RawFd,
    slave_fd: RawFd,
    shell: &str,
    cwd: Option<&str>,
) -> std::io::Result<libc::pid_t> {
    let shell_c =
        CString::new(shell).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "shell"))?;
    let login_arg = CString::new("-l").unwrap();
    let cwd_c = cwd
        .map(|c| CString::new(c).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "cwd")))
        .transpose()?;

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if pid == 0 {
        unsafe {
            libc::close(master_fd);
            libc::setsid();
            libc::ioctl(slave_fd, libc::TIOCSCTTY as libc::c_ulong, 0);
            libc::dup2(slave_fd, 0);
            libc::dup2(slave_fd, 1);
            libc::dup2(slave_fd, 2);
            if slave_fd > 2 {
                libc::close(slave_fd);
            }
            if let Some(cwd_c) = &cwd_c {
                libc::chdir(cwd_c.as_ptr());
            }
            let argv: [*const libc::c_char; 3] = [shell_c.as_ptr(), login_arg.as_ptr(), ptr::null()];
            libc::execvp(shell_c.as_ptr(), argv.as_ptr());
            libc::_exit(127);
        }
    }
    Ok(pid)
}

impl PtyTerminal {
    pub fn new_with_cwd(cwd: Option<String>, cx: &mut Context<Self>) -> Self {
        let parser = Arc::new(Mutex::new(vt100::Parser::new(24, 80, 0)));
        let parser_clone = parser.clone();

        let mut master_fd: RawFd = -1;
        let mut child_pid: Option<u32> = None;

        match unsafe { open_pty(24, 80) } {
            Ok((m, s)) => {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string());
                unsafe {
                    std::env::set_var("TERM", "xterm-256color");
                    std::env::set_var("COLORTERM", "truecolor");
                }
                match unsafe { spawn_shell(m, s, &shell, cwd.as_deref()) } {
                    Ok(pid) => {
                        unsafe {
                            libc::close(s);
                        }
                        master_fd = m;
                        child_pid = Some(pid as u32);
                    }
                    Err(e) => {
                        eprintln!("vterm: failed to spawn shell: {e}");
                        unsafe {
                            libc::close(m);
                            libc::close(s);
                        }
                    }
                }
            }
            Err(e) => eprintln!("vterm: failed to open pty: {e}"),
        }

        let master = MasterPty { fd: master_fd };
        let writer: Box<dyn Write + Send> = if master_fd >= 0 {
            Box::new(unsafe { std::fs::File::from_raw_fd(libc::dup(master_fd)) })
        } else {
            Box::new(std::io::sink())
        };

        if master_fd >= 0 {
            let reader = unsafe { std::fs::File::from_raw_fd(libc::dup(master_fd)) };
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let mut reader = reader;
                let mut buf = [0u8; 8192];
                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            });

            cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    while let Ok(bytes) = rx.recv() {
                        parser_clone.lock().unwrap().process(&bytes);
                        if this.update(&mut cx, |_, cx| cx.notify()).is_err() {
                            break;
                        }
                    }
                }
            })
            .detach();
        }

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
        self.master.resize(rows, cols).ok();
        self.parser.lock().unwrap().screen_mut().set_size(rows, cols);
    }
}

impl Drop for PtyTerminal {
    fn drop(&mut self) {
        if let Some(pid) = self.child_pid {
            let p = pid as libc::pid_t;
            unsafe {
                libc::kill(-p, libc::SIGHUP);
                let mut status = 0;
                libc::waitpid(p, &mut status, 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_open_and_spawn_pty() {
        let (m, s) = unsafe { open_pty(24, 80) }.expect("open_pty");
        let pid = unsafe { spawn_shell(m, s, "/bin/sh", None) }.expect("spawn_shell");
        unsafe {
            libc::close(s);
        }

        let mut master = unsafe { std::fs::File::from_raw_fd(m) };
        std::thread::sleep(std::time::Duration::from_millis(300));
        master.write_all(b"echo pty_ok\r").ok();
        master.flush().ok();

        let mut buf = [0u8; 1024];
        let mut got = String::new();
        for _ in 0..40 {
            match master.read(&mut buf) {
                Ok(n) if n > 0 => {
                    got.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if got.contains("pty_ok") {
                        break;
                    }
                }
                _ => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }

        unsafe {
            libc::kill(-pid, libc::SIGHUP);
        }
        assert!(got.contains("pty_ok"), "pty output was: {:?}", got);
    }

    #[test]
    fn test_reader_channel_pipeline() {
        use std::sync::mpsc::channel;
        use std::sync::{Arc, Mutex};

        let (m, s) = unsafe { open_pty(24, 80) }.expect("open_pty");
        let pid = unsafe { spawn_shell(m, s, "/bin/sh", None) }.expect("spawn_shell");
        unsafe {
            libc::close(s);
        }

        let reader = unsafe { std::fs::File::from_raw_fd(libc::dup(m)) };
        let mut writer = unsafe { std::fs::File::from_raw_fd(libc::dup(m)) };
        unsafe {
            libc::close(m);
        }

        let parser = Arc::new(Mutex::new(vt100::Parser::new(24, 80, 0)));
        let parser_clone = parser.clone();
        let (tx, rx) = channel();

        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(300));
        writer.write_all(b"echo pipe_ok\r").ok();
        writer.flush().ok();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut found = false;
        while std::time::Instant::now() < deadline {
            while let Ok(bytes) = rx.try_recv() {
                parser_clone.lock().unwrap().process(&bytes);
                let screen = parser_clone.lock().unwrap().screen().contents();
                if screen.contains("pipe_ok") {
                    found = true;
                }
            }
            if found {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        unsafe {
            libc::kill(-pid, libc::SIGHUP);
        }
        assert!(found, "parser screen never contained pipe_ok");
    }
}
