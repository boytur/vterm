use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const CURRENT_VERSION: &str = env!("VTERM_VERSION");
pub const REPO: &str = "boytur/vterm";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
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
/// newer version than the running build is available. Returns `None` when the
/// app is up to date, the network/API is unreachable, or parsing fails.
pub fn check_for_update() -> Option<UpdateInfo> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let agent = ureq::AgentBuilder::new().user_agent("vterm").build();

    let resp = agent.get(&url).call().ok()?;
    let release: Release = resp.into_json().ok()?;

    let latest = release.tag_name.trim_start_matches('v');
    if !is_newer(latest, CURRENT_VERSION) {
        return None;
    }

    let download_url = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".dmg"))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".zip")))
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_else(|| format!("https://github.com/{}/releases/latest", REPO));

    Some(UpdateInfo {
        version: latest.to_string(),
        download_url,
        release_notes: release.body.unwrap_or_default(),
    })
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (semver::Version::parse(latest), semver::Version::parse(current)) {
        (Ok(l), Ok(c)) => l > c,
        _ => latest != current,
    }
}

/// macOS: download the new release, mount it, copy the app over the running
/// bundle, then relaunch. Falls back to opening the download URL in a browser
/// if any step fails (e.g. a dev build that isn't a real .app bundle).
#[cfg(target_os = "macos")]
pub fn notify(info: &UpdateInfo) {
    match download_and_install(info) {
        Ok(()) => {
            // download_and_install relaunches and exits; we only get here on error.
        }
        Err(e) => {
            eprintln!("vterm: auto-update failed ({e}); opening download page");
            let _ = std::process::Command::new("open")
                .arg(&info.download_url)
                .output();
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn notify(_info: &UpdateInfo) {}

#[cfg(target_os = "macos")]
fn download_and_install(info: &UpdateInfo) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let current_app = find_app_bundle(&exe).ok_or("not running from a .app bundle")?;

    show_notification(&format!("Updating to v{}…", info.version));

    let dmg = download_to_temp(&info.download_url)?;
    let mount = attach_dmg(&dmg)?;
    let new_app = find_in_volume(&mount)?;
    install_app(&new_app, &current_app)?;
    detach_dmg(&mount);

    relaunch(&current_app)?;
    Ok(())
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
fn download_to_temp(url: &str) -> Result<PathBuf, String> {
    let agent = ureq::AgentBuilder::new().user_agent("vterm").build();
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader();

    let tmp = std::env::temp_dir().join(format!("vterm-update-{}.dmg", std::process::id()));
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|e| e.to_string())?;
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
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1].starts_with("/Volumes/") {
            return Ok(PathBuf::from(parts[1]));
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
        .arg(dest_parent.as_os_str())
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

#[cfg(target_os = "macos")]
fn show_notification(message: &str) {
    let script = format!(
        "display notification \"{}\" with title \"vterm\" subtitle \"Updating\"",
        message.replace('"', "")
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
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
}
