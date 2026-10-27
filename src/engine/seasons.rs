//! Seasonal touches: now and then a little flurry drifts down over the Dock, such as snow in winter,
//! leaves in autumn, blossoms in spring, plus bats on Halloween and confetti at New Year. Seasons follow
//! the northern hemisphere.

use super::World;
use crate::domain::Activity::*;
use crate::support::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Touch {
    Snow,
    Leaves,
    Blossoms,
    Halloween,
    NewYear,
}

/// Seconds between flurries, and how long one lasts.
const EVERY: (f64, f64) = (180.0, 360.0);
const FLURRY: f64 = 8.0;
/// Seconds between bits of one flurry.
const BIT_EVERY: f64 = 0.35;
/// Highest a bit starts (points above the Dock at size 1), so it stays in the strip the window covers when compact.
const TOP: f64 = 130.0;

/// What's in the air on this date (and hour, for New Year's Eve).
pub fn touch(month: u32, day: u32, hour: f64) -> Option<Touch> {
    match (month, day) {
        (12, 31) if hour >= 20.0 => Some(Touch::NewYear),
        (1, 1) if hour < 3.0 => Some(Touch::NewYear),
        (10, 31) => Some(Touch::Halloween),
        (12 | 1, _) => Some(Touch::Snow),
        (10 | 11, _) => Some(Touch::Leaves),
        (4, _) => Some(Touch::Blossoms),
        _ => None,
    }
}

impl Touch {
    fn bits(self) -> &'static [&'static str] {
        match self {
            Touch::Snow => &["❄️", "❄", "✻"],
            Touch::Leaves => &["🍂", "🍁"],
            Touch::Blossoms => &["🌸", "🌼"],
            Touch::Halloween => &["🦇", "🎃", "👻"],
            Touch::NewYear => &["🎉", "🎊", "✨"],
        }
    }

    /// What a pet says when a flurry starts.
    fn line(self, month: u32) -> &'static str {
        match self {
            Touch::Snow => "❄️ Ooh, snow!",
            Touch::Leaves => "🍂 Crunchy leaves!",
            Touch::Blossoms => "🌸 Spring is here!",
            Touch::Halloween => "🎃 Boo! Happy Halloween!",
            Touch::NewYear if month == 1 => "🎉 Happy New Year!",
            Touch::NewYear => "🎉 Almost New Year!",
        }
    }
}

/// When the next flurry comes and what's left of the current one.
pub struct Flurry {
    pub next: f64,
    pub left: f64,
    bit_in: f64,
}

impl Default for Flurry {
    /// The first flurry comes soon after starting, so it's noticed.
    fn default() -> Flurry { Flurry { next: 30.0, left: 0.0, bit_in: 0.0 } }
}

impl World {
    pub(super) fn seasonal_step(&mut self, dt: f64) {
        let Some(touch) = self.touch else { return };
        // Nothing drifts in front of a film or while the user is focusing.
        if self.fullscreen || self.focusing { return }
        let f = &mut self.flurry;
        if f.left <= 0.0 {
            f.next -= dt;
            if f.next > 0.0 { return }
            (f.next, f.left, f.bit_in) = (rnd(EVERY.0, EVERY.1), FLURRY, 0.0);
            let month = self.month;
            let awake: Vec<usize> = (0..self.pets.len()).filter(|&i| !matches!(self.pets[i].activity, Sleep | Hide | Remind | Dragged)).collect();
            if let (false, true) = (awake.is_empty(), chance(0.6)) {
                self.pets[pick(&awake)].say(touch.line(month), 2.6);
            }
            return;
        }
        f.left -= dt;
        f.bit_in -= dt;
        if f.bit_in > 0.0 { return }
        f.bit_in = BIT_EVERY;
        let s = self.s;
        let at = pt(rnd(self.min_x + 20.0 * s, (self.min_x + 21.0 * s).max(self.max_x - 20.0 * s)), self.ground + rnd(90.0, TOP) * s);
        self.emit_with(pick(touch.bits()), at, 13.0, pt(rnd(-12.0, 12.0), -rnd(20.0, 30.0)), 4.0);
    }
}

#[cfg(test)]
#[path = "seasons_tests.rs"]
mod tests;
