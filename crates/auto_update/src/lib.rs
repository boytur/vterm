use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const CURRENT_VERSION: &str = env!("VTERM_VERSION");
pub const REPO: &str = "boytur/vterm";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub can_auto_install: bool,
    #[allow(dead_code)]
    pub release_notes: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    body: Option<String>,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Queries the GitHub "latest release" endpoint and returns update info when a
/// newer version than the running build is available.
pub fn check_for_update_detailed() -> Result<Option<UpdateInfo>, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("vterm")
        .build();

    let resp = agent.get(&url).call().map_err(|e| e.to_string())?;
    let release: Release = resp.into_json().map_err(|e| e.to_string())?;

    let latest = release.tag_name.trim_start_matches('v');
    if !is_newer(latest, CURRENT_VERSION.trim_start_matches('v')) {
        return Ok(None);
    }

    let download = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".dmg"))
        .map(|a| a.browser_download_url.clone());
    let download_url = download
        .clone()
        .unwrap_or_else(|| format!("https://github.com/{}/releases/latest", REPO));

    Ok(Some(UpdateInfo {
        version: latest.to_string(),
        download_url,
        can_auto_install: download.is_some(),
        release_notes: release.body.unwrap_or_default(),
    }))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(latest),
        semver::Version::parse(current),
    ) {
        (Ok(l), Ok(c)) => l > c,
        _ => latest != current,
    }
}

#[cfg(target_os = "macos")]
pub fn download_update(
    info: &UpdateInfo,
    progress: async_channel::Sender<f32>,
) -> Result<PathBuf, String> {
    if !info.can_auto_install {
        return Err("no macOS disk image is available for this release".into());
    }

    let dmg = download_to_temp(&info.download_url, &progress)?;
    let mount = match attach_dmg(&dmg) {
        Ok(mount) => mount,
        Err(error) => {
            let _ = std::fs::remove_file(&dmg);
            return Err(error);
        }
    };
    let result = find_in_volume(&mount).and_then(|new_app| stage_app(&new_app, &info.version));
    detach_dmg(&mount);
    let _ = std::fs::remove_file(&dmg);
    result
}

#[cfg(not(target_os = "macos"))]
pub fn download_update(
    _info: &UpdateInfo,
    _progress: async_channel::Sender<f32>,
) -> Result<PathBuf, String> {
    Err("automatic updates are currently supported on macOS only".into())
}

#[cfg(target_os = "macos")]
pub fn install_update(staged_app: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let current_app = find_app_bundle(&exe).ok_or("not running from a .app bundle")?;

    install_app(staged_app, &current_app)?;
    let _ = std::fs::remove_dir_all(staged_app);
    relaunch(&current_app)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn install_update(_staged_app: &Path) -> Result<(), String> {
    Err("automatic updates are currently supported on macOS only".into())
}

#[cfg(target_os = "macos")]
fn find_app_bundle(exe: &Path) -> Option<PathBuf> {
    let mut p = exe.to_path_buf();
    loop {
        if p.extension().and_then(|e| e.to_str()) == Some("app") {
            return Some(p);
        }
        p = p.parent()?.to_path_buf();
    }
}

#[cfg(target_os = "macos")]
fn download_to_temp(url: &str, progress: &async_channel::Sender<f32>) -> Result<PathBuf, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("vterm")
        .build();
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let total = resp
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());
    let mut reader = resp.into_reader();

    let tmp = std::env::temp_dir().join(format!("vterm-update-{}.dmg", std::process::id()));
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut downloaded = 0_u64;
    let _ = progress.send_blocking(0.0);
    loop {
        let read = std::io::Read::read(&mut reader, &mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buffer[..read]).map_err(|e| e.to_string())?;
        downloaded += read as u64;
        if let Some(total) = total {
            let _ = progress.send_blocking((downloaded as f32 / total as f32).min(1.0));
        }
    }
    let _ = progress.send_blocking(1.0);
    Ok(tmp)
}

#[cfg(target_os = "macos")]
fn attach_dmg(dmg: &Path) -> Result<PathBuf, String> {
    let out = std::process::Command::new("hdiutil")
        .arg("attach")
        .arg("-nobrowse")
        .arg("-noautoopen")
        .arg(dmg.as_os_str())
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if let Some(volume) = line
            .split('\t')
            .map(str::trim)
            .find(|part| part.starts_with("/Volumes/"))
        {
            return Ok(PathBuf::from(volume));
        }
    }
    Err("could not locate mounted volume".into())
}

#[cfg(target_os = "macos")]
fn detach_dmg(mount: &Path) {
    let _ = std::process::Command::new("hdiutil")
        .arg("detach")
        .arg(mount.as_os_str())
        .arg("-force")
        .output();
}

#[cfg(target_os = "macos")]
fn find_in_volume(mount: &Path) -> Result<PathBuf, String> {
    for entry in std::fs::read_dir(mount).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("app") {
            return Ok(path);
        }
    }
    Err("no .app found in update disk image".into())
}

#[cfg(target_os = "macos")]
fn stage_app(new_app: &Path, version: &str) -> Result<PathBuf, String> {
    if !new_app.join("Contents/MacOS/vterm").is_file() {
        return Err("update bundle is missing its executable".into());
    }
    let staged =
        std::env::temp_dir().join(format!("vterm-update-{}-{version}.app", std::process::id()));
    let _ = std::fs::remove_dir_all(&staged);

    let result = std::process::Command::new("cp")
        .arg("-Rf")
        .arg(new_app.as_os_str())
        .arg(&staged)
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).trim().to_string());
    }
    Ok(staged)
}

#[cfg(target_os = "macos")]
fn install_app(new_app: &Path, current_app: &Path) -> Result<(), String> {
    let dest_parent = current_app.parent().ok_or("bad app path")?;
    let name = current_app
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("bad app name")?;
    let dest = dest_parent.join(name);
    let old = dest_parent.join(format!("{name}.old"));

    // Move the running bundle aside (rename doesn't disturb the live process),
    // then copy the new one in. Overwriting the running executable directly
    // fails with "text file busy", so the move-aside avoids that.
    let _ = std::fs::remove_dir_all(&old);
    let mv = std::process::Command::new("mv")
        .args([dest.as_os_str(), old.as_os_str()])
        .output()
        .map_err(|e| e.to_string())?;
    if !mv.status.success() {
        return Err(String::from_utf8_lossy(&mv.stderr).trim().to_string());
    }

    let cp = std::process::Command::new("cp")
        .arg("-Rf")
        .arg(new_app.as_os_str())
        .arg(dest.as_os_str())
        .output()
        .map_err(|e| e.to_string())?;
    if !cp.status.success() {
        // Restore the previous version so the app still launches.
        let _ = std::process::Command::new("mv")
            .args([old.as_os_str(), dest.as_os_str()])
            .output();
        return Err(String::from_utf8_lossy(&cp.stderr).trim().to_string());
    }

    // Clear any quarantine so Gatekeeper doesn't block the freshly copied app.
    let _ = std::process::Command::new("xattr")
        .arg("-dr")
        .arg("com.apple.quarantine")
        .arg(dest.as_os_str())
        .output();

    // Best-effort cleanup of the moved-aside old bundle.
    let _ = std::process::Command::new("rm")
        .arg("-rf")
        .arg(old.as_os_str())
        .output();

    Ok(())
}

#[cfg(target_os = "macos")]
fn relaunch(app: &Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("-n")
        .arg(app)
        .spawn()
        .map_err(|e| e.to_string())?;
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(!is_newer("beta", "beta"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_install_app_keeps_original_bundle_path() {
        let root = std::env::temp_dir().join(format!("vterm-install-test-{}", std::process::id()));
        let current = root.join("vterm.app");
        let staged = root.join("vterm-update-0.2.3.app");
        std::fs::create_dir_all(current.join("Contents/MacOS")).unwrap();
        std::fs::create_dir_all(staged.join("Contents/MacOS")).unwrap();
        std::fs::write(current.join("Contents/MacOS/vterm"), b"old").unwrap();
        std::fs::write(staged.join("Contents/MacOS/vterm"), b"new").unwrap();

        install_app(&staged, &current).unwrap();

        assert_eq!(
            std::fs::read(current.join("Contents/MacOS/vterm")).unwrap(),
            b"new"
        );
        assert!(!root.join("vterm.app.old").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
