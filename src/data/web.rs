//! Fetching a web page with the system's curl, so PawPal needs no TLS library of its own.

use std::process::Command;

/// The body at `url`, or None when offline, on an error status, or if curl is missing.
/// Slow (it goes over the network), so call it off the main thread.
pub fn get(url: &str, headers: &[&str]) -> Option<String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "20", "-A", "PawPal"]);
    for h in headers { cmd.args(["-H", h]); }
    cmd.arg("--").arg(url);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashing up
    }
    let out = cmd.output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}
