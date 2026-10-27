use std::rc::Rc;

use super::{Activity, Species};
use crate::data::skin::Skin;
use crate::support::{Point, Size, Span};

/// A pet's state: where it is, what it's doing and how it feels. Rendering lives in the render layer.
/// What a pet was doing before a calendar heads-up, and where, so it can go back to it.
#[derive(Clone, Debug)]
pub struct Resume {
    pub activity: Activity,
    pub timer: f64,
    pub commanded: bool,
    pub x: f64,
    pub floor: f64,
    pub span: Option<Span>,
}

#[derive(Default)]
pub struct Pet {
    /// Stable identity (the Swift version compared objects with ===); 0 is never a real pet.
    pub id: u64,
    pub name: String,
    pub species: Species,
    pub skin: Option<Rc<Skin>>,

    // Position (x along the Dock, y = height above it) and motion, in points.
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    /// Where the pet lives: the height it stands at and the stretch it roams (None: on the Dock, floor 0).
    pub floor: f64,
    pub span: Option<Span>,
    pub facing: f64,
    /// 0 = standing on the Dock, 1 = fully ducked into it (peekaboo).
    pub sink: f64,
    /// On-screen size, reported by the renderer from the clip being shown.
    pub size: Size,
    /// How far ahead of its centre the pet's paws and face reach, reported by the renderer.
    pub reach: f64,

    pub activity: Activity,
    pub timer: f64,
    pub age: f64,
    pub step_time: f64,
    pub step: i32,
    pub target_x: Option<f64>,
    pub commanded: bool,
    pub fx: f64,
    pub cooldown: f64,
    pub leap_from: Point,
    pub leap_to: Point,
    pub leap_time: f64,
    /// In the middle of a jump from one level to another (moved along an arc, not by gravity).
    pub leaping: bool,
    /// How far along the current leap is, 0 to 1 (the jump clip follows it).
    pub leap_k: f64,
    /// Hopping up onto the Dock from the screen bottom beside it, in the middle of whatever it was doing.
    pub climbing: bool,
    /// The current activity is a trick it was asked for (see `engine::tricks`).
    pub trick: bool,
    /// What it was doing before going out to give a calendar heads-up.
    pub resume: Option<Resume>,

    // Needs, 0…100.
    pub hunger: f64,
    pub energy: f64,
    pub happiness: f64,

    pub bubble: Option<String>,
    pub bubble_ttl: f64,

    /// Seconds until a pet woken from its nap goes back to bed.
    pub back_to_sleep: Option<f64>,
}

impl Pet {
    pub fn new(id: u64, name: String, species: Species, skin: Option<Rc<Skin>>) -> Pet {
        Pet {
            id,
            name,
            species,
            skin,
            x: 300.0,
            facing: 1.0,
            size: Size { width: 60.0, height: 50.0 },
            reach: 30.0,
            timer: 2.0,
            hunger: 20.0,
            energy: 85.0,
            happiness: 70.0,
            ..Default::default()
        }
    }

    pub fn on_ground(&self) -> bool { self.y <= self.floor + 0.01 && self.vy == 0.0 }
    /// Pets only travel when they have a walking or running clip to show.
    pub fn can_move(&self) -> bool { self.skin.as_ref().is_some_and(|s| s.clips.contains_key("walk") || s.clips.contains_key("run")) }
    pub fn supports(&self, a: Activity) -> bool { self.skin.as_ref().is_some_and(|s| s.supports(a)) }

    pub fn say(&mut self, text: impl Into<String>, duration: f64) {
        self.bubble = Some(text.into());
        self.bubble_ttl = duration;
    }
}

#[cfg(test)]
#[path = "pet_tests.rs"]
mod tests;
