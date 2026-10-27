//! Break nudges: after a long while at the computer, a pet comes out to suggest a stretch, a drink of water
//! or a breath away from the screen. Leaving the keyboard and mouse alone for a few minutes counts as a break.

use crate::support::pick;

/// Seconds without a key press or mouse move that count as having had a break.
pub const AWAY: f64 = 300.0;
/// The choices in the tray menu (minutes); 0 turns nudges off.
pub const BREAK_EVERY: [u32; 3] = [25, 50, 90];
pub const DEFAULT_BREAK_EVERY: u32 = 50;

const NUDGES: [&str; 4] = [
    "🧘 Time for a stretch!",
    "💧 Have a sip of water",
    "🌬️ Take a deep breath and look away from the screen",
    "🚶 Stand up and walk around a little",
];

/// How long the user has been at the computer without a break.
#[derive(Default)]
pub struct BreakTimer {
    worked: f64,
    /// When `due` last ran (seconds since the epoch).
    last: Option<f64>,
}

impl BreakTimer {
    /// Whether it's time for a nudge at `now` (seconds since the epoch), with the user's last key press or mouse
    /// move `idle` seconds ago. While `hold` is set (a movie is on, another heads-up is out) the nudge waits.
    pub fn due(&mut self, now: f64, idle: f64, every_min: u32, hold: bool) -> bool {
        let dt = self.last.map_or(0.0, |t| (now - t).max(0.0));
        self.last = Some(now);
        // A long gap between checks means the computer slept, which is a break too.
        if every_min == 0 || idle >= AWAY || dt >= AWAY {
            self.worked = 0.0;
            return false;
        }
        self.worked += dt;
        if hold || self.worked < every_min as f64 * 60.0 { return false }
        self.worked = 0.0;
        true
    }

    /// Starts counting again, as after a break.
    pub fn reset(&mut self) { self.worked = 0.0 }
}

/// What the pet suggests.
pub fn nudge_text() -> &'static str { pick(&NUDGES) }

#[cfg(test)]
#[path = "breaks_tests.rs"]
mod tests;
