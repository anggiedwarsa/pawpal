//! How pets decide what to do and how each activity plays out.

use super::rhythm::{weight, DayPart};
use super::world::{on_dock, World, VISIT_DURATION, VISIT_FOLLOW_DISTANCE};
use crate::data::settings;
use crate::domain::Activity::{self, *};
use crate::domain::{Pet, Species};
use crate::support::*;

/// Stages of bedtime, in `Pet::step`.
pub const BED_WALK: i32 = 0;
pub const ASLEEP: i32 = 1;
pub const HOP_DOWN: i32 = 2;
pub const BED_HOP: i32 = 3;

impl World {
    pub fn start(&mut self, p: &mut Pet, a: Activity, commanded: bool) {
        let s = self.s;
        // Down in the corner (when pets keep to the Dock) there's only room to nap, hide or bounce;
        // anything else starts by hopping back up.
        let a = if p.floor < 0.0 && !self.low_roam() && !matches!(a, Sleep | Hide | Dragged | Jump | Falling | Petted | Hop | Remind) { Hop } else { a };
        if matches!(a, Dragged | Hop | Remind) { p.climbing = false }
        p.resume = None;
        p.activity = a;
        p.age = 0.0;
        p.step = 0;
        p.step_time = 0.0;
        p.target_x = None;
        p.commanded = commanded;
        p.leaping = p.climbing;
        if on_dock(a) && p.floor > 0.0 {
            p.floor = 0.0;
            p.span = None;
        }
        if a != Peekaboo { p.sink = 0.0 }
        match a {
            Idle => p.timer = rnd(2.0, 5.0),
            Walk => p.timer = rnd(5.0, 12.0),
            Sniff => p.timer = rnd(4.0, 8.0),
            Run => p.timer = 12.0,
            Sit => p.timer = rnd(5.0, 12.0),
            Lounge => p.timer = rnd(8.0, 18.0),
            Knead => {
                p.timer = rnd(5.0, 9.0);
                p.say(p.species.knead(), 2.6);
            }
            Sleep => {
                // Bedtime is in the bottom-right corner of the screen; the nap starts once the pet gets there.
                // A pet living up the screen hops down level by level on the way, then walks to the end of
                // the Dock and hops down into the corner.
                p.timer = f64::INFINITY;
                let bed = self.corner_x(p, self.corner_slot(p, Sleep));
                p.target_x = Some(if self.corner_floor(p, bed) < 0.0 { bed } else { self.clamp_to_dock(bed, p.size.width / 2.0) });
                if p.floor > 0.0 {
                    p.step = HOP_DOWN;
                    self.plan_hop_down(p);
                }
            }
            Groom => p.timer = rnd(3.0, 6.0),
            Stretch => p.timer = 2.4,
            Jump => {
                p.timer = 4.0;
                if p.on_ground() {
                    p.vy = 380.0 * s;
                    p.vx = if p.can_move() { p.facing * rnd(0.0, 60.0) * s } else { 0.0 };
                }
            }
            Pounce => p.timer = 5.0,
            Eat => {
                p.timer = 18.0;
                if commanded && self.nearest_bowl(p).is_none() { self.feed_one(p) }
            }
            PlayBall => {
                p.timer = 25.0;
                if commanded && self.nearest_ball(p).is_none() { self.throw_ball_at(p.x, p.floor, self.range(p)) }
            }
            ChaseButterfly => {
                p.timer = 22.0;
                if commanded && self.nearest_butterfly(p).is_none() { self.release_butterfly_at(p.floor, self.range(p)) }
            }
            ChaseCursor => p.timer = 12.0,
            VisitCursor => {
                p.timer = VISIT_DURATION;
                p.vx = 0.0;
                p.vy = 0.0;
                self.visit_anchor = self.mouse;
                p.leap_from = pt(p.x, p.y);
                p.leap_to = self.perch_point(p);
                p.leap_time = 0.0;
                p.say(p.species.line("Ooh, a mouse! 🐭", "Whatcha doing? 👀"), 2.0);
            }
            Hop => {
                p.timer = 4.0;
                p.vx = 0.0;
                p.vy = 0.0;
                p.leap_from = pt(p.x, p.y);
                p.leap_to = self.hop_target(p);
                p.leap_time = 0.0;
                p.leaping = true;
                p.facing = if p.leap_to.x >= p.x { 1.0 } else { -1.0 };
            }
            Dance => p.timer = 6.0,
            Wave => {
                p.timer = 3.0;
                p.say(p.species.hello(), 2.6);
            }
            Meow => {
                p.timer = 2.6;
                if p.bubble.is_none() { p.say(pick(p.species.sounds()), 2.6) }
            }
            Peekaboo => p.timer = 20.0,
            Remind => p.timer = f64::INFINITY,
            Hide => {
                p.timer = f64::INFINITY;
                p.target_x = Some(self.corner_x(p, self.corner_slot(p, Hide)));
            }
            PlayFriend => p.timer = 12.0,
            Petted => {
                p.timer = 2.2;
                p.happiness = (p.happiness + 12.0).min(100.0);
                p.say(pick(p.species.petted()), 2.6);
            }
            Dragged => p.timer = f64::INFINITY,
            Falling => p.timer = 6.0,
        }
    }

    fn finish(&mut self, p: &mut Pet) {
        if p.activity == Sleep {
            // At night it just sleeps on until morning.
            if self.sleeps_through(p) {
                p.timer = sleep_duration(p);
                return;
            }
            let morning = self.day() == DayPart::Morning;
            self.start(p, if p.supports(Stretch) { Stretch } else { Idle }, false);
            let yawn = p.species.line("*yaaawn*", "*big yawn*");
            if morning { p.say(format!("{yawn} Good morning! ☀️"), 3.0) } else { p.say(yawn, 2.6) }
            return;
        }
        self.choose_next(p);
    }

    /// Free will: pick the next activity from the pet's needs, what's around, and the clips it has.
    pub(super) fn choose_next(&mut self, p: &mut Pet) {
        // Nothing gets in front of a full-screen video: back to the corner.
        if self.fullscreen { return self.start(p, Hide, false) }
        if !settings::auto() {
            self.start(p, Idle, false);
            p.timer = 3.0;
            return;
        }
        let skin = p.skin.clone();
        let can = |a: Activity| skin.as_ref().is_some_and(|k| k.supports(a));
        if p.energy < 15.0 && can(Sleep) { return self.start(p, Sleep, false) }
        if self.nearest_bowl(p).is_some() && p.hunger > 10.0 && can(Eat) { return self.start(p, Eat, false) }
        if self.nearest_ball(p).is_some() && can(PlayBall) && chance(0.6) { return self.start(p, PlayBall, false) }
        if self.nearest_butterfly(p).is_some() && can(ChaseButterfly) && chance(0.7) { return self.start(p, ChaseButterfly, false) }
        if self.day() == DayPart::Night && can(Sleep) {
            if p.activity != Stretch { p.say("*yawn* Bedtime… 🌙", 2.6) }
            return self.start(p, Sleep, false);
        }
        if p.hunger > 70.0 && chance(0.35) {
            p.say(pick(p.species.hungry()), 3.0);
            let next = if can(Meow) { Meow } else { Idle };
            return self.start(p, next, false);
        }

        let mut options: Vec<(Activity, f64)> = vec![
            (Idle, 3.0), (Walk, 5.0), (Sniff, 1.5), (Run, if p.energy > 50.0 { 1.2 } else { 0.2 }), (Sit, 3.0), (Lounge, 2.0),
            (Groom, 2.0), (Knead, if p.happiness > 50.0 { 1.0 } else { 0.3 }), (Jump, 1.0), (Pounce, 1.0),
            (Dance, if p.happiness > 60.0 { 0.7 } else { 0.2 }), (Wave, 0.5), (Meow, 1.0), (Peekaboo, 0.5),
            (Sleep, if p.energy < 40.0 { 3.0 } else { 0.3 }), (Hop, 1.5), (Stretch, 0.4),
        ];
        if self.pets.len() > 1 { options.push((PlayFriend, 1.2)) }
        options.retain(|o| can(o.0));
        let day = self.day();
        for o in &mut options { o.1 *= weight(day, o.0) }
        let mut r = fastrand::f64() * options.iter().map(|o| o.1).sum::<f64>().max(0.001);
        for (a, w) in options {
            r -= w;
            if r < 0.0 { return self.start(p, a, false) }
        }
        self.start(p, Idle, false);
    }

    /// Whether a nap that's over carries on: at night, unless the pet was only sent for a nap.
    fn sleeps_through(&self, p: &Pet) -> bool { !p.commanded && settings::auto() && self.day() == DayPart::Night }

    /// Moves toward tx; returns true once arrived (or straight away for pets that can't travel).
    pub(super) fn move_to(&self, p: &mut Pet, tx: f64, speed: f64) -> bool {
        if !p.can_move() {
            p.vx = 0.0;
            return true;
        }
        // Aim only where the pet can actually stand, or it runs in place against the screen edge.
        let dx = self.clamp_x(p, tx, p.size.width / 2.0) - p.x;
        if dx.abs() < 4.0 {
            p.vx = 0.0;
            return true;
        }
        p.facing = if dx > 0.0 { 1.0 } else { -1.0 };
        p.vx = p.facing * speed.min(dx.abs() * 8.0);
        false
    }

    pub fn update(&mut self, p: &mut Pet, dt: f64) {
        let s = self.s;
        let ground = self.ground;
        p.age += dt;
        p.step_time += dt;
        p.timer -= dt;
        p.fx -= dt;
        p.cooldown -= dt;
        update_needs(p, dt);
        if let Some(left) = p.back_to_sleep {
            p.back_to_sleep = Some(left - dt);
            if left - dt <= 0.0 && !matches!(p.activity, Dragged | Falling | Hop | VisitCursor | Hide | Remind) {
                p.back_to_sleep = None;
                self.start(p, Sleep, false);
                p.say("*yawn* Back to bed… 💤", 2.0);
            }
        }
        if p.bubble.is_some() {
            p.bubble_ttl -= dt;
            if p.bubble_ttl <= 0.0 { p.bubble = None }
        }

        if p.climbing {
            if self.leap(p, dt) { p.climbing = false }
        } else { match p.activity {
            Idle => {
                p.vx = 0.0;
                if p.age > 1.5 && chance(dt * 0.25) { p.facing *= -1.0 }
            }
            Walk => {
                let target = p.target_x.unwrap_or_else(|| self.random_x_for(p));
                p.target_x = Some(target);
                if self.move_to(p, target, 45.0 * s) {
                    // At the end of a walk, sometimes carry on at another height.
                    if chance(0.35) && p.supports(Hop) { self.start(p, Hop, false) } else { p.timer = 0.0 }
                }
            }
            Sniff => {
                let target = p.target_x.unwrap_or_else(|| self.clamp_x(p, p.x + rnd(-150.0, 150.0), 30.0 * s));
                p.target_x = Some(target);
                if self.move_to(p, target, 16.0 * s) { p.target_x = None }
                if p.fx <= 0.0 {
                    p.fx = 1.6;
                    self.emit("?", self.head_point(p), 11.0);
                }
            }
            Run => {
                let r = self.range(p);
                let target = p.target_x.unwrap_or_else(|| {
                    if p.step % 2 == 0 { r.hi - rnd(30.0, 90.0) * s } else { r.lo + rnd(30.0, 90.0) * s }
                });
                p.target_x = Some(target);
                if self.move_to(p, target, 200.0 * s) {
                    p.step += 1;
                    p.target_x = None;
                    if p.step >= fastrand::i32(3..=5) { p.timer = 0.0 }
                }
                if p.fx <= 0.0 && p.vx != 0.0 {
                    p.fx = 0.12;
                    let at = pt(p.x - p.facing * p.size.width / 2.0, ground + p.y + 8.0 * s);
                    self.emit_with("💨", at, 10.0, pt(-p.facing * 20.0, 10.0), 1.4);
                }
            }
            Sit | Lounge | Groom | Stretch | Wave | Meow => p.vx = 0.0,
            Knead => {
                p.vx = 0.0;
                if p.fx <= 0.0 {
                    p.fx = 0.9;
                    self.emit_with("♥", self.head_point(p), 10.0, pt(rnd(-10.0, 10.0), 22.0), 1.4);
                }
            }
            Sleep if p.step == HOP_DOWN => self.hop_down(p, dt),
            Sleep if p.step == BED_WALK => {
                let bed = p.target_x.unwrap_or(p.x);
                // Down past the Dock already, it walks straight to bed; on the Dock, it hops off the end.
                let hop = self.corner_floor(p, bed) < 0.0 && p.floor >= 0.0;
                let stop = if hop { self.dock_edge(p, bed) } else { bed };
                if p.on_ground() && self.move_to(p, stop, 60.0 * s) {
                    if hop { self.plan_bed_hop(p) } else { self.fall_asleep(p) }
                }
            }
            Sleep if p.step == BED_HOP => {
                if self.leap(p, dt) {
                    (p.floor, p.y, p.span) = (p.leap_to.y, p.leap_to.y, None);
                    self.fall_asleep(p);
                }
            }
            Sleep => {
                p.vx = 0.0;
                if p.fx <= 0.0 {
                    p.fx = 1.3;
                    let at = self.head_point(p) + pt(8.0 * s * p.facing, 8.0 * s);
                    self.emit_with("z", at, 12.0, pt(p.facing * 8.0, 18.0), 2.4);
                }
                if p.energy >= 100.0 && !p.commanded && !self.sleeps_through(p) { p.timer = p.timer.min(0.0) }
            }
            Jump => {
                if p.age > 0.15 && p.on_ground() { p.timer = 0.0 }
            }
            Pounce => {
                if p.step == 0 {
                    p.vx = 0.0;
                    if p.step_time > 1.3 {
                        p.step = 1;
                        p.step_time = 0.0;
                        p.vy = 300.0 * s;
                        p.vx = if p.can_move() { p.facing * 200.0 * s } else { 0.0 };
                    }
                } else if p.step_time > 0.15 && p.on_ground() {
                    p.vx = 0.0;
                    if chance(0.5) { self.emit("✨", pt(p.x + p.facing * p.size.width / 2.0, ground + p.y + 14.0 * s), 13.0) }
                    p.timer = 0.0;
                }
            }
            Eat => self.eat(p, dt),
            PlayBall => self.play_ball(p),
            ChaseButterfly => self.chase_butterfly(p, dt),
            ChaseCursor => {
                let above = self.mouse.y - ground - p.floor;
                if above > 260.0 * s || above < -ground { p.timer = p.timer.min(0.0) }
                let dx = self.clamp_x(p, self.mouse.x, 20.0 * s) - p.x;
                if dx.abs() > 120.0 {
                    self.move_to(p, p.x + dx, 170.0 * s);
                } else if dx.abs() > 25.0 {
                    self.move_to(p, p.x + dx, 60.0 * s);
                } else {
                    p.vx = 0.0;
                    p.facing = if dx >= 0.0 { 1.0 } else { -1.0 };
                }
            }
            VisitCursor => self.visit_cursor(p, dt),
            Dance => {
                p.vx = 0.0;
                if p.fx <= 0.0 {
                    p.fx = 0.5;
                    let at = self.head_point(p) + pt(rnd(-14.0, 14.0) * s, 10.0 * s);
                    self.emit_with(pick(&["♪", "♫", "🎵"]), at, 13.0, pt(rnd(-15.0, 15.0), 25.0), 1.4);
                }
            }
            Peekaboo => self.peekaboo(p, dt),
            Hide => self.hide(p, dt),
            Remind => self.remind_step(p, dt),
            PlayFriend => self.play_friend(p),
            Petted => {
                p.vx = 0.0;
                if p.fx <= 0.0 {
                    p.fx = 0.35;
                    let at = self.head_point(p) + pt(rnd(-10.0, 10.0) * s, 6.0 * s);
                    self.emit_with("❤️", at, 12.0, pt(rnd(-12.0, 12.0), 30.0), 1.4);
                }
            }
            Hop => self.hop(p, dt),
            Dragged => {}
            Falling => {
                if p.age > 0.05 && p.on_ground() {
                    p.vx = 0.0;
                    if chance(0.5) { p.say(pick(&["!", "Wheee!", "Again!", "Hmph."]), 1.6) }
                    self.start(p, Idle, false);
                }
            }
        } }

        self.apply_physics(p, dt);
        if p.timer <= 0.0 && p.activity != Dragged && !p.climbing { self.finish(p) }
    }

    // Pieces of update

    fn apply_physics(&self, p: &mut Pet, dt: f64) {
        if p.leaping || matches!(p.activity, Dragged | VisitCursor | Hide) { return }
        if p.activity == Remind && p.step == 2 { return } // standing in mid-screen
        if p.y > p.floor || p.vy != 0.0 {
            p.vy -= 1200.0 * self.s * dt;
            p.y += p.vy * dt;
            if p.y <= p.floor {
                p.y = p.floor;
                p.vy = 0.0;
            }
        }
        p.x += p.vx * dt;
        if p.activity == Falling || !p.on_ground() { p.vx *= 0.4f64.powf(dt) }
        let home = self.clamp_x(p, p.x, p.size.width / 2.0);
        // A pet hopping down from up the screen drifts back over the Dock instead of snapping.
        p.x = if p.activity == Falling { lerp(p.x, home, (dt * 4.0).min(1.0)) } else { home };
        self.follow_ground(p);
    }

    /// For pets at the bottom: stepping off the end of the Dock drops them to the screen's bottom edge,
    /// and walking into the Dock from beside it makes them hop up onto it.
    fn follow_ground(&self, p: &mut Pet) {
        if p.span.is_some() || p.floor > 0.0 || self.below <= 0.0 { return }
        let ground = self.base_floor(p.x);
        if ground < p.floor {
            p.floor = ground;
        } else if ground > p.floor && p.on_ground() {
            let half = p.size.width / 2.0;
            // It has just stepped past one end of the Dock: from the left end it climbs on rightwards.
            let side = if p.x - self.dock_start < self.dock_end - p.x { 1.0 } else { -1.0 };
            p.climbing = true;
            p.leaping = true;
            p.leap_from = pt(p.x, p.y);
            p.leap_to = pt(self.dock_edge(p, p.x + side * half), 0.0);
            p.leap_time = 0.0;
            (p.vx, p.vy) = (0.0, 0.0);
            p.facing = side;
        }
    }

    fn eat(&mut self, p: &mut Pet, dt: f64) {
        let s = self.s;
        let Some(bi) = self.nearest_bowl(p) else { p.timer = 0.0; return };
        let bx = self.bowls[bi].x;
        let side = if p.x < bx { -1.0 } else { 1.0 };
        // Stand so the mouth ends up over the food, just behind the bowl's front rim.
        let reach = p.skin.as_ref().and_then(|k| k.clips.get("eat")).and_then(|c| c.reach);
        let stand = reach.map_or(p.size.width / 2.0 + 8.0 * s, |r| r * p.size.width + 3.0 * s);
        if p.step == 0 {
            if self.move_to(p, bx + side * stand, 80.0 * s) {
                p.step = 1;
                p.facing = -side;
            }
        } else {
            p.vx = 0.0;
            p.facing = -side;
            self.bowls[bi].food -= dt / 6.0;
            p.hunger = (p.hunger - 14.0 * dt).max(0.0);
            p.happiness = (p.happiness + dt).min(100.0);
            if self.bowls[bi].food <= 0.0 {
                p.say(p.species.line("Yum! 😋", "Delicious!! 😋"), 2.6);
                p.timer = 0.0;
            }
        }
    }

    fn play_ball(&mut self, p: &mut Pet) {
        let s = self.s;
        let Some(bi) = self.nearest_ball(p) else { p.timer = 0.0; return };
        let (bx, by) = (self.balls[bi].x, self.balls[bi].y);
        let dx = bx - p.x;
        let reach = p.reach + 6.0 * s;
        if dx.abs() < reach && by < 18.0 * s && p.cooldown <= 0.0 {
            self.balls[bi].vx = p.facing * rnd(180.0, 380.0) * s;
            self.balls[bi].vy = rnd(120.0, 320.0) * s;
            p.cooldown = 0.6;
            p.happiness = (p.happiness + 3.0).min(100.0);
            if chance(0.3) && p.on_ground() { p.vy = 260.0 * s }
        } else if dx.abs() < reach + 20.0 * s && by > 30.0 * s && by < 100.0 * s && p.on_ground() && p.cooldown <= 0.0 {
            p.vy = 340.0 * s;
            p.cooldown = 0.8;
        } else {
            self.move_to(p, bx - (if dx > 0.0 { 1.0 } else { -1.0 }) * reach * 0.6, 170.0 * s);
        }
    }

    fn chase_butterfly(&mut self, p: &mut Pet, dt: f64) {
        let s = self.s;
        let Some(bi) = self.nearest_butterfly(p) else { p.timer = 0.0; return };
        let (bx, by) = (self.butterflies[bi].x, self.butterflies[bi].y);
        // Keep the paws under the butterfly; a dead zone stops it flicking between running and standing.
        let spot = bx - (if bx > p.x { 1.0 } else { -1.0 }) * p.reach * 0.7;
        if (spot - p.x).abs() > (if p.vx == 0.0 { 30.0 } else { 6.0 }) * s {
            self.move_to(p, spot, 120.0 * s);
        } else {
            p.vx = 0.0;
            p.facing = if bx > p.x { 1.0 } else { -1.0 };
        }
        if (spot - p.x).abs() < 40.0 * s && by < 120.0 * s && p.on_ground() && chance(dt * 2.0) {
            p.vy = 400.0 * s;
            self.butterflies[bi].flee = 1.0;
        }
    }

    fn play_friend(&mut self, p: &mut Pet) {
        let s = self.s;
        let friend = (0..self.pets.len())
            .filter(|&j| self.pets[j].id != 0 && self.pets[j].id != p.id)
            .min_by(|&a, &b| (self.pets[a].x - p.x).abs().total_cmp(&(self.pets[b].x - p.x).abs()));
        let Some(j) = friend else { p.timer = 0.0; return };
        let (fx, fw) = (self.pets[j].x, self.pets[j].size.width);
        let side = if fx > p.x { 1.0 } else { -1.0 };
        if self.move_to(p, fx - side * (fw / 2.0 + p.size.width / 2.0), 150.0 * s) {
            self.emit("💕", self.head_point(p), 14.0);
            if !matches!(self.pets[j].activity, Dragged | Sleep | Falling) {
                self.with_pet(j, |w, f| w.start(f, Jump, false));
            }
            self.start(p, Jump, false);
            p.say(p.species.line("Tag! You're it!", "Play with me!!"), 2.6);
        }
    }

    fn visit_cursor(&mut self, p: &mut Pet, dt: f64) {
        let s = self.s;
        let goal = self.perch_point(p);
        if p.step == 0 {
            // Leap up from the Dock to the cursor in an arc.
            let dist = (p.leap_to.x - p.leap_from.x).hypot(p.leap_to.y - p.leap_from.y);
            let dur = clamp(dist / 700.0, 0.5, 1.3);
            p.leap_time += dt;
            p.leap_to = goal;
            let k = (p.leap_time / dur).min(1.0);
            let e = k * k * (3.0 - 2.0 * k);
            p.facing = if p.leap_to.x >= p.leap_from.x { 1.0 } else { -1.0 };
            p.x = lerp(p.leap_from.x, p.leap_to.x, e);
            p.y = lerp(p.leap_from.y, p.leap_to.y, e) + (k * std::f64::consts::PI).sin() * (dist * 0.3).min(120.0) * s;
            if k >= 1.0 {
                p.step = 1;
                p.say(p.species.line("Found you! ♥", "Hi hi hi! ♥"), 2.0);
            }
        } else {
            // Sitting beside the idle cursor, keeping it company.
            p.facing = if self.mouse.x >= p.x { 1.0 } else { -1.0 };
            if self.mouse_still > 30.0 && p.fx <= 0.0 {
                p.fx = 1.5;
                self.emit_with("z", self.head_point(p), 11.0, pt(8.0, 16.0), 1.4);
            }
        }
        // When the user is back (or the visit runs long), the pet stays where it is and carries on there.
        let moved = (self.mouse.x - self.visit_anchor.x).hypot(self.mouse.y - self.visit_anchor.y) > VISIT_FOLLOW_DISTANCE;
        if p.step == 1 && (p.timer <= 0.0 || moved) {
            self.settle(p);
            self.start(p, if p.supports(Sit) { Sit } else { Idle }, false);
            p.say(p.species.line("I like it here! 🐾", "Nice spot! 🐾"), 2.0);
        }
    }

    /// Jumps in an arc to leap_to, then makes that height home and walks on from there.
    fn hop(&mut self, p: &mut Pet, dt: f64) {
        if self.leap(p, dt) {
            self.start(p, if chance(0.6) && p.supports(Walk) { Walk } else { Idle }, false);
        }
    }

    /// Moves along the arc from leap_from to leap_to; on landing, makes that height home and returns true.
    pub(super) fn leap(&mut self, p: &mut Pet, dt: f64) -> bool {
        let s = self.s;
        let dist = (p.leap_to.x - p.leap_from.x).hypot(p.leap_to.y - p.leap_from.y);
        let dur = clamp(dist / (450.0 * s), 0.45, 1.0);
        p.leap_time += dt;
        let k = (p.leap_time / dur).min(1.0);
        let e = k * k * (3.0 - 2.0 * k);
        let rise = (p.leap_to.y - p.leap_from.y).max(0.0);
        p.x = lerp(p.leap_from.x, p.leap_to.x, e);
        p.y = lerp(p.leap_from.y, p.leap_to.y, e) + (k * std::f64::consts::PI).sin() * (25.0 * s + rise * 0.35);
        p.leap_k = k;
        if k < 1.0 { return false }
        p.y = p.leap_to.y;
        p.leaping = false;
        if p.y > 0.0 { self.settle(p) } else { self.go_home(p, false) }
        true
    }

    /// Picks the next step down towards the bed: one level lower, and over towards the corner.
    fn plan_hop_down(&self, p: &mut Pet) {
        let s = self.s;
        let bed = p.target_x.unwrap_or(p.x);
        let dx = (bed - p.x).signum() * (bed - p.x).abs().min(rnd(40.0, 120.0) * s);
        let half = p.size.width / 2.0;
        let x = clamp(p.x + dx, half, self.width - half);
        let mut y = p.y - rnd(70.0, 150.0) * s;
        if y < 30.0 * s { y = self.base_floor(x) }
        p.leap_from = pt(p.x, p.y);
        p.leap_to = pt(x, y);
        p.leap_time = 0.0;
        p.leaping = true;
        p.vx = 0.0;
        p.vy = 0.0;
        p.facing = if p.leap_to.x >= p.x { 1.0 } else { -1.0 };
    }

    /// The last hop of bedtime: off the end of the Dock, down into the corner.
    fn plan_bed_hop(&self, p: &mut Pet) {
        let bed = p.target_x.unwrap_or(p.x);
        p.step = BED_HOP;
        p.leap_from = pt(p.x, p.y);
        p.leap_to = pt(bed, self.corner_floor(p, bed));
        p.leap_time = 0.0;
        p.leaping = true;
        (p.vx, p.vy) = (0.0, 0.0);
        p.facing = if bed >= p.x { 1.0 } else { -1.0 };
    }

    fn fall_asleep(&self, p: &mut Pet) {
        p.step = ASLEEP;
        p.timer = sleep_duration(p);
        p.vx = 0.0;
    }

    /// Bedtime for a pet up the screen: hop down a level at a time, then walk to the bed on the Dock.
    fn hop_down(&mut self, p: &mut Pet, dt: f64) {
        if !self.leap(p, dt) { return }
        if p.floor > 0.0 { self.plan_hop_down(p) } else { p.step = BED_WALK }
    }

    fn peekaboo(&mut self, p: &mut Pet, dt: f64) {
        p.vx = 0.0;
        match p.step {
            0 => {
                p.sink = (p.sink + dt / 0.6).min(1.0);
                if p.sink >= 1.0 {
                    p.step = 1;
                    p.step_time = 0.0;
                }
            }
            1 => {
                if p.step_time > rnd(3.5, 3.7) {
                    p.x = self.random_x_for(p);
                    p.facing = if chance(0.5) { 1.0 } else { -1.0 };
                    p.step = 2;
                    p.step_time = 0.0;
                }
            }
            2 => {
                p.sink = (p.sink - dt / 0.5).max(0.55);
                if p.step_time > 1.6 { p.step = 3 }
            }
            _ => {
                p.sink = (p.sink - dt / 0.3).max(0.0);
                if p.sink <= 0.0 {
                    p.say("Peekaboo! 🙈", 2.6);
                    self.start(p, if p.supports(Wave) { Wave } else { Idle }, false);
                    p.timer = 2.0;
                }
            }
        }
    }
}

fn update_needs(p: &mut Pet, dt: f64) {
    p.hunger = (p.hunger + 0.014 * dt).min(100.0);
    if p.activity == Sleep && p.step == ASLEEP {
        p.energy = (p.energy + 1.5 * dt).min(100.0);
    } else {
        let drain = if matches!(p.activity, Run | PlayBall) { 0.12 } else { 0.01 };
        p.energy = (p.energy - drain * dt).max(0.0);
    }
    p.happiness = clamp(p.happiness - (if p.hunger > 70.0 { 0.02 } else { 0.006 }) * dt, 0.0, 100.0);
}

/// How long a nap lasts, from when the pet reaches its bed.
fn sleep_duration(p: &Pet) -> f64 {
    if p.commanded { 30.0 } else if p.energy < 30.0 { rnd(40.0, 70.0) } else { rnd(15.0, 30.0) }
}

/// What a pet says when the user picks it up.
pub fn picked_up_line(species: Species) -> &'static str { species.line("Mrrow?!", "Whoa!") }

#[cfg(test)]
#[path = "behavior_tests.rs"]
mod tests;
