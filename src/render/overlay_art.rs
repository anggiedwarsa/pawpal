//! Artwork for toys, food bowls, butterflies and speech bubbles.

use femtovg::{Color, LineCap, Paint, Path};

use super::stage::{Img, Stage};
use crate::domain::{Ball, Bowl, Butterfly};
use crate::engine::World;
use crate::support::*;

fn color(c: Rgba) -> Color { Color::rgbaf(c.r, c.g, c.b, c.a) }
fn fill(c: Rgba) -> Paint { Paint::color(color(c)) }
fn stroke(c: Rgba, w: f64) -> Paint { Paint::color(color(c)).with_line_width(w as f32) }

impl Stage {
    fn ellipse(&mut self, c: Point, rx: f64, ry: f64, rot: f64, paint: &Paint) {
        self.canvas.save();
        self.canvas.translate(c.x as f32, c.y as f32);
        self.canvas.rotate(rot as f32);
        let mut path = Path::new();
        path.ellipse(0.0, 0.0, rx as f32, ry as f32);
        self.canvas.fill_path(&path, paint);
        self.canvas.restore();
    }

    pub(super) fn draw_ball(&mut self, b: &Ball, world: &World) {
        let s = world.s;
        let r = 7.0 * s;
        let c = pt(b.x, world.ground + b.floor + b.y + r);
        self.canvas.save();
        self.canvas.set_global_alpha((b.life / 2.0).min(1.0) as f32);
        let mut circle = Path::new();
        circle.circle(c.x as f32, c.y as f32, r as f32);
        self.canvas.fill_path(&circle, &fill(Rgba::hex(0xE74C3C)));
        // The stripe runs edge to edge, so it ends on the ball's outline.
        let mut stripe = Path::new();
        stripe.move_to((c.x - b.rot.cos() * r) as f32, (c.y - b.rot.sin() * r) as f32);
        stripe.line_to((c.x + b.rot.cos() * r) as f32, (c.y + b.rot.sin() * r) as f32);
        self.canvas.stroke_path(&stripe, &stroke(Rgba::white(1.0, 1.0), 2.2 * s).with_line_cap(LineCap::Butt));
        self.canvas.stroke_path(&circle, &stroke(Rgba::hex(0x922B21), 1.0));
        self.canvas.restore();
    }

    /// The laser pointer's dot: a hot red centre in a soft glow.
    pub(super) fn draw_laser(&mut self, at: Point, world: &World) {
        let s = world.s;
        let glow = Paint::radial_gradient(at.x as f32, at.y as f32, 0.0, (10.0 * s) as f32, color(Rgba::hex_alpha(0xFF2A2A, 0.55)), color(Rgba::hex_alpha(0xFF2A2A, 0.0)));
        let mut halo = Path::new();
        halo.circle(at.x as f32, at.y as f32, (10.0 * s) as f32);
        self.canvas.fill_path(&halo, &glow);
        let mut dot = Path::new();
        dot.circle(at.x as f32, at.y as f32, (3.0 * s) as f32);
        self.canvas.fill_path(&dot, &fill(Rgba::hex(0xFF3B3B)));
    }

    pub(super) fn draw_bowl(&mut self, b: &Bowl, world: &World) {
        let (s, g) = (world.s, world.ground + b.floor);
        self.canvas.save();
        self.canvas.set_global_alpha((b.life / 2.0).min(1.0) as f32);
        if b.food > 0.0 {
            self.ellipse(pt(b.x, g + 9.0 * s), 12.0 * s, 4.0 * s * b.food + 1.0, 0.0, &fill(Rgba::hex(0x9C5B2E)));
        }
        let mut bowl = Path::new();
        bowl.move_to((b.x - 15.0 * s) as f32, (g + 9.0 * s) as f32);
        bowl.line_to((b.x + 15.0 * s) as f32, (g + 9.0 * s) as f32);
        bowl.line_to((b.x + 11.0 * s) as f32, g as f32);
        bowl.line_to((b.x - 11.0 * s) as f32, g as f32);
        bowl.close();
        self.canvas.fill_path(&bowl, &fill(Rgba::hex(0x4A90E2)));
        self.canvas.stroke_path(&bowl, &stroke(Rgba::hex(0x2C5F9E), 1.0));
        self.canvas.restore();
    }

    pub(super) fn draw_butterfly(&mut self, b: &Butterfly, world: &World) {
        let s = world.s;
        let flap = (b.t * 16.0).sin().abs() * 0.8 + 0.2;
        self.canvas.save();
        self.canvas.translate(b.x as f32, (world.ground + b.floor + b.y) as f32);
        self.canvas.scale((s * b.dir) as f32, s as f32);
        for side in [-1.0, 1.0] {
            self.ellipse(pt(side * 5.0 * flap, 3.0), 5.0 * flap, 6.0, side * 0.4, &fill(b.color));
            self.ellipse(pt(side * 4.0 * flap, -3.0), 3.5 * flap, 4.0, -side * 0.4, &fill(b.color.darker(0.15)));
        }
        self.ellipse(pt(0.0, 0.0), 1.2, 6.0, 0.0, &fill(Rgba::hex(0x333333)));
        self.canvas.restore();
    }

    pub(super) fn bubble_image(&mut self, text: &str) -> Option<Img> { self.text_image(text, 12.0, 600, Rgba::hex(0x333333)) }

    pub(super) fn draw_bubble(&mut self, text: &str, at: Point) {
        let Some(img) = self.bubble_image(text) else { return };
        let (w, h) = (img.w + 16.0, img.h + 8.0);
        let x = clamp(at.x - w / 2.0, 4.0, self.width - w - 4.0);
        let r = Rect::new(x, at.y, w, h);
        let mut bubble = Path::new();
        bubble.rounded_rect(r.x as f32, r.y as f32, r.w as f32, r.h as f32, 9.0);
        let tx = clamp(at.x, r.min_x() + 10.0, r.max_x() - 10.0);
        let mut tail = Path::new();
        tail.move_to((tx - 5.0) as f32, (r.min_y() + 1.0) as f32);
        tail.line_to(tx as f32, (r.min_y() - 6.0) as f32);
        tail.line_to((tx + 5.0) as f32, (r.min_y() + 1.0) as f32);
        tail.close();
        let white = fill(Rgba::white(1.0, 0.96));
        self.canvas.fill_path(&bubble, &white);
        self.canvas.fill_path(&tail, &white);
        self.canvas.stroke_path(&bubble, &stroke(Rgba::white(0.0, 0.15), 1.0));
        self.draw_image(img, r.min_x() + 8.0, r.min_y() + 4.0, 1.0, false);
    }
}
