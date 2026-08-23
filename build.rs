use std::process::Command;

fn main() {
    let version = std::env::var("GITHUB_REF_NAME")
        .ok()
        .filter(|s| s.starts_with('v'))
        .or_else(|| {
            Command::new("git")
                .args(["describe", "--tags", "--abbrev=0"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    // Strip a leading "v" so VTERM_VERSION matches the bare version the GitHub
    // release tag is compared against (e.g. "0.1.3", not "v0.1.3").
    let version = version.trim_start_matches('v').to_string();

    println!("cargo:rustc-env=VTERM_VERSION={}", version);
}
