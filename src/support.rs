//! Geometry, colour and randomness helpers shared by every layer.
//! Coordinates follow the old Cocoa convention: y grows upwards from the bottom of the stage.

use std::ops::Add;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub const fn pt(x: f64, y: f64) -> Point {
    Point { x, y }
}

impl Add for Point {
    type Output = Point;
    fn add(self, b: Point) -> Point {
        pt(self.x + b.x, self.y + b.y)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// An axis-aligned rectangle; `y` is its bottom edge in stage coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn min_x(&self) -> f64 { self.x }
    pub fn max_x(&self) -> f64 { self.x + self.w }
    pub fn min_y(&self) -> f64 { self.y }
    pub fn max_y(&self) -> f64 { self.y + self.h }
    pub fn is_empty(&self) -> bool { self.w <= 0.0 || self.h <= 0.0 }

    /// Whether the two overlap with some area, like CGRect.intersects.
    pub fn intersects(&self, o: &Rect) -> bool {
        !self.is_empty() && !o.is_empty()
            && self.min_x() < o.max_x() && o.min_x() < self.max_x()
            && self.min_y() < o.max_y() && o.min_y() < self.max_y()
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min_x() && p.x < self.max_x() && p.y >= self.min_y() && p.y < self.max_y()
    }
}

/// A closed range along x, such as the stretch a pet roams.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub lo: f64,
    pub hi: f64,
}

impl Span {
    pub fn contains(&self, x: f64) -> bool { x >= self.lo && x <= self.hi }
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }
/// Like Swift's `min(max(v, lo), hi)`: when lo > hi, hi wins (never panics, unlike f64::clamp).
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 { v.max(lo).min(hi) }
pub fn rnd(a: f64, b: f64) -> f64 { a + fastrand::f64() * (b - a) }
pub fn chance(p: f64) -> bool { fastrand::f64() < p }
pub fn pick<T: Copy>(items: &[T]) -> T { items[fastrand::usize(..items.len())] }

/// An sRGB colour with straight alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn hex(hex: u32) -> Rgba { Rgba::hex_alpha(hex, 1.0) }
    pub const fn hex_alpha(hex: u32, a: f32) -> Rgba {
        Rgba {
            r: ((hex >> 16) & 0xFF) as f32 / 255.0,
            g: ((hex >> 8) & 0xFF) as f32 / 255.0,
            b: (hex & 0xFF) as f32 / 255.0,
            a,
        }
    }
    pub const fn white(w: f32, a: f32) -> Rgba { Rgba { r: w, g: w, b: w, a } }
    pub fn darker(&self, f: f32) -> Rgba { Rgba { r: self.r * (1.0 - f), g: self.g * (1.0 - f), b: self.b * (1.0 - f), a: self.a } }
}

#[cfg(test)]
#[path = "support_tests.rs"]
mod tests;
