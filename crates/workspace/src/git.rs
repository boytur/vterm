//! Git + process helpers extracted from the `Workspace` god object.
//!
//! Keeps PTY pid -> cwd resolution and `git` invocation in one place so
//! `workspace.rs` stays focused on UI state.

/// Resolves the cwd for `pid` by walking to the deepest child (the real shell)
/// then reading its cwd. Unix-only; Windows returns `None` and callers fall
/// back to the saved `TerminalData::cwd`.
pub(crate) fn process_cwd(#[allow(unused_variables)] pid: u32) -> Option<String> {
    #[cfg(unix)]
    {
        let child_pids = std::process::Command::new("pgrep")
            .args(["-P", &pid.to_string()])
            .output()
            .ok()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|line| line.trim().parse::<u32>().ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for child_pid in child_pids {
            if let Some(cwd) = process_cwd(child_pid) {
                return Some(cwd);
            }
        }

        let output = std::process::Command::new("lsof")
            .args(["-p", &pid.to_string(), "-a", "-d", "cwd", "-F", "n"])
            .output()
            .ok()?;

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| line.strip_prefix('n').map(str::to_string))
    }

    #[cfg(windows)]
    {
        // Dynamic CWD tracking isn't implemented on Windows (it would need
        // NtQueryInformationProcess + reading the target PEB). Callers fall
        // back to the saved `TerminalData::cwd`, so git branch + session
        // restore still work from the last known directory.
        None
    }
}

pub(crate) fn git_command(cwd: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(cwd);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x08000000); // CREATE_NO_WINDOW
    cmd
}
