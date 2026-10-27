//! Focus mode: for a set time, or while the user is in a call, the pets curl up in their beds and leave
//! the user be, then wake up. Calendar heads-ups and the user's own reminders still come through.

use super::World;
use crate::domain::Activity::*;

/// The focus lengths in the tray menu (minutes).
pub const FOCUS_FOR: [u32; 3] = [25, 50, 90];
/// How long (seconds) without any sign of a call before it counts as over, so a moment with the mic
/// closed or the Meet tab hidden doesn't wake the pets.
pub const CALL_GRACE: f64 = 45.0;

/// Tells when a call starts and ends from how often it's seen.
#[derive(Default)]
pub struct CallWatch {
    pub on: bool,
    last_seen: f64,
}

impl CallWatch {
    /// At `now` (seconds since the epoch), with a call `seen` or not: Some(true) as a call starts,
    /// Some(false) once it's been gone `CALL_GRACE`.
    pub fn update(&mut self, now: f64, seen: bool) -> Option<bool> {
        if seen { self.last_seen = now }
        let on = seen || (self.on && now - self.last_seen < CALL_GRACE);
        (on != std::mem::replace(&mut self.on, on)).then_some(on)
    }
}

impl World {
    /// Sends the pets to bed; one of them says `line`.
    pub fn start_focus(&mut self, line: &str) {
        self.focusing = true;
        let mut told = false;
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                if matches!(p.activity, Dragged | Remind) { return }
                if p.activity != Sleep || p.commanded { w.start(p, Sleep, false) }
                if !told {
                    p.say(line, 3.0);
                    told = true;
                }
            });
        }
    }

    /// Focus time is up (or ended early): the pets wake and one comes to say well done.
    pub fn end_focus(&mut self, finished: bool) {
        if !self.focusing { return }
        self.focusing = false;
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                if p.activity == Sleep && !p.commanded && !w.sleeps_through(p) {
                    w.start(p, if p.supports(Stretch) { Stretch } else { Idle }, false);
                }
            });
        }
        if finished { self.remind("⭐ Focus done, great work! Time for a short break.") }
    }
}

#[cfg(test)]
#[path = "focus_tests.rs"]
mod tests;
