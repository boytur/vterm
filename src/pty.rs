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
        }

        let _ = unsafe {
            libc::setenv(
                CString::new("TERM").unwrap().as_ptr(),
                CString::new("xterm-256color").unwrap().as_ptr(),
                1,
            )
        };
        let _ = unsafe {
            libc::setenv(
                CString::new("COLORTERM").unwrap().as_ptr(),
                CString::new("truecolor").unwrap().as_ptr(),
                1,
            )
        };

        if let Some(cwd) = cwd {
            if let Ok(cwd_c) = CString::new(cwd) {
                unsafe {
                    libc::chdir(cwd_c.as_ptr());
                }
            }
        }

        let argv: [*const libc::c_char; 3] = [shell_c.as_ptr(), login_arg.as_ptr(), ptr::null()];
        unsafe {
            libc::execvp(shell_c.as_ptr(), argv.as_ptr());
            libc::_exit(127);
        }
    }
    Ok(pid)
}

impl PtyTerminal {
    pub fn new_with_cwd(cwd: Option<String>, cx: &mut Context<Self>) -> Self {
        let (master_fd, slave_fd) = unsafe { open_pty(24, 80).expect("failed to open pty") };
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string());
        let child_pid = unsafe {
            spawn_shell(master_fd, slave_fd, &shell, cwd.as_deref()).expect("failed to spawn shell")
        };
        unsafe {
            libc::close(slave_fd);
        }

        let master = MasterPty { fd: master_fd };
        let reader = unsafe { std::fs::File::from_raw_fd(libc::dup(master_fd)) };
        let writer = unsafe { std::fs::File::from_raw_fd(libc::dup(master_fd)) };

        let parser = Arc::new(Mutex::new(vt100::Parser::new(24, 80, 0)));
        let parser_clone = parser.clone();

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

        Self {
            parser,
            writer: Box::new(writer),
            master,
            child_pid: Some(child_pid as u32),
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
