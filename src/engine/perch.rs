//! Perching on windows: now and then a pet hops onto the top edge of the front window and roams along it.
//! When the window moves, the pet rides along; when it closes, changes size or another window comes to the
//! front, the pet jumps down.

use super::World;
use crate::domain::Activity::*;
use crate::domain::Pet;
use crate::support::*;

/// How likely free will hops up onto the front window, when it can.
pub const PERCH_CHANCE: f64 = 0.08;

/// Windows whose size changes by less than this (points) are taken to be the same window.
const SAME_SIZE: f64 = 2.0;

impl World {
    /// Where the pet could land on the front window, if it's worth a hop: the window's top edge, under the pet
    /// as far as the window reaches, with room above it for the pet.
    pub(super) fn perch_spot(&self, p: &Pet) -> Option<Point> {
        let r = self.perch?;
        if self.fullscreen || p.perched.is_some() || p.floor < 0.0 || !p.on_ground() || !p.supports(Hop) { return None }
        let (top, half) = (r.max_y() - self.ground, p.size.width / 2.0);
        let room = top + p.size.height + 20.0 <= self.height - self.ground;
        let wide = r.w >= p.size.width * 2.0;
        // Close to where the pet already lives, it's not worth calling a perch.
        let higher = (top - p.floor).abs() > 40.0 * self.s && top > 40.0 * self.s;
        (room && wide && higher).then(|| pt(clamp(p.x, r.min_x() + half, r.max_x() - half), top))
    }

    /// Hops up onto the front window.
    pub(super) fn perch_on(&mut self, p: &mut Pet, spot: Point) {
        self.start(p, Hop, false);
        p.leap_to = spot;
        p.facing = if spot.x >= p.x { 1.0 } else { -1.0 };
        p.perch_pending = self.perch;
    }

    /// Landed from a hop: on the window it was aiming for, that window is now home.
    pub(super) fn land(&mut self, p: &mut Pet) {
        let Some(r) = p.perch_pending.take() else { return };
        if (p.floor - (r.max_y() - self.ground)).abs() > 1.0 { return }
        p.perched = Some(r);
        p.span = Some(Span { lo: r.min_x(), hi: r.max_x() });
    }

    /// Keeps perched pets with their window: riding along when it moves, jumping down when it's gone.
    pub(super) fn follow_perch(&mut self, p: &mut Pet) {
        let Some(old) = p.perched else { return };
        // It went somewhere else by itself (hopped, was dragged, went to bed…).
        if p.leaping || p.activity == Dragged || (p.floor - (old.max_y() - self.ground)).abs() > 1.0 {
            if !p.leaping || p.activity != Hop { p.perched = None }
            return;
        }
        match self.perch {
            Some(new) if (new.w - old.w).abs() < SAME_SIZE && (new.h - old.h).abs() < SAME_SIZE => {
                if new == old { return }
                let top = new.max_y() - self.ground;
                if top + p.size.height > self.height - self.ground {
                    return self.jump_down(p);
                }
                let (dx, dy) = (new.min_x() - old.min_x(), new.max_y() - old.max_y());
                p.x += dx;
                if let Some(t) = &mut p.target_x { *t += dx }
                (p.floor, p.y) = (top, p.y + dy);
                p.span = Some(Span { lo: new.min_x(), hi: new.max_x() });
                p.perched = Some(new);
            }
            _ => self.jump_down(p),
        }
    }

    fn jump_down(&mut self, p: &mut Pet) {
        p.perched = None;
        self.go_home(p, true);
        p.say(pick(&["Whoa!", "Hey! 🙀", "Wheee!"]), 1.6);
    }
}

#[cfg(test)]
#[path = "perch_tests.rs"]
mod tests;
