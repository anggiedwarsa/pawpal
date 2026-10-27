//! Low battery: on a laptop running on battery, a pet comes out at 10% and again at 5% to ask for the charger.

/// Battery levels (percent) where a pet says so, highest first.
pub const MARKS: [u32; 2] = [10, 5];
/// Charged back above this (or plugged in), the heads-ups can come again.
const RESET_ABOVE: f64 = 15.0;

/// Which heads-ups have been given since the battery was last charged.
#[derive(Default)]
pub struct BatteryWatch {
    told: Option<u32>,
}

impl BatteryWatch {
    /// The mark due at this battery level, if any. Only the lowest mark reached counts, so starting at 4% gives one
    /// heads-up, not two.
    pub fn due(&mut self, percent: f64, charging: bool) -> Option<u32> {
        if charging || percent > RESET_ABOVE {
            self.told = None;
            return None;
        }
        let mark = MARKS.into_iter().filter(|&m| percent <= m as f64).min()?;
        if self.told.is_some_and(|t| t <= mark) { return None }
        self.told = Some(mark);
        Some(mark)
    }
}

/// What the pet says.
pub fn battery_text(mark: u32) -> String {
    if mark <= 5 { format!("🪫 Only {mark}% battery left! Plug in soon") } else { format!("🔋 Battery's at {mark}%… plug me in?") }
}

#[cfg(test)]
#[path = "battery_tests.rs"]
mod tests;
