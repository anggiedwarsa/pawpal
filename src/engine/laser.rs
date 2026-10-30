//! The laser pointer toy: a red dot at the cursor that pets stalk, pounce on and hop up or down after.

use super::World;
use crate::domain::Activity::*;
use crate::domain::Pet;
use crate::support::*;

/// How likely free will goes back to chasing the dot while the laser is on.
pub const LASER_CHANCE: f64 = 0.85;
/// Seconds the cursor can stay still before the laser switches itself off.
pub const LASER_IDLE: f64 = 60.0;

/// Activities the dot doesn't pull a pet away from.
fn hooked_elsewhere(p: &Pet) -> bool { matches!(p.activity, Dragged | Falling | Hop | Sleep | Hide | Remind | VisitCursor | Fly) || p.leaping }

impl World {
    /// Turns the laser on or off. Pets drop what they're doing for it, and wonder where it went afterwards.
    pub fn set_laser(&mut self, on: bool) {
        if on == self.laser { return }
        self.laser = on;
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                if on && !hooked_elsewhere(p) && p.supports(ChaseCursor) {
                    w.start(p, ChaseCursor, false);
                } else if !on && p.activity == ChaseCursor {
                    p.timer = 0.0;
                    if chance(0.5) { p.say("Where'd it go? 👀", 2.0) }
                }
            });
        }
    }

    /// Chasing the dot: run under it, creep up when it's close, pounce when it's just above,
    /// and hop up or down a level when it's far above or below.
    pub(super) fn chase_laser(&mut self, p: &mut Pet, dt: f64) {
        let s = self.s;
        p.timer = p.timer.max(1.0); // it never tires of it while the laser is on
        let dot = self.mouse;
        let above = dot.y - self.ground - p.floor;
        if (above > 220.0 * s || above < -20.0 * s) && p.on_ground() && p.cooldown <= 0.0 && p.supports(Hop) {
            let top = (self.height - self.ground - p.size.height - 20.0).max(0.0);
            let mut y = clamp(dot.y - self.ground - p.size.height * 0.3, 0.0, top);
            if y < 30.0 * s { y = 0.0 }
            let half = p.size.width / 2.0;
            let x = p.x + clamp(dot.x - p.x, -200.0 * s, 200.0 * s);
            p.cooldown = 0.8;
            self.start(p, Hop, false);
            p.leap_to = pt(if y == 0.0 { self.clamp_to_dock(x, half) } else { clamp(x, half, self.width - half) }, y);
            p.facing = if p.leap_to.x >= p.x { 1.0 } else { -1.0 };
            return;
        }
        let dx = self.clamp_x(p, dot.x, p.size.width / 2.0) - p.x;
        if dx.abs() > 30.0 * s {
            // Dashes over, then creeps up on it.
            self.move_to(p, dot.x, if dx.abs() > 120.0 * s { 220.0 * s } else { 70.0 * s });
            return;
        }
        p.vx = 0.0;
        p.facing = if dot.x >= p.x { 1.0 } else { -1.0 };
        if !p.on_ground() || p.cooldown > 0.0 { return }
        if (10.0 * s..160.0 * s).contains(&above) {
            p.vy = (300.0 * s + above * 2.0).min(560.0 * s);
            p.cooldown = 1.2;
            if chance(0.4) { self.emit("✨", self.head_point(p), 13.0) }
        } else if chance(dt * 1.5) {
            // Pinning it to the ground.
            p.vy = 200.0 * s;
            p.cooldown = 1.0;
        }
    }
}

#[cfg(test)]
#[path = "laser_tests.rs"]
mod tests;
