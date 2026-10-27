//! Calendar heads-ups: 10, 5 and 1 minute before an event, a pet walks out, jumps to the middle of the
//! screen and says what's coming up. Clicking it (or waiting 30 seconds) sends it back to what it was doing.

use std::collections::HashSet;

use super::World;
use crate::domain::Activity::{self, *};
use crate::domain::{CalendarEvent, ClickAction, Pet, Resume};
use crate::support::*;

/// Minutes before an event when a pet comes to say so.
pub const REMIND_AT: [u32; 3] = [10, 5, 1];
/// How long the pet waits in the middle of the screen with its bubble.
pub const REMIND_TIME: f64 = 30.0;

/// Stages of a heads-up, in `Pet::step`.
const GO_OUT: i32 = 0;
const LEAP_UP: i32 = 1;
const TELL: i32 = 2;
const GO_BACK: i32 = 3;

/// Which heads-ups have been given, so each one comes once.
#[derive(Default)]
pub struct Reminders {
    told: HashSet<(String, u32)>,
}

impl Reminders {
    /// The heads-up due at `now`, if any: the event and the minutes left (rounded up).
    /// Only the nearest mark counts, so starting the app 3 minutes before a meeting gives one heads-up, not three.
    pub fn due(&mut self, events: &[CalendarEvent], now: f64) -> Option<(CalendarEvent, u32)> {
        for e in events {
            let left = e.start - now;
            if left <= 0.0 { continue }
            let Some(mark) = REMIND_AT.into_iter().filter(|&m| left <= m as f64 * 60.0).min() else { continue };
            if self.told.contains(&(e.id.clone(), mark)) { continue }
            for m in REMIND_AT.into_iter().filter(|&m| m >= mark) {
                self.told.insert((e.id.clone(), m));
            }
            return Some((e.clone(), ((left / 60.0).ceil() as u32).max(1)));
        }
        None
    }
}

/// What the pet says: the event's name (shortened if long) and when it starts.
pub fn heads_up_text(title: &str, minutes: u32) -> String {
    let title = title.trim();
    let title = if title.is_empty() { "Your event" } else { title };
    let short: String = if title.chars().count() > 40 { title.chars().take(39).collect::<String>() + "…" } else { title.to_string() };
    if minutes <= 1 { format!("📅 {short} starts in 1 minute!") } else { format!("📅 {short} in {minutes} minutes") }
}

/// Activities that are only a moment long, so after a heads-up the pet just carries on calmly instead.
fn passing(a: Activity) -> bool { matches!(a, Dragged | Falling | Hop | Jump | Remind | Petted | Pounce) }

impl World {
    /// Sends the pet nearest the middle of the screen out with a heads-up (or refreshes the one already out).
    pub fn remind(&mut self, text: &str) { self.remind_with(text, None) }

    /// A heads-up that does something when the pet is clicked, such as joining the meeting.
    pub fn remind_with(&mut self, text: &str, on_click: Option<ClickAction>) {
        let mid = self.width / 2.0;
        if let Some(i) = self.pets.iter().position(|p| p.activity == Remind) {
            return self.with_pet(i, |_, p| {
                p.say(text, REMIND_TIME + 15.0);
                p.on_click = on_click;
                if p.step == TELL { p.timer = REMIND_TIME }
            });
        }
        let Some(i) = (0..self.pets.len())
            .filter(|&i| self.pets[i].activity != Dragged)
            .min_by(|&a, &b| (self.pets[a].x - mid).abs().total_cmp(&(self.pets[b].x - mid).abs()))
        else { return };
        self.with_pet(i, |w, p| {
            let resume = Resume {
                activity: if passing(p.activity) { Idle } else { p.activity },
                timer: p.timer,
                commanded: p.commanded,
                x: p.x,
                floor: p.floor,
                span: p.span,
            };
            w.start(p, Remind, true);
            p.resume = Some(resume);
            p.on_click = on_click;
            p.say(text, REMIND_TIME + 15.0);
        });
    }

    /// Where the pet stands to give its heads-up: the middle of the screen.
    fn remind_spot(&self, p: &Pet) -> Point {
        pt(self.width / 2.0, ((self.height - self.ground - p.size.height) / 2.0).max(0.0))
    }

    pub(super) fn remind_step(&mut self, p: &mut Pet, dt: f64) {
        match p.step {
            GO_OUT => {
                // Walk along its own level towards the middle, then jump up to it.
                if !p.on_ground() { return }
                let spot = self.remind_spot(p);
                let goal = self.clamp_x(p, spot.x, p.size.width / 2.0);
                if self.move_to(p, goal, 110.0 * self.s) {
                    self.plan_leap(p, spot);
                    p.step = LEAP_UP;
                }
            }
            LEAP_UP => {
                if self.leap(p, dt) {
                    p.step = TELL;
                    p.timer = REMIND_TIME;
                    p.bubble_ttl = REMIND_TIME;
                    p.facing = if p.x < self.mouse.x { 1.0 } else { -1.0 };
                }
            }
            TELL => {
                p.vx = 0.0;
                if p.timer <= 0.0 { self.end_remind(p) }
            }
            _ => {
                if self.leap(p, dt) { self.resume(p) }
            }
        }
    }

    /// Done telling (or clicked): hop back to where it was.
    pub fn end_remind(&mut self, p: &mut Pet) {
        if p.activity != Remind || p.step == GO_BACK { return }
        p.bubble = None;
        let Some(r) = p.resume.clone() else { return self.start(p, Idle, false) };
        let home = pt(r.x, r.floor);
        self.plan_leap(p, home);
        p.step = GO_BACK;
        p.timer = f64::INFINITY;
    }

    /// Back home: picks up what it was doing before the heads-up.
    fn resume(&mut self, p: &mut Pet) {
        let Some(r) = p.resume.take() else { return self.start(p, Idle, false) };
        (p.y, p.floor, p.span) = (r.floor, r.floor, r.span);
        self.start(p, r.activity, r.commanded);
        if r.activity != Sleep && r.timer.is_finite() && r.timer > 0.0 { p.timer = r.timer }
    }

    /// Starts a jump along an arc from where the pet is to `to`.
    fn plan_leap(&self, p: &mut Pet, to: Point) {
        p.leap_from = pt(p.x, p.y);
        p.leap_to = to;
        p.leap_time = 0.0;
        p.leaping = true;
        (p.vx, p.vy) = (0.0, 0.0);
        if (to.x - p.x).abs() > 1.0 { p.facing = if to.x > p.x { 1.0 } else { -1.0 } }
    }
}

#[cfg(test)]
#[path = "calendar_tests.rs"]
mod tests;
