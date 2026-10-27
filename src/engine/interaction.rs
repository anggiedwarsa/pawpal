//! How pets react to the user: clicks, being put down, and full-screen apps.

use super::World;
use crate::domain::Activity::*;
use crate::domain::Pet;
use crate::support::*;

/// How long a pet woken by a double-click stays up before going back to bed.
pub const WAKE_TIME: f64 = 30.0;

impl World {
    /// A click (or double-click) on a pet that wasn't dragged.
    pub fn clicked(&mut self, p: &mut Pet, clicks: u32) {
        if p.activity == Remind {
            // Got it: the pet goes back to what it was doing.
            self.end_remind(p);
        } else if p.activity == VisitCursor {
            if clicks >= 2 {
                self.go_home(p, true);
                p.say("Okay, going home!", 2.0);
            } else {
                p.say("Purrr ♥", 1.5);
                p.happiness = (p.happiness + 8.0).min(100.0);
                self.emit("❤️", self.head_point(p), 12.0);
            }
        } else if clicks >= 2 {
            if p.activity == Sleep {
                p.back_to_sleep = Some(WAKE_TIME);
                p.say("Okay, okay, I'm up! 🥱", 2.0);
            }
            self.start(p, Jump, true);
            self.emit("✨", self.head_point(p), 14.0);
        } else if p.activity == Sleep {
            // Too sleepy to get up; a double-click wakes it properly.
            p.say("What's up? I'm so sleepy… 😴", 2.4);
            self.emit_with("z", self.head_point(p), 12.0, pt(p.facing * 8.0, 18.0), 2.4);
        } else {
            self.start(p, Petted, true);
        }
    }

    /// A pet let go after being dragged: dropped up on the screen, it makes that spot home;
    /// dropped near the Dock, it lands on it.
    pub fn dropped(&mut self, p: &mut Pet, vx: f64) {
        self.settle(p);
        self.start(p, if p.y > p.floor { Falling } else { Idle }, false);
        p.vx = clamp(vx, -500.0, 500.0);
    }

    /// While something plays full screen, pets duck out of the way and peek from the bottom-right corner.
    pub fn set_fullscreen(&mut self, on: bool) {
        if on == self.fullscreen { return }
        self.fullscreen = on;
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                if on && !matches!(p.activity, Dragged | Remind) {
                    w.start(p, Hide, false);
                } else if !on && p.activity == Hide {
                    w.start(p, Idle, false);
                    p.say(if w.sharing { "Done sharing? 🐾" } else { "Is it over? 🍿" }, 2.0);
                }
            });
        }
    }

    /// Where the `slot`-th pet waits in the bottom-right corner, counting leftwards.
    pub fn corner_x(&self, p: &Pet, slot: usize) -> f64 {
        self.width - p.size.width / 2.0 - 6.0 - slot as f64 * p.size.width * 0.8
    }

    /// The level of the bottom-right corner at `x`: the screen's bottom edge where the Dock doesn't reach, else the Dock.
    pub fn corner_floor(&self, p: &Pet, x: f64) -> f64 {
        if self.below > 0.0 && x - p.size.width / 2.0 >= self.dock_end - 1.0 { -self.below } else { 0.0 }
    }

    /// The strip of screen bottom between the end of the Dock and the right edge.
    pub fn corner_span(&self) -> Span { Span { lo: self.dock_end, hi: self.width } }

    /// How many other pets are already in (or on their way to) the corner doing `a`.
    pub fn corner_slot(&self, p: &Pet, a: crate::domain::Activity) -> usize {
        self.pets.iter().filter(|o| o.id != 0 && o.id != p.id && o.activity == a).count()
    }

    /// Ducks down, pops up in the corner and stays there with only the top of its face showing.
    pub(super) fn hide(&mut self, p: &mut Pet, dt: f64) {
        p.vx = 0.0;
        p.vy = 0.0;
        match p.step {
            0 => {
                p.sink = (p.sink + dt / 0.4).min(1.0);
                if p.sink >= 1.0 {
                    p.x = p.target_x.unwrap_or_else(|| self.corner_x(p, 0));
                    p.floor = self.corner_floor(p, p.x);
                    (p.y, p.span, p.facing) = (p.floor, None, -1.0);
                    p.step = 1;
                }
            }
            // Peeking out of the corner, or out of sight altogether while the screen is shared.
            _ => p.sink = if self.sharing { 1.0 } else { (p.sink - dt / 0.5).max(0.5) },
        }
    }
}

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod tests;
