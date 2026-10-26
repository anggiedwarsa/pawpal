use crate::support::{Point, Rgba, Span};

/// Toys and food sit on a level: the Dock (floor 0) or the height a pet lives at up the screen.
/// Their `y` is measured from that floor, and they stay within `area` along x.
pub struct Ball {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub rot: f64,
    pub life: f64,
    pub floor: f64,
    pub area: Span,
}

pub struct Bowl {
    pub x: f64,
    pub food: f64,
    pub life: f64,
    pub floor: f64,
}

pub struct Butterfly {
    pub x: f64,
    pub y: f64,
    pub t: f64,
    pub dir: f64,
    pub flee: f64,
    pub life: f64,
    pub color: Rgba,
    pub floor: f64,
    pub area: Span,
}

/// Whether two floors are the same level.
pub fn same_level(a: f64, b: f64) -> bool { (a - b).abs() < 1.0 }

/// A floating emoji or symbol (hearts, z's, notes…).
pub struct Particle {
    pub text: String,
    pub p: Point,
    pub v: Point,
    pub ttl: f64,
    pub life: f64,
    pub size: f64,
}

#[cfg(test)]
#[path = "props_tests.rs"]
mod tests;
