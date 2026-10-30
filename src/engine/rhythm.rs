//! The pets' day: stretching in the morning, playful in the afternoon, calmer in the evening,
//! and asleep in the corner from bedtime until morning. Only free will follows it; commands don't.

use super::World;
use crate::domain::Activity::{self, *};

/// When night ends (local hour).
pub const WAKE: f64 = 7.0;
/// The bedtimes offered in the tray menu (local hour; 0 is midnight).
pub const BEDTIMES: [u32; 4] = [21, 22, 23, 0];
pub const DEFAULT_BEDTIME: u32 = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayPart {
    Morning,
    Midday,
    Afternoon,
    Evening,
    Night,
}

/// What part of the day `hour` (local, 0–24) is with this bedtime; with none, the day has no rhythm.
pub fn day_part(hour: f64, bedtime: Option<u32>) -> DayPart {
    let Some(bed) = bedtime else { return DayPart::Midday };
    // A bedtime of midnight puts the evening's end at 24.
    let bed = if bed < 12 { bed as f64 + 24.0 } else { bed as f64 };
    let h = if hour < WAKE { hour + 24.0 } else { hour };
    match h {
        h if h >= bed => DayPart::Night,
        h if h < 11.0 => DayPart::Morning,
        h if (13.0..18.0).contains(&h) => DayPart::Afternoon,
        h if h >= 18.0 => DayPart::Evening,
        _ => DayPart::Midday,
    }
}

/// How much more (or less) likely free will picks `a` at this time of day.
pub fn weight(day: DayPart, a: Activity) -> f64 {
    match (day, a) {
        (DayPart::Morning, Stretch | Groom) => 2.5,
        (DayPart::Morning, Sniff) => 1.5,
        (DayPart::Morning, Run) => 0.6,
        (DayPart::Afternoon, Run | Pounce | Jump | Dance | Hop | PlayFriend | PlayFight) => 1.8,
        (DayPart::Afternoon, Sleep) => 0.5,
        (DayPart::Evening, Lounge | Knead | Sit | Groom) => 1.6,
        (DayPart::Evening, Run | Pounce | Dance | Jump) => 0.5,
        (DayPart::Evening, Sleep) => 1.5,
        _ => 1.0,
    }
}

impl World {
    pub fn day(&self) -> DayPart { day_part(self.hour, self.bedtime) }
}

#[cfg(test)]
#[path = "rhythm_tests.rs"]
mod tests;
