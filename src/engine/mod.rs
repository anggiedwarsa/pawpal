//! The simulation: how pets behave and how toys move. Knows nothing about drawing or the OS.

mod behavior;
pub mod alarms;
pub mod battery;
pub mod breaks;
pub mod calendar;
pub mod focus;
pub mod greet;
mod interaction;
mod props_physics;
pub mod rhythm;
pub mod tricks;
mod world;

pub use behavior::picked_up_line;
pub use world::{World, TYPING_PAUSE};
