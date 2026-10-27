//! Welcome back: when the user returns after a while away from the keyboard and mouse, a pet says hello.

use super::breaks::AWAY;
use super::World;
use crate::domain::Activity::*;

/// After this long away (seconds), the pet missed the user and is extra glad.
pub const MISSED: f64 = 3600.0;

/// Notices the user coming back.
#[derive(Default)]
pub struct Presence {
    /// When the user last pressed a key or moved the mouse (seconds since the epoch), as of the last look.
    last_input: Option<f64>,
}

impl Presence {
    /// At `now` (seconds since the epoch), with the last input `idle` seconds ago: how long the user was away,
    /// if they've just come back after `AWAY` or more. A computer asleep in between counts as away.
    pub fn back(&mut self, now: f64, idle: f64) -> Option<f64> {
        let input = now - idle;
        let gap = input - self.last_input.replace(input)?;
        (gap >= AWAY).then_some(gap)
    }
}

impl World {
    /// The pet nearest the cursor greets the user. Sleeping, hiding or busy pets are left alone.
    pub fn welcome_back(&mut self, away: f64) {
        if self.fullscreen { return }
        let mx = self.mouse.x;
        let Some(i) = (0..self.pets.len())
            .filter(|&i| !matches!(self.pets[i].activity, Dragged | Falling | Sleep | Hide | Remind | VisitCursor | Hop))
            .min_by(|&a, &b| (self.pets[a].x - mx).abs().total_cmp(&(self.pets[b].x - mx).abs()))
        else { return };
        self.with_pet(i, |w, p| {
            p.facing = if mx >= p.x { 1.0 } else { -1.0 };
            if away >= MISSED {
                w.start(p, if p.supports(Dance) { Dance } else { Wave }, false);
                p.timer = 3.0;
                p.happiness = (p.happiness + 5.0).min(100.0);
                w.emit("💞", w.head_point(p), 14.0);
                p.say("You're back! I missed you 💞", 3.0);
            } else {
                if p.supports(Wave) { w.start(p, Wave, false) }
                p.say("You're back! 🐾", 2.6);
            }
        });
    }
}

#[cfg(test)]
#[path = "greet_tests.rs"]
mod tests;
