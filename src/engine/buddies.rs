//! Pets together: napping side by side, grooming each other and tagging along on each other's walks.

use super::World;
use crate::domain::Activity::*;
use crate::domain::{same_level, Pet};
use crate::support::*;

/// How likely free will picks something to do with another pet, when there's one to do it with.
pub const BUDDY_CHANCE: f64 = 0.25;
/// How likely a pet joins a friend that's gone to bed, if it could do with a rest itself.
pub const SNUGGLE_CHANCE: f64 = 0.5;
/// Pets with less energy than this join a napping friend.
pub const SNUGGLE_ENERGY: f64 = 70.0;
/// How long grooming a friend lasts, walking over included.
pub const GROOM_TIME: f64 = 9.0;
/// How far away (points, at size 1) a friend can be for a pet to go over to it.
const FRIEND_RANGE: f64 = 400.0;

/// Activities during which a friend keeps still enough to be groomed.
fn still(p: &Pet) -> bool { matches!(p.activity, Sit | Idle | Lounge | Groom | Sleep) }

impl World {
    /// Free will with friends around: join one that's gone to bed, groom one sitting nearby,
    /// or tag along on one's walk. Returns whether it picked something.
    pub(super) fn buddy_up(&mut self, p: &mut Pet) -> bool {
        let napping = self.pets.iter().any(|o| o.id != 0 && o.id != p.id && o.activity == Sleep && !o.commanded);
        if napping && p.energy < SNUGGLE_ENERGY && p.supports(Sleep) && chance(SNUGGLE_CHANCE) {
            self.start(p, Sleep, false);
            p.say(p.line("cuddle", "Room for one more? 💕", "Scoot over! 💕"), 2.6);
            return true;
        }
        if !p.can_move() || !chance(BUDDY_CHANCE) { return false }
        if p.supports(Groom) {
            if let Some(id) = self.friend(p, |o| still(o) && o.activity != Sleep && o.buddy.is_none()) {
                self.start(p, Groom, false);
                (p.buddy, p.timer) = (Some(id), GROOM_TIME);
                return true;
            }
        }
        if let Some(id) = self.friend(p, |o| o.activity == Walk && o.buddy.is_none()) {
            self.start(p, Walk, false);
            (p.buddy, p.timer) = (Some(id), rnd(6.0, 10.0));
            if chance(0.4) { p.say("Wait for me! 🐾", 2.0) }
            return true;
        }
        false
    }

    /// The nearest other pet on this one's level, within reach, that `ok` accepts.
    fn friend(&self, p: &Pet, ok: impl Fn(&Pet) -> bool) -> Option<u64> {
        self.pets
            .iter()
            .filter(|o| o.id != 0 && o.id != p.id && same_level(o.floor, p.floor) && (o.x - p.x).abs() < FRIEND_RANGE * self.s && ok(o))
            .min_by(|a, b| (a.x - p.x).abs().total_cmp(&(b.x - p.x).abs()))
            .map(|o| o.id)
    }

    /// Grooming a friend: walk up beside it, then lick its head while it holds still and both feel loved.
    pub(super) fn groom_buddy(&mut self, p: &mut Pet, dt: f64) {
        let s = self.s;
        let Some(j) = p.buddy.and_then(|id| self.index_of(id)) else { p.timer = 0.0; return };
        if !still(&self.pets[j]) || !same_level(self.pets[j].floor, p.floor) { p.timer = 0.0; return }
        let (fx, fw) = (self.pets[j].x, self.pets[j].size.width);
        let side = if fx >= p.x { 1.0 } else { -1.0 };
        if p.step == 0 {
            if self.move_to(p, fx - side * (fw / 2.0 + p.size.width * 0.3), 60.0 * s) {
                p.step = 1;
                p.say("*lick lick*", 1.6);
            }
            return;
        }
        p.vx = 0.0;
        p.facing = side;
        p.happiness = (p.happiness + dt).min(100.0);
        let f = &mut self.pets[j];
        f.timer = f.timer.max(1.0); // holds still until it's done
        f.happiness = (f.happiness + 2.0 * dt).min(100.0);
        if p.fx <= 0.0 {
            p.fx = 1.0;
            let at = self.head_point(&self.pets[j]);
            self.emit_with("💕", at, 12.0, pt(rnd(-10.0, 10.0), 25.0), 1.4);
        }
    }

    /// Tagging along: keeps a little way behind the friend, at its pace.
    pub(super) fn follow_buddy(&mut self, p: &mut Pet) {
        let s = self.s;
        let Some(j) = p.buddy.and_then(|id| self.index_of(id)) else { p.timer = 0.0; return };
        let f = &self.pets[j];
        if f.activity != Walk || !same_level(f.floor, p.floor) { p.timer = 0.0; return }
        let (fx, facing, speed) = (f.x, f.facing, f.vx.abs());
        let spot = fx - facing * (f.size.width / 2.0 + p.size.width / 2.0 + 8.0 * s);
        if (spot - p.x).abs() > (if p.vx == 0.0 { 30.0 } else { 6.0 }) * s {
            self.move_to(p, spot, (speed + 20.0 * s).max(45.0 * s));
        } else {
            p.vx = 0.0;
            p.facing = if fx >= p.x { 1.0 } else { -1.0 };
        }
    }

    /// Asleep right next to another sleeping pet.
    pub(super) fn snuggled(&self, p: &Pet) -> bool {
        self.pets.iter().any(|o| o.id != 0 && o.id != p.id && o.activity == Sleep && (o.x - p.x).abs() < p.size.width * 1.2 && same_level(o.floor, p.floor))
    }
}

#[cfg(test)]
#[path = "buddies_tests.rs"]
mod tests;
