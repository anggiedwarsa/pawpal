//! The simulation: how pets behave and how toys move. Knows nothing about drawing or the OS.

mod behavior;
mod interaction;
mod props_physics;
mod world;

pub use behavior::picked_up_line;
pub use world::{World, TYPING_PAUSE};
