//! Toys, food and particles: how they move, and the commands that create them.
//! Toys and food appear on the level of the pet they're for, so a pet living up the screen plays there.

use super::World;
use crate::domain::Activity::*;
use crate::domain::{same_level, Ball, Bowl, Butterfly, Pet};
use crate::support::*;

impl World {
    pub fn update_props(&mut self, dt: f64) {
        let s = self.s;
        let r = 7.0 * s;
        for b in &mut self.balls {
            b.life -= dt;
            b.vy -= 1100.0 * s * dt;
            b.x += b.vx * dt;
            b.y += b.vy * dt;
            if b.y <= 0.0 {
                b.y = 0.0;
                b.vy = if b.vy.abs() > 60.0 * s { -b.vy * 0.55 } else { 0.0 };
                b.vx *= 0.25f64.powf(dt);
            }
            if b.x < b.area.lo + r { b.x = b.area.lo + r; b.vx = b.vx.abs() * 0.7 }
            if b.x > b.area.hi - r { b.x = b.area.hi - r; b.vx = -b.vx.abs() * 0.7 }
            b.rot -= b.vx * dt / r;
        }
        self.balls.retain(|b| b.life > 0.0);

        for b in &mut self.bowls {
            b.life -= dt;
            if b.food <= 0.0 { b.life = b.life.min(2.0) }
        }
        self.bowls.retain(|b| b.life > 0.0);

        for b in &mut self.butterflies {
            b.t += dt;
            b.life -= dt;
            if b.life < 3.0 { b.flee = 1.0 }
            if chance(dt * 0.4) { b.dir *= -1.0 }
            b.x += b.dir * (40.0 + b.flee * 60.0) * s * dt;
            let base_y = (70.0 + (b.t * 0.7).sin() * 40.0) * s + b.flee * 120.0 * s;
            b.y = lerp(b.y, base_y + (b.t * 5.0).sin() * 10.0 * s, (dt * 2.0).min(1.0));
            b.flee = (b.flee - dt * 0.5).max(0.0);
            if b.x < b.area.lo + 20.0 { b.dir = 1.0 }
            if b.x > b.area.hi - 20.0 { b.dir = -1.0 }
        }
        self.butterflies.retain(|b| b.life > 0.0);

        for q in &mut self.particles {
            q.ttl -= dt;
            q.p.x += q.v.x * dt;
            q.p.y += q.v.y * dt;
        }
        self.particles.retain(|q| q.ttl > 0.0);
    }

    /// Puts a bowl down in front of every pet.
    pub fn feed(&mut self) {
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| w.feed_one(p));
        }
    }

    /// Puts a bowl down in front of one pet, on the level it lives on.
    pub fn feed_one(&mut self, p: &mut Pet) {
        let x = self.clamp_x(p, p.x + p.facing * (p.size.width / 2.0 + 30.0 * self.s), 30.0 * self.s);
        self.bowls.push(Bowl { x, food: 1.0, life: 120.0, floor: p.floor });
        if p.activity != Dragged && p.supports(Eat) { self.start(p, Eat, false) }
    }

    pub fn treat(&mut self) {
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                p.hunger = (p.hunger - 15.0).max(0.0);
                p.happiness = (p.happiness + 8.0).min(100.0);
                w.emit(p.species.line("🐟", "🦴"), w.head_point(p), 14.0);
                if p.activity != Dragged { w.start(p, Petted, false) }
                p.say("Yum! Thank you ♥", 2.6);
            });
        }
    }

    /// The level and stretch of a random pet, or the Dock when there are none.
    fn random_level(&self) -> (f64, f64, Span) {
        let real: Vec<&Pet> = self.pets.iter().filter(|p| p.id != 0).collect();
        match real.get(fastrand::usize(..real.len().max(1))) {
            Some(p) => (p.x, p.floor, self.range(p)),
            None => ((self.min_x + self.max_x) / 2.0, 0.0, Span { lo: self.min_x, hi: self.max_x }),
        }
    }

    pub fn throw_ball(&mut self) {
        let (x, floor, area) = self.random_level();
        self.throw_ball_at(x, floor, area);
    }

    /// Throws a ball near x on one level; every pet there that can play joins in.
    pub fn throw_ball_at(&mut self, x: f64, floor: f64, area: Span) {
        let s = self.s;
        let x = clamp(x + rnd(-80.0, 80.0), area.lo + 20.0, (area.lo + 21.0).max(area.hi - 20.0));
        self.balls.push(Ball { x, y: 140.0 * s, vx: rnd(-150.0, 150.0) * s, vy: 100.0 * s, rot: 0.0, life: 60.0, floor, area });
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                if same_level(p.floor, floor) && !matches!(p.activity, Dragged | Sleep) && p.supports(PlayBall) { w.start(p, PlayBall, false) }
            });
        }
    }

    pub fn release_butterfly(&mut self) {
        let (_, floor, area) = self.random_level();
        self.release_butterfly_at(floor, area);
    }

    /// Lets a butterfly go above one level; every pet there that can chase it joins in.
    pub fn release_butterfly_at(&mut self, floor: f64, area: Span) {
        const COLORS: [u32; 5] = [0xFF8FB1, 0xFFD166, 0x7FDBFF, 0xC39BFF, 0xFFA552];
        let x = rnd(area.lo + 60.0, (area.lo + 61.0).max(area.hi - 60.0));
        let color = Rgba::hex(pick(&COLORS));
        self.butterflies.push(Butterfly { x, y: 160.0 * self.s, t: 0.0, dir: 1.0, flee: 0.0, life: 30.0, color, floor, area });
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                if same_level(p.floor, floor) && !matches!(p.activity, Dragged | Sleep) && p.supports(ChaseButterfly) {
                    w.start(p, ChaseButterfly, false)
                }
            });
        }
    }

    pub fn kick_ball(&mut self, i: usize) {
        let s = self.s;
        let b = &mut self.balls[i];
        b.vx = rnd(-300.0, 300.0) * s;
        b.vy = rnd(250.0, 420.0) * s;
        b.life = 60.0;
    }
}

#[cfg(test)]
#[path = "props_physics_tests.rs"]
mod tests;
