//! Finding out whether a newer PawPal is out, from the project's latest GitHub release.

use std::process::Command;

/// Where releases are published, and the page the About box links to.
pub const WEBSITE: &str = "https://github.com/anggiedwarsa/pawpal";
const LATEST: &str = "https://api.github.com/repos/anggiedwarsa/pawpal/releases/latest";
/// How often to look again while PawPal keeps running.
pub const CHECK_EVERY: f64 = 24.0 * 3600.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Without the tag's leading "v".
    pub version: String,
    /// The release's page, with its downloads.
    pub url: String,
}

/// Reads the latest release from GitHub's answer; None for a draft, a pre-release or anything unexpected.
pub fn parse_latest(json: &str) -> Option<Release> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) { return None }
    let tag = v["tag_name"].as_str()?;
    let version = tag.strip_prefix('v').unwrap_or(tag).to_string();
    let url = v["html_url"].as_str().map_or_else(|| format!("{WEBSITE}/releases/latest"), String::from);
    Some(Release { version, url })
}

/// Whether `latest` is a later version than `current`, comparing numbers part by part ("1.10" > "1.9").
pub fn is_newer(latest: &str, current: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.split(['.', '-', '+']).map_while(|n| n.parse().ok()).collect() };
    let (a, b) = (parts(latest), parts(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y { return x > y }
    }
    false
}

/// Asks GitHub for the latest release, with the system's curl so PawPal needs no TLS library of its own.
/// Slow (it goes over the network), so it's run off the main thread; None when offline or curl is missing.
pub fn fetch_latest() -> Option<Release> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "15", "-H", "Accept: application/vnd.github+json", "-A", "PawPal", LATEST]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashing up
    }
    let out = cmd.output().ok()?;
    if !out.status.success() { return None }
    parse_latest(&String::from_utf8_lossy(&out.stdout))
}

/// The newer release, if the latest one is newer than this build.
pub fn newer_than_this(latest: Option<Release>) -> Option<Release> {
    latest.filter(|r| is_newer(&r.version, env!("CARGO_PKG_VERSION")))
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod tests;
