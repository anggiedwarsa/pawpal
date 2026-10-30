//! Games two pets play together — tag and a play fight — and each kind of pet's special skill:
//! a kitsune's shapeshifting, a dragon's fire breath and flight, a snake growing into a giant
//! cobra, a corgi herding its friends, a cat vanishing, and a T-rex's earthquake stomp.
//!
//! Both games start the same way: one pet walks up to the nearest friend and invites it, and from
//! then on each pet plays its own side, keeping track of the other through `Pet::buddy`. The game
//! ends for both together (they share the same timer), or as soon as either one is called away.

use std::rc::Rc;

use super::world::World;
use crate::domain::Activity::{self, *};
use crate::domain::{same_level, Pet};
use crate::support::*;

/// Steps of a game with a friend (`Pet::step`).
pub const APPROACH: i32 = 0;
/// Tag: keeping away from whoever is "it".
pub const RUN_AWAY: i32 = 1;
/// Tag: "it", chasing the other one.
pub const CHASE: i32 = 2;
/// Play fight: facing each other, waiting for an opening.
pub const SQUARE_OFF: i32 = 3;
/// Play fight: pouncing on the other one.
pub const LUNGE: i32 = 4;
/// Play fight: playfully bowled over, tumbling back.
pub const BOUNCED: i32 = 5;

/// How long a game with a friend lasts.
pub const GAME_TIME: f64 = 14.0;
/// How long a kitsune powers up before it shapeshifts.
pub const CHARGE_TIME: f64 = 2.4;
/// How long a dragon breathes in before the flames come out, and the whole fire breath; matched
/// to the dragon's 24-frame clip at 8 fps (6 frames breathing in, 15 of flame, 3 of smoke).
pub const INHALE_TIME: f64 = 0.75;
pub const FIRE_TIME: f64 = 3.0;
/// How far ahead a dragon's flames reach other pets (points, at scale 1).
pub const FIRE_REACH: f64 = 160.0;

/// Steps of a flight (`Pet::step`).
pub const TAKE_OFF: i32 = 0;
pub const CRUISE: i32 = 1;
pub const LANDING: i32 = 2;
/// How long before a flight is over the pet starts coming down.
pub const LAND_TIME: f64 = 2.5;

/// Growing giant: how long it lasts, how long growing and shrinking back take.
pub const GROW_TIME: f64 = 9.0;
const GROW_IN: f64 = 1.2;
const SHRINK: f64 = 1.0;
/// How long a corgi herds, and how far either side of the flock it runs.
pub const HERD_TIME: f64 = 9.0;
/// A herded pet's step: hurrying to its place in the flock, then waiting there.
pub const FLOCK: i32 = 10;
const HERD_SWEEP: f64 = 150.0;
/// A T-rex's stomp: two quakes, timed to the clip's foot coming down (frame 6 of 12 at 8 fps, twice).
pub const STOMP_TIME: f64 = 3.0;
pub const QUAKES: [f64; 2] = [0.75, 2.25];
const QUAKE_REACH: f64 = 500.0;

/// Whether a pet is free to be pulled into something by another pet: not asleep, not being
/// handled, not already in a game, and not in the middle of its own special skill.
fn free(a: Activity) -> bool {
    !a.is_special() && !matches!(a, Dragged | Falling | Sleep | Hide | Remind | VisitCursor | Hop | PlayFriend | PlayFight)
}

impl World {
    /// The nearest pet that's free to play on the same level.
    fn nearest_playmate(&self, p: &Pet) -> Option<usize> {
        (0..self.pets.len())
            .filter(|&j| {
                let f = &self.pets[j];
                f.id != 0 && f.id != p.id && same_level(f.floor, p.floor) && free(f.activity)
            })
            .min_by(|&a, &b| (self.pets[a].x - p.x).abs().total_cmp(&(self.pets[b].x - p.x).abs()))
    }

    /// The friend this pet is playing with, while that friend is still playing the same game with it.
    fn playmate(&self, p: &Pet) -> Option<usize> {
        let j = self.index_of(p.buddy?)?;
        (self.pets[j].activity == p.activity && self.pets[j].buddy == Some(p.id)).then_some(j)
    }

    /// Walks up to the nearest friend; once there, both start `game`, each on its own side.
    /// Returns true on the frame they start.
    fn invite(&mut self, p: &mut Pet, game: Activity, mine: i32, theirs: i32) -> bool {
        let Some(j) = self.nearest_playmate(p) else {
            p.timer = 0.0;
            return false;
        };
        let (fx, fw) = (self.pets[j].x, self.pets[j].size.width);
        let side = if fx > p.x { 1.0 } else { -1.0 };
        if !self.move_to(p, fx - side * (fw / 2.0 + p.size.width / 2.0 + 6.0 * self.s), 150.0 * self.s) { return false }
        let me = p.id;
        let friend = self.with_pet(j, |w, f| {
            w.start(f, game, false);
            (f.buddy, f.step, f.timer) = (Some(me), theirs, GAME_TIME);
            f.facing = if p.x > f.x { 1.0 } else { -1.0 };
            f.id
        });
        (p.buddy, p.step, p.step_time, p.timer) = (Some(friend), mine, 0.0, GAME_TIME);
        p.facing = side;
        true
    }

    /// The game's over: both make up with a happy word and a heart.
    fn game_over(&mut self, p: &mut Pet, key: &str, cat: &str, dog: &str) {
        if p.buddy.take().is_none() { return }
        p.vx = 0.0;
        p.happiness = (p.happiness + 10.0).min(100.0);
        p.say(p.line(key, cat, dog), 2.4);
        self.emit("💖", self.head_point(p), 14.0);
    }

    // Tag

    pub(super) fn play_tag(&mut self, p: &mut Pet) {
        let s = self.s;
        if p.timer <= 0.0 { return self.game_over(p, "tag_over", "That was fun! ♥", "Again! Again! ♥") }
        if p.step == APPROACH {
            if self.invite(p, PlayFriend, RUN_AWAY, CHASE) {
                self.emit("💕", self.head_point(p), 14.0);
                p.say(p.line("tag", "Tag! You're it!", "Play with me!!"), 2.6);
                p.vy = 220.0 * s;
            }
            return;
        }
        let Some(j) = self.playmate(p) else {
            p.timer = 0.0;
            return;
        };
        let fx = self.pets[j].x;
        let gap = (fx - p.x).abs() - (self.pets[j].size.width + p.size.width) / 2.0;
        match p.step {
            RUN_AWAY => {
                let away = if p.x >= fx { 1.0 } else { -1.0 };
                let target = self.clamp_x(p, p.x + away * 200.0 * s, p.size.width / 2.0);
                if (target - p.x).abs() < 8.0 {
                    // Cornered at the end of the Dock: turn and face whoever's "it".
                    p.vx = 0.0;
                    p.facing = -away;
                } else {
                    self.move_to(p, target, 140.0 * s);
                }
            }
            CHASE => {
                // "It" counts to three first, giving the other a head start.
                if p.step_time < 0.7 {
                    p.vx = 0.0;
                    p.facing = if fx > p.x { 1.0 } else { -1.0 };
                    return;
                }
                self.move_to(p, fx, 185.0 * s);
                if gap < 6.0 * s {
                    let at = pt((p.x + fx) / 2.0, self.ground + p.y + p.size.height * 0.6);
                    self.emit("✨", at, 15.0);
                    p.say(pick(&["Tag!", "Gotcha!", "You're it!"]), 1.6);
                    (p.step, p.step_time) = (RUN_AWAY, 0.0);
                    if p.on_ground() { p.vy = 200.0 * s }
                    self.with_pet(j, |w, f| {
                        (f.step, f.step_time) = (CHASE, 0.0);
                        if f.on_ground() { f.vy = 260.0 * w.s }
                        if chance(0.4) { f.say(pick(&["Eep!", "No fair!", "Hehe!"]), 1.4) }
                    });
                }
            }
            _ => {}
        }
    }

    // Play fight

    pub(super) fn play_fight(&mut self, p: &mut Pet) {
        let s = self.s;
        if p.timer <= 0.0 { return self.game_over(p, "fight_over", "Good match! 🤝", "Hehe, you win~ ♥") }
        if p.step == APPROACH {
            if self.invite(p, PlayFight, SQUARE_OFF, SQUARE_OFF) {
                p.say(p.line_or("taunt", p.species.taunt()), 2.0);
                self.emit("💢", self.head_point(p), 14.0);
                p.cooldown = 0.6;
                let first = p.buddy;
                if let Some(j) = first.and_then(|id| self.index_of(id)) { self.pets[j].cooldown = rnd(1.4, 2.0) }
            }
            return;
        }
        let Some(j) = self.playmate(p) else {
            p.timer = 0.0;
            return;
        };
        let (fx, fw, f_ready) = (self.pets[j].x, self.pets[j].size.width, self.pets[j].step == SQUARE_OFF && self.pets[j].on_ground());
        let toward = if fx > p.x { 1.0 } else { -1.0 };
        let gap = (fx - p.x).abs() - (fw + p.size.width) / 2.0;
        match p.step {
            SQUARE_OFF if p.on_ground() => {
                p.facing = toward;
                if gap > 40.0 * s {
                    self.move_to(p, fx - toward * (fw / 2.0 + p.size.width / 2.0 + 14.0 * s), 110.0 * s);
                    p.facing = toward;
                } else {
                    p.vx = 0.0;
                }
                if p.cooldown <= 0.0 && f_ready && gap <= 60.0 * s {
                    // Pounce!
                    (p.step, p.step_time) = (LUNGE, 0.0);
                    p.vy = 300.0 * s;
                    p.vx = toward * clamp((gap.max(0.0) + 14.0 * s) / 0.4, 60.0 * s, 300.0 * s);
                    if chance(0.45) { p.say(pick(&["Rawr!", "Hyah!", "Take that!", "Pounce!"]), 1.2) }
                }
            }
            LUNGE if p.step_time > 0.1 && p.on_ground() => {
                p.vx = 0.0;
                let mid = pt((p.x + fx) / 2.0, self.ground + p.y + p.size.height * 0.55);
                self.emit("💥", mid, 18.0);
                if chance(0.35) {
                    // A cartoon scuffle cloud.
                    for k in 0..6 {
                        let a = k as f64 * std::f64::consts::PI / 3.0 + rnd(-0.3, 0.3);
                        self.emit_with(pick(&["💨", "⭐", "💢"]), mid, 14.0, pt(a.cos() * 60.0 * s, a.sin() * 50.0 * s + 20.0 * s), 0.9);
                    }
                    p.say("*tussle tussle*", 1.4);
                }
                let me_x = p.x;
                self.with_pet(j, |w, f| {
                    // Playfully bowled over: tumbles back, then wants a rematch.
                    let away = if f.x >= me_x { 1.0 } else { -1.0 };
                    (f.step, f.step_time) = (BOUNCED, 0.0);
                    f.vy = 220.0 * w.s;
                    f.vx = away * 140.0 * w.s;
                    if chance(0.35) { f.say(pick(&["Oof!", "Hey!", "Hehe!", "Not fair~"]), 1.4) }
                });
                (p.step, p.step_time) = (SQUARE_OFF, 0.0);
                p.cooldown = rnd(1.4, 2.4);
            }
            BOUNCED if p.step_time > 0.1 && p.on_ground() => {
                p.vx = 0.0;
                (p.step, p.step_time) = (SQUARE_OFF, 0.0);
                p.cooldown = rnd(0.5, 1.1);
            }
            _ => {}
        }
    }

    // Shapeshifting

    pub(super) fn transform(&mut self, p: &mut Pet) {
        let s = self.s;
        p.vx = 0.0;
        if p.step != 0 { return }
        if p.fx <= 0.0 {
            p.fx = 0.18;
            let at = pt(p.x + rnd(-0.4, 0.4) * p.size.width, self.ground + p.y + rnd(0.1, 0.9) * p.size.height);
            self.emit_with(pick(&["🔥", "✨"]), at, 12.0, pt(rnd(-10.0, 10.0) * s, 45.0 * s), 1.0);
        }
        if p.step_time < CHARGE_TIME { return }
        (p.step, p.step_time) = (1, 0.0);
        self.poof(p);
        // Turn into one of the other pets, if there are any.
        let others: Vec<_> = self.pets.iter()
            .filter(|o| o.id != 0 && o.id != p.id)
            .filter_map(|o| o.skin.clone())
            .filter(|k| !p.skin.as_ref().is_some_and(|own| Rc::ptr_eq(own, k)))
            .collect();
        if others.is_empty() {
            p.say("Behold my true power! 🔥", 2.4);
        } else {
            p.disguise = Some(others[fastrand::usize(..others.len())].clone());
            p.disguise_ttl = rnd(14.0, 22.0);
            p.say("Guess who? 😏", 2.4);
        }
        p.timer = 0.8;
    }

    /// Drops a disguise in a puff of smoke.
    pub(super) fn reveal(&mut self, p: &mut Pet, line: &'static str) {
        if p.disguise.take().is_none() { return }
        self.poof(p);
        p.say(line, 2.6);
    }

    // Fire breath

    pub(super) fn breathe_fire(&mut self, p: &mut Pet) {
        let s = self.s;
        p.vx = 0.0;
        if p.step_time < INHALE_TIME { return }
        let mouth = pt(p.x + p.facing * p.size.width * 0.48, self.ground + p.y + p.size.height * 0.72);
        if p.step == 0 {
            p.step = 1;
            p.say(pick(&["FWOOOSH! 🔥", "Rawr~ 🔥🔥", "Toasty!"]), 1.8);
            self.startle(p, FIRE_REACH, true, &["Hot hot hot! 🥵", "Yikes! 🔥", "My fur!! 😱"]);
        }
        if p.fx > 0.0 { return }
        if p.timer < 0.4 {
            // The flames die down to a wisp of smoke.
            p.fx = 0.15;
            self.emit_with("💨", mouth, 13.0, pt(p.facing * 40.0 * s, 30.0 * s), 0.8);
        } else {
            p.fx = 0.05;
            let v = pt(p.facing * rnd(200.0, 280.0) * s, rnd(10.0, 50.0) * s);
            self.emit_with("🔥", mouth, rnd(12.0, 20.0), v, rnd(0.5, 0.8));
        }
    }

    /// Other pets on this level within `reach` (only those ahead of it, if `ahead_only`) jump with a
    /// yelp — all in fun, nobody gets hurt. Sleeping pets and ones busy elsewhere aren't bothered.
    fn startle(&mut self, p: &Pet, reach: f64, ahead_only: bool, lines: &[&'static str]) {
        let reach = reach * self.s;
        let near: Vec<usize> = (0..self.pets.len())
            .filter(|&j| {
                let f = &self.pets[j];
                let ahead = (f.x - p.x) * p.facing;
                let close = if ahead_only { ahead > 0.0 && ahead < reach } else { (f.x - p.x).abs() < reach };
                f.id != 0 && f.id != p.id && same_level(f.floor, p.floor) && close && free(f.activity)
            })
            .collect();
        for j in near {
            self.with_pet(j, |w, f| {
                f.say(pick(lines), 1.8);
                if f.on_ground() { f.vy = 260.0 * w.s }
            });
        }
    }

    // Growing giant (snakes)

    pub(super) fn grow(&mut self, p: &mut Pet, dt: f64) {
        p.vx = 0.0;
        if p.timer < SHRINK {
            p.bigness = (p.bigness - dt / SHRINK).max(0.0);
            if p.step == 1 {
                p.step = 2;
                p.say("…hehe, just kidding~ 🐍", 2.0);
            }
            return;
        }
        p.bigness = (p.bigness + dt / GROW_IN).min(1.0);
        if p.step == 0 && p.bigness >= 1.0 {
            p.step = 1;
            p.say("HSSSSSS!", 1.6);
            self.startle(p, 260.0, false, &["A c-cobra!! 😱", "Eeek! 🙀", "So big!!"]);
        } else if p.step == 1 && p.fx <= 0.0 && chance(0.02) {
            p.fx = 2.0;
            p.say(pick(&["Hssss~", "*sways menacingly*", "Fear me~ 🐍"]), 1.6);
        }
    }

    // Herding (corgis)

    pub(super) fn herd(&mut self, p: &mut Pet) {
        let s = self.s;
        if p.step == FLOCK {
            // One of the flock: hurry to its spot, then wait there watching the herder.
            let Some(leader) = p.buddy.and_then(|id| self.index_of(id)).filter(|&j| self.pets[j].activity == Herd) else {
                p.timer = 0.0;
                return;
            };
            let spot = p.target_x.unwrap_or(p.x);
            if self.move_to(p, spot, 130.0 * s) {
                p.vx = 0.0;
                p.facing = if self.pets[leader].x > p.x { 1.0 } else { -1.0 };
            }
            return;
        }
        if p.step == 0 {
            // Round up everyone free on this level to gather beside the herder.
            let flock: Vec<usize> = (0..self.pets.len())
                .filter(|&j| {
                    let f = &self.pets[j];
                    f.id != 0 && f.id != p.id && same_level(f.floor, p.floor) && f.can_move() && free(f.activity)
                })
                .collect();
            if flock.is_empty() {
                p.say("No one to herd… 🥺", 2.0);
                p.timer = 0.0;
                return;
            }
            p.rally = self.clamp_x(p, p.x, p.size.width / 2.0);
            let side = if p.rally > (self.min_x + self.max_x) / 2.0 { -1.0 } else { 1.0 };
            let mut offset = p.size.width * 0.8;
            for j in flock {
                let w = self.pets[j].size.width;
                let spot = p.rally + side * (offset + w / 2.0);
                offset += w * 0.9;
                let me = p.id;
                self.with_pet(j, |w, f| {
                    w.start(f, Herd, false);
                    (f.step, f.buddy, f.timer) = (FLOCK, Some(me), HERD_TIME);
                    f.target_x = Some(w.clamp_x(f, spot, f.size.width / 2.0));
                    f.bubble = None;
                    if chance(0.5) { f.say(pick(&["Okay okay! 🐑", "Coming~", "Yes, boss!"]), 1.6) }
                });
            }
            (p.step, p.target_x) = (1, Some(p.rally + side * HERD_SWEEP * s));
            return;
        }
        if p.step == 1 {
            // Dash back and forth around the flock, barking.
            let goal = p.target_x.unwrap_or(p.rally);
            if self.move_to(p, goal, 210.0 * s) {
                let back = if goal > p.rally { -1.0 } else { 1.0 };
                p.target_x = Some(self.clamp_x(p, p.rally + back * HERD_SWEEP * s, p.size.width / 2.0));
            }
            if p.fx <= 0.0 {
                p.fx = 1.3;
                p.say(p.one_of("sounds", p.species.sounds()), 1.0);
            }
            if p.timer < 2.0 {
                p.step = 2;
                p.say("All here! 😤", 2.0);
            }
            return;
        }
        // Done: trot back to the flock and stand guard.
        if self.move_to(p, p.rally, 120.0 * s) { p.vx = 0.0 }
    }

    // Vanishing (cats)

    pub(super) fn vanish(&mut self, p: &mut Pet, dt: f64) {
        let s = self.s;
        p.vx = 0.0;
        let at = pt(p.x, self.ground + p.y + p.size.height * 0.5);
        match p.step {
            0 => {
                p.sink = (p.sink + dt / 0.5).min(1.0);
                if p.sink >= 1.0 {
                    (p.step, p.step_time) = (1, 0.0);
                    self.sparkle(at);
                }
            }
            1 => {
                if p.step_time > 0.7 {
                    // Pop up well away from where it went down.
                    let from = p.x;
                    let mut x = self.random_x_for(p);
                    for _ in 0..6 {
                        if (x - from).abs() > 150.0 * s { break }
                        x = self.random_x_for(p);
                    }
                    p.x = x;
                    p.facing = if chance(0.5) { 1.0 } else { -1.0 };
                    (p.step, p.step_time) = (2, 0.0);
                    self.sparkle(pt(p.x, self.ground + p.y + p.size.height * 0.5));
                }
            }
            2 => {
                p.sink = (p.sink - dt / 0.4).max(0.0);
                if p.sink <= 0.0 {
                    p.step = 3;
                    p.say(pick(&["…ta-da! 😼", "Miss me? ✨", "Over here~ 😼"]), 2.0);
                    p.timer = p.timer.min(1.2);
                }
            }
            _ => {}
        }
    }

    fn sparkle(&mut self, at: Point) {
        let s = self.s;
        for k in 0..6 {
            let a = k as f64 * std::f64::consts::PI / 3.0;
            self.emit_with("✨", at, 13.0, pt(a.cos() * 50.0 * s, a.sin() * 40.0 * s + 20.0 * s), 0.9);
        }
    }

    // Earthquake stomp (T-rexes)

    pub(super) fn stomp(&mut self, p: &mut Pet) {
        let s = self.s;
        p.vx = 0.0;
        let due = QUAKES.iter().filter(|&&t| p.age >= t).count() as i32;
        if due <= p.step { return }
        p.step = due;
        let feet = pt(p.x + p.facing * p.size.width * 0.1, self.ground + p.y + 4.0 * s);
        self.emit("💥", feet, 20.0);
        for dir in [-1.0, 1.0] {
            self.emit_with("💨", feet, 16.0, pt(dir * 90.0 * s, 15.0 * s), 0.8);
        }
        self.startle(p, QUAKE_REACH, false, &["Earthquake!! 😵", "Whoa whoa!", "Eek! 🙀", "The ground!!"]);
    }

    // Flight

    pub(super) fn take_off(&mut self, p: &mut Pet) {
        let s = self.s;
        p.timer = rnd(10.0, 18.0);
        (p.vx, p.vy) = (0.0, 0.0);
        p.cruise = clamp(p.floor + rnd(90.0, 220.0) * s, p.floor + 40.0 * s, self.flight_ceiling(p));
        p.say(p.one_of("fly", &["Up, up and away! 🐉", "Wheee~", "Look, I can fly!"]), 2.0);
        self.emit_with("💨", pt(p.x, self.ground + p.y + 4.0 * s), 15.0, pt(0.0, 10.0 * s), 0.8);
    }

    /// The highest a flying pet goes: below the top of the screen with room for its bubble.
    fn flight_ceiling(&self, p: &Pet) -> f64 {
        (self.height - self.ground - p.size.height - 40.0 * self.s).max(p.floor + 40.0 * self.s)
    }

    pub(super) fn fly(&mut self, p: &mut Pet, dt: f64) {
        let s = self.s;
        p.vy = 0.0;
        if p.step != LANDING && p.timer < LAND_TIME { p.step = LANDING }
        match p.step {
            TAKE_OFF => {
                p.y = (p.y + 170.0 * s * dt).min(p.cruise);
                p.x = self.clamp_x(p, p.x + p.facing * 50.0 * s * dt, p.size.width / 2.0);
                if p.y >= p.cruise - 1.0 { (p.step, p.step_time) = (CRUISE, 0.0) }
            }
            CRUISE => {
                // Bob gently on the wing while gliding from one spot to the next.
                let goal = p.target_x.unwrap_or_else(|| self.random_x_for(p));
                p.target_x = Some(goal);
                let dx = goal - p.x;
                if dx.abs() < 10.0 * s {
                    p.target_x = None;
                    if chance(0.3) { p.cruise = clamp(p.floor + rnd(90.0, 220.0) * s, p.floor + 40.0 * s, self.flight_ceiling(p)) }
                } else {
                    p.facing = dx.signum();
                    p.x = self.clamp_x(p, p.x + p.facing * (160.0 * s * dt).min(dx.abs()), p.size.width / 2.0);
                }
                let bob = p.cruise + 10.0 * s * (p.age * 2.4).sin();
                p.y += (bob - p.y) * (dt * 3.0).min(1.0);
                if p.fx <= 0.0 && chance(dt * 0.15) {
                    p.fx = 3.0;
                    p.say(pick(&["Wheee~ 🐉", "*flap flap*", "Nice view up here!"]), 1.8);
                }
            }
            _ => {
                // Coming in to land; the flight isn't over until its feet are down.
                p.y = (p.y - (70.0 * s).max((p.y - p.floor) * 1.6) * dt).max(p.floor);
                if p.y > p.floor + 0.5 {
                    p.timer = p.timer.max(0.1);
                } else if p.timer > 0.0 {
                    p.y = p.floor;
                    p.timer = 0.0;
                    self.emit_with("💨", pt(p.x, self.ground + p.y + 4.0 * s), 15.0, pt(0.0, 10.0 * s), 0.8);
                }
            }
        }
    }

    // Skills of a pet's own (pet.json)

    /// Starts one of the pet's own skills: it stays put playing the skill's clip for the skill's
    /// time, with its line, burst and startle as it starts.
    pub(super) fn begin_skill(&mut self, p: &mut Pet, i: u8) {
        p.vx = 0.0;
        let Some(skill) = p.skin.as_ref().and_then(|k| k.pet.skills.get(i as usize)).cloned() else {
            p.timer = 0.0;
            return;
        };
        p.timer = skill.seconds;
        if let Some(line) = skill.say.pick() { p.say(line, 2.2) }
        if let Some(burst) = &skill.burst {
            let s = self.s;
            let c = pt(p.x, self.ground + p.y + p.size.height * 0.5);
            for k in 0..6 {
                let a = k as f64 * std::f64::consts::PI / 3.0;
                self.emit_with(burst, c, 14.0, pt(a.cos() * 55.0 * s, a.sin() * 45.0 * s + 20.0 * s), 0.9);
            }
        }
        if skill.startle { self.startle(p, 220.0, false, &["Whoa! 😲", "Eek! 🙀", "Wow!!"]) }
    }

    fn poof(&mut self, p: &Pet) {
        let s = self.s;
        let c = pt(p.x, self.ground + p.y + p.size.height * 0.45);
        for k in 0..8 {
            let a = k as f64 * std::f64::consts::PI / 4.0;
            self.emit_with("💨", c, 18.0, pt(a.cos() * 70.0 * s, a.sin() * 55.0 * s + 15.0 * s), 0.8);
        }
    }
}

#[cfg(test)]
#[path = "play_tests.rs"]
mod tests;
