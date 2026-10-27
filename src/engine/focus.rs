//! Focus mode: for a set time the pets curl up in their beds and leave the user be, then wake up
//! and suggest a break. Calendar heads-ups and the user's own reminders still come through.

use super::World;
use crate::domain::Activity::*;

/// The focus lengths in the tray menu (minutes).
pub const FOCUS_FOR: [u32; 3] = [25, 50, 90];

impl World {
    pub fn start_focus(&mut self, minutes: u32) {
        self.focusing = true;
        let mut told = false;
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                if matches!(p.activity, Dragged | Remind) { return }
                if p.activity != Sleep || p.commanded { w.start(p, Sleep, false) }
                if !told {
                    p.say(format!("Shh… focus time 🎯 See you in {minutes} min"), 3.0);
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
