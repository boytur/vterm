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

    println!("cargo:rustc-env=VTERM_VERSION={}", version);
}
