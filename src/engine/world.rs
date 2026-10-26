//! The simulation: pets, toys and food on a strip above the Dock. Knows nothing about drawing.
//! Coordinates are points in the stage's space (y up); `ground` is where pets stand.

use crate::data::settings;
use crate::domain::Activity::{self, *};
use crate::domain::{same_level, Ball, Bowl, Butterfly, Particle, Pet};
use crate::support::*;

pub struct World {
    pub pets: Vec<Pet>,
    pub balls: Vec<Ball>,
    pub bowls: Vec<Bowl>,
    pub butterflies: Vec<Butterfly>,
    pub particles: Vec<Particle>,

    pub ground: f64,
    /// How far the screen's bottom edge is below the Dock line; the bottom-right corner past the Dock is that low.
    pub below: f64,
    /// Where the Dock's icons start and end; beside it the ground is `below` lower.
    pub dock_start: f64,
    pub dock_end: f64,
    pub min_x: f64,
    pub max_x: f64,
    pub width: f64,
    pub height: f64,
    /// The pet size setting, read once per step.
    pub s: f64,

    /// Cursor position, fed in every frame by the app.
    pub mouse: Point,
    pub mouse_still: f64,
    pub visited_this_idle: bool,
    pub visit_anchor: Point,
    /// Seconds since the last key press, and the spot being typed at (stage coordinates) while it's recent.
    pub since_key: f64,
    pub typing_zone: Option<Rect>,
    /// False when typing_zone is only a guess (a whole window or row): pets on the Dock are then left alone.
    pub typing_zone_is_exact: bool,
    /// Something is playing full screen: pets wait in the corner.
    pub fullscreen: bool,
}

/// Seconds the cursor must stay still before a pet comes to visit it.
pub const IDLE_CURSOR_DELAY: f64 = 60.0;
/// Longest a pet stays by an idle cursor before heading home on its own.
pub const VISIT_DURATION: f64 = 600.0;
/// How far the cursor must move (points) for a visiting pet to settle where it is.
pub const VISIT_FOLLOW_DISTANCE: f64 = 40.0;
/// Seconds pets keep sitting and watching after the user's last key press.
pub const TYPING_PAUSE: f64 = 10.0;
/// How far a pet living off the Dock roams either side of where it was put.
pub const ROAM: f64 = 160.0;

/// Activities that happen on the Dock (friends): a pet living elsewhere comes down for them.
/// Food and toys don't need it: they appear on the level of the pet they're for.
pub fn on_dock(a: Activity) -> bool { matches!(a, PlayFriend) }
/// Activities typing doesn't interrupt.
fn busy(a: Activity) -> bool { matches!(a, Dragged | Falling | VisitCursor | Hop | Sleep | Eat | Peekaboo | Petted | Hide | Remind) }

impl World {
    pub fn new() -> World {
        World {
            pets: Vec::new(),
            balls: Vec::new(),
            bowls: Vec::new(),
            butterflies: Vec::new(),
            particles: Vec::new(),
            ground: 6.0,
            min_x: 0.0,
            max_x: 800.0,
            width: 800.0,
            height: 600.0,
            s: settings::scale(),
            mouse: Point::default(),
            mouse_still: 0.0,
            visited_this_idle: false,
            visit_anchor: Point::default(),
            since_key: f64::INFINITY,
            typing_zone: None,
            typing_zone_is_exact: false,
            fullscreen: false,
            below: 0.0,
            dock_start: f64::NEG_INFINITY,
            dock_end: f64::INFINITY,
        }
    }

    /// Runs `f` on one pet while it is lifted out of `pets`, so it can use the rest of the world freely.
    /// Its slot holds a placeholder (id 0) meanwhile, which every lookup of other pets skips.
    pub fn with_pet<R>(&mut self, i: usize, f: impl FnOnce(&mut World, &mut Pet) -> R) -> R {
        let mut p = std::mem::take(&mut self.pets[i]);
        let r = f(self, &mut p);
        self.pets[i] = p;
        r
    }

    pub fn index_of(&self, id: u64) -> Option<usize> { self.pets.iter().position(|p| p.id == id && id != 0) }

    /// Runs `f` on the pet with this id, if it still exists.
    pub fn with_id(&mut self, id: u64, f: impl FnOnce(&mut World, &mut Pet)) {
        if let Some(i) = self.index_of(id) { self.with_pet(i, f) }
    }

    /// `since_key` is the time since the last key press; only that is read, never which key.
    pub fn step(&mut self, dt: f64, m: Point, since_key: f64) {
        self.s = settings::scale();
        if (m.x - self.mouse.x).hypot(m.y - self.mouse.y) > 2.0 {
            self.mouse_still = 0.0;
            self.mouse = m;
            self.visited_this_idle = false;
        } else {
            self.mouse_still += dt;
        }
        self.since_key = since_key;
        if since_key < 0.5 {
            self.mouse_still = 0.0;
            self.watch_typing(since_key);
        }
        if settings::visit_idle_cursor() && !self.visited_this_idle && self.mouse_still >= IDLE_CURSOR_DELAY {
            let nearest = (0..self.pets.len())
                .filter(|&i| !matches!(self.pets[i].activity, Dragged | Falling))
                .min_by(|&a, &b| (self.pets[a].x - m.x).abs().total_cmp(&(self.pets[b].x - m.x).abs()));
            if let Some(i) = nearest {
                self.visited_this_idle = true;
                self.with_pet(i, |w, p| w.start(p, VisitCursor, false));
            }
        }
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| w.update(p, dt));
        }
        self.update_props(dt);
    }

    /// How high above the window's bottom edge (points) anything is or is about to be drawn;
    /// infinite when a pet may go anywhere (dragged, visiting the cursor, giving a heads-up).
    pub fn headroom(&self) -> f64 {
        let mut top: f64 = 0.0;
        for p in &self.pets {
            if matches!(p.activity, Dragged | VisitCursor | Remind) || p.floor > 0.0 { return f64::INFINITY }
            let y = if p.leaping { p.y.max(p.leap_to.y).max(p.leap_from.y) } else { p.y };
            let bubble = if p.bubble.is_some() { 60.0 } else { 0.0 };
            top = top.max(self.ground + y + p.size.height + bubble);
        }
        for q in &self.particles { top = top.max(q.p.y + q.size) }
        for b in &self.butterflies { top = top.max(self.ground + b.floor + b.y + 30.0 * self.s) }
        for b in &self.balls { top = top.max(self.ground + b.floor + b.y + 30.0 * self.s) }
        top
    }

    pub fn is_calm(&self) -> bool {
        self.balls.is_empty()
            && self.butterflies.is_empty()
            && self.particles.is_empty()
            && self.pets.iter().all(|p| p.activity.is_calm() && p.vx == 0.0)
    }

    // Geometry

    /// While the user types, pets sit down and watch; they carry on with their day a while after the last key press.
    fn watch_typing(&mut self, since: f64) {
        for i in 0..self.pets.len() {
            self.with_pet(i, |w, p| {
                if p.commanded || busy(p.activity) || !p.supports(Sit) { return }
                // In the way of the text: get out of it first.
                if let Some(zone) = w.typing_zone {
                    if w.pet_rect(p).intersects(&zone) && (p.floor > 0.0 || w.typing_zone_is_exact) {
                        w.dodge(p, zone);
                        return;
                    }
                }
                if p.activity != Sit {
                    w.start(p, Sit, false);
                    p.vx = 0.0;
                }
                p.timer = TYPING_PAUSE - since;
            });
        }
    }

    /// Moves a pet out of the spot being typed at: sideways along its row if there's room, else up over it.
    fn dodge(&mut self, p: &mut Pet, zone: Rect) {
        let r = self.range(p);
        let half = p.size.width / 2.0 + 10.0;
        let mut sides: Vec<f64> = [zone.min_x() - half, zone.max_x() + half].into_iter().filter(|&x| r.contains(x)).collect();
        sides.sort_by(|a, b| (a - p.x).abs().total_cmp(&(b - p.x).abs()));
        if let (Some(&x), true) = (sides.first(), p.can_move()) {
            self.start(p, Walk, false);
            p.target_x = Some(x);
            p.commanded = true; // so typing doesn't sit it back down halfway
            return;
        }
        if !p.supports(Hop) { return }
        let top = self.height - self.ground - p.size.height - 20.0;
        let y = zone.max_y() - self.ground + 10.0;
        self.start(p, Hop, false);
        if y <= top { p.leap_to = pt(p.x, y) } // above the text; otherwise hop_target's pick, which avoids it
        p.facing = if p.leap_to.x >= p.x { 1.0 } else { -1.0 };
    }

    pub fn random_x(&self) -> f64 { rnd(self.min_x + 30.0 * self.s, (self.min_x + 31.0 * self.s).max(self.max_x - 30.0 * self.s)) }

    /// The stretch a pet roams: its own spot, or the Dock.
    pub fn range(&self, p: &Pet) -> Span {
        if p.floor < 0.0 { return if self.low_roam() { Span { lo: self.min_x, hi: self.width } } else { self.corner_span() } }
        p.span.unwrap_or(Span { lo: self.min_x, hi: self.max_x })
    }

    pub fn random_x_for(&self, p: &Pet) -> f64 {
        let r = self.range(p);
        rnd(r.lo + 30.0 * self.s, (r.lo + 31.0 * self.s).max(r.hi - 30.0 * self.s))
    }

    pub fn clamp_x(&self, p: &Pet, x: f64, margin: f64) -> f64 {
        let r = self.range(p);
        clamp(x, r.lo + margin, (r.lo + margin + 1.0).max(r.hi - margin))
    }

    /// Makes the spot where a pet is now its home, so it carries on there instead of going back to the Dock.
    pub fn settle(&mut self, p: &mut Pet) {
        if p.y <= 10.0 * self.s {
            self.go_home(p, false);
            return;
        }
        p.floor = p.y.min(self.height - self.ground - p.size.height - 10.0);
        p.span = Some(Span { lo: (p.x - ROAM).max(0.0), hi: self.width.min(p.x + ROAM) });
    }

    /// Picks somewhere higher or lower to hop to, so pets roam the whole screen rather than one row.
    pub fn hop_target(&self, p: &Pet) -> Point {
        let s = self.s;
        // From down in the corner, the way out is back up onto the end of the Dock.
        if p.floor < 0.0 { return pt(self.dock_edge(p, p.x), 0.0) }
        let top = self.height - self.ground - p.size.height - 20.0;
        // Never land where the user is typing; the Dock is always fine.
        for _ in 0..8 {
            let up = p.floor < 30.0 * s || (p.floor < top - 60.0 * s && chance(0.5));
            let mut y = clamp(p.floor + (if up { 1.0 } else { -1.0 }) * rnd(60.0, 160.0) * s, 0.0, top.max(0.0));
            if y < 30.0 * s { y = 0.0 }
            let x = p.x + (if chance(0.8) { p.facing } else { -p.facing }) * rnd(40.0, 120.0) * s;
            let half = p.size.width / 2.0;
            let spot = pt(if y == 0.0 { self.clamp_to_dock(x, half) } else { clamp(x, half, self.width - half) }, y);
            let landing = Rect::new(spot.x - half, self.ground + y, p.size.width, p.size.height);
            if y == 0.0 || !self.typing_zone.is_some_and(|z| z.intersects(&landing)) { return spot }
        }
        pt(self.clamp_to_dock(p.x, p.size.width / 2.0), 0.0)
    }

    /// Sends a pet back to living on the Dock.
    pub fn go_home(&mut self, p: &mut Pet, fall: bool) {
        // Home is the Dock, or the screen bottom when the pet isn't over the Dock.
        p.floor = self.base_floor(p.x);
        p.span = None;
        if p.floor < 0.0 { p.x = self.clamp_x(p, p.x, p.size.width / 2.0) }
        if fall { self.start(p, Falling, false) }
    }

    /// The nearest spot to `x` standing on the Dock itself.
    pub fn dock_edge(&self, p: &Pet, x: f64) -> f64 {
        let half = p.size.width / 2.0;
        let x = self.clamp_to_dock(x, half);
        if self.below <= 0.0 { return x }
        clamp(x, self.dock_start + half, (self.dock_start + half + 1.0).max(self.dock_end - half))
    }

    /// The ground at `x` for pets living at the bottom: the Dock's top, or the screen's bottom edge beside it.
    pub fn base_floor(&self, x: f64) -> f64 {
        if self.below > 0.0 && (x < self.dock_start || x > self.dock_end) { -self.below } else { 0.0 }
    }

    /// Whether pets may roam the screen bottom beside the Dock (the walk area is the whole screen),
    /// rather than only curling up in the corner.
    pub fn low_roam(&self) -> bool { self.below > 0.0 && (self.max_x > self.dock_end + 1.0 || self.min_x < self.dock_start - 1.0) }

    pub fn clamp_to_dock(&self, x: f64, margin: f64) -> f64 {
        clamp(x, self.min_x + margin, (self.min_x + margin + 1.0).max(self.max_x - margin))
    }

    pub fn pet_rect(&self, p: &Pet) -> Rect {
        Rect::new(p.x - p.size.width / 2.0, self.ground + p.y - p.sink * p.size.height, p.size.width, p.size.height)
    }

    pub fn head_point(&self, p: &Pet) -> Point {
        pt(p.x + p.facing * p.size.width * 0.3, self.ground + p.y + p.size.height * 0.95 - p.sink * p.size.height)
    }

    pub fn bubble_y(&self, p: &Pet) -> f64 { self.ground + p.y + p.size.height + 8.0 }

    /// Where a visiting pet sits: beside the cursor tip, feet just below it.
    pub fn perch_point(&self, p: &Pet) -> Point {
        let side = if p.x <= self.mouse.x { -1.0 } else { 1.0 };
        pt(
            clamp(self.mouse.x + side * (p.size.width / 2.0 + 6.0), 20.0 * self.s, self.width - 20.0 * self.s),
            clamp(self.mouse.y - self.ground - 10.0 * self.s, 0.0, self.height - p.size.height - 20.0),
        )
    }

    /// The nearest bowl with food left on the pet's level.
    pub fn nearest_bowl(&self, p: &Pet) -> Option<usize> {
        (0..self.bowls.len())
            .filter(|&i| self.bowls[i].food > 0.0 && same_level(self.bowls[i].floor, p.floor))
            .min_by(|&a, &b| (self.bowls[a].x - p.x).abs().total_cmp(&(self.bowls[b].x - p.x).abs()))
    }

    /// The nearest ball on the pet's level.
    pub fn nearest_ball(&self, p: &Pet) -> Option<usize> {
        (0..self.balls.len()).filter(|&i| same_level(self.balls[i].floor, p.floor)).min_by(|&a, &b| (self.balls[a].x - p.x).abs().total_cmp(&(self.balls[b].x - p.x).abs()))
    }

    /// The nearest butterfly above the pet's level.
    pub fn nearest_butterfly(&self, p: &Pet) -> Option<usize> {
        (0..self.butterflies.len())
            .filter(|&i| same_level(self.butterflies[i].floor, p.floor))
            .min_by(|&a, &b| (self.butterflies[a].x - p.x).abs().total_cmp(&(self.butterflies[b].x - p.x).abs()))
    }

    pub fn emit(&mut self, text: &str, at: Point, size: f64) { self.emit_with(text, at, size, pt(0.0, 28.0), 1.4) }

    pub fn emit_with(&mut self, text: &str, at: Point, size: f64, v: Point, life: f64) {
        self.particles.push(Particle { text: text.to_string(), p: at, v, ttl: life, life, size: size * self.s });
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
