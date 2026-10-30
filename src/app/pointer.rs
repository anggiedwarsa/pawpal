//! Turns clicks and drags on the overlay into world actions.

use std::time::Instant;

use super::App;
use crate::domain::Activity::*;
use crate::engine::picked_up_line;
use crate::platform;
use crate::support::*;

/// What the mouse is doing to a pet.
#[derive(Default)]
pub struct Pointer {
    pub pressed: Option<u64>,
    press_point: Point,
    dragging: bool,
    drag_vx: f64,
    last: Point,
    last_down: Option<(Instant, Point)>,
    clicks: u32,
}

impl App {
    pub fn pet_at(&self, at: Point) -> Option<u64> {
        self.world.pets.iter().rev().find(|p| p.sink < 0.8 && self.world.pet_rect(p).contains(at)).map(|p| p.id)
    }

    pub fn ball_at(&self, at: Point) -> Option<usize> {
        let w = &self.world;
        w.balls.iter().position(|b| (b.x - at.x).hypot(w.ground + b.floor + b.y + 7.0 * w.s - at.y) < 14.0 * w.s)
    }

    /// Whether clicks at the cursor should land on us (a pet or toy) or pass through to the apps below.
    pub fn wants_mouse(&self) -> bool {
        if self.pointer.pressed.is_some() { return true }
        let m = self.mouse_in_stage();
        self.pet_at(m).is_some() || self.ball_at(m).is_some()
    }

    pub fn mouse_down(&mut self, at: Point) {
        let double = self.pointer.last_down.is_some_and(|(t, p)| t.elapsed().as_secs_f64() < 0.4 && (p.x - at.x).hypot(p.y - at.y) < 5.0);
        self.pointer.clicks = if double { self.pointer.clicks + 1 } else { 1 };
        self.pointer.last_down = Some((Instant::now(), at));
        if let Some(id) = self.pet_at(at) {
            platform::will_interact();
            self.pointer.pressed = Some(id);
            self.pointer.press_point = at;
            self.pointer.last = at;
            self.pointer.dragging = false;
            return;
        }
        if let Some(i) = self.ball_at(at) { self.world.kick_ball(i) }
    }

    pub fn mouse_moved(&mut self, at: Point) {
        let Some(id) = self.pointer.pressed else { return };
        let ptr = &mut self.pointer;
        let start_drag = !ptr.dragging && (at.x - ptr.press_point.x).hypot(at.y - ptr.press_point.y) > 4.0;
        if start_drag { ptr.dragging = true }
        let dragging = ptr.dragging;
        ptr.drag_vx = (at.x - ptr.last.x) * 40.0;
        ptr.last = at;
        self.world.with_id(id, |w, p| {
            if start_drag {
                w.start(p, Dragged, false);
                p.say(p.line_or("picked_up", picked_up_line(p.species)), 1.5);
            }
            if dragging {
                // Pets can be carried anywhere on screen, not just along the Dock.
                p.x = clamp(at.x, p.size.width / 2.0, w.width - p.size.width / 2.0);
                p.y = clamp(at.y - w.ground - p.size.height * 0.7, 0.0, w.height - w.ground - p.size.height - 10.0);
                p.vy = 0.0;
            }
        });
    }

    pub fn mouse_up(&mut self) {
        let Some(id) = self.pointer.pressed.take() else { return };
        let (dragging, drag_vx, clicks) = (self.pointer.dragging, self.pointer.drag_vx, self.pointer.clicks);
        self.pointer.dragging = false;
        self.world.with_id(id, |w, p| if dragging { w.dropped(p, drag_vx) } else { w.clicked(p, clicks) });
        platform::done_interacting();
    }
}
