use std::process::Command;

fn main() {
    // Prefer an explicit version from CI (e.g. "0.2.0" or "0.2.0-nightly.abc1234").
    // Fall back to CARGO_PKG_VERSION + git short SHA for local dev builds.
    let version = if let Ok(v) = std::env::var("MIN_MAX_VERSION") {
        v
    } else {
        let base = env!("CARGO_PKG_VERSION");
        let sha = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if sha.is_empty() {
            base.to_string()
        } else {
            format!("{base}-dev.{sha}")
        }
    };

    println!("cargo:rustc-env=MIN_MAX_VERSION={version}");
    // Re-run only when the git HEAD changes or the env var changes.
    println!("cargo:rerun-if-env-changed=MIN_MAX_VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}
