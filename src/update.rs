use serde::Deserialize;

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

#[cfg(target_os = "macos")]
pub fn notify(info: &UpdateInfo) {
    let script = format!(
        "display notification \"Version {} is available\" with title \"vterm\" subtitle \"Update ready\"",
        info.version
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
    let _ = std::process::Command::new("open").arg(&info.download_url).output();
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
