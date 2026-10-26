//! Everything that differs per OS: where the Dock or taskbar is, where the user is typing,
//! the cursor, and how the overlay window behaves. Each OS module provides the same functions.
//!
//! Positions are global desktop coordinates in physical pixels, top-left origin, y down
//! (what winit uses); the app converts them to stage points.

use std::time::Instant;

use crate::support::Point;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

/// The strip pets live on: the Dock or taskbar and the screen it is on.
pub struct Shelf {
    /// The whole screen hosting the Dock or taskbar.
    pub screen: crate::support::Rect,
    /// Top of the usable area (below the menu bar); the overlay window starts here.
    pub top: f64,
    /// Where pets stand: the top edge of the Dock or taskbar.
    pub ground: f64,
    /// The walkable stretch along x.
    pub min_x: f64,
    pub max_x: f64,
    /// Where the Dock's icons really are along x (whatever the walk area), when known.
    pub dock: Option<crate::support::Span>,
}

/// Guesses when a key was last pressed from "time since any input" plus whether the cursor moved,
/// on systems that only report the former. Clicks without moving the mouse count too.
#[allow(dead_code)]
pub struct KeyGuess {
    last_idle: f64,
    last_cursor: Option<Point>,
    last_key: Option<Instant>,
}

#[allow(dead_code)]
impl KeyGuess {
    pub const fn new() -> KeyGuess { KeyGuess { last_idle: f64::INFINITY, last_cursor: None, last_key: None } }

    /// `idle` is seconds since any input; returns seconds since the last key press.
    pub fn update(&mut self, idle: f64, cursor: Option<Point>) -> f64 {
        let fresh_input = idle + 0.05 < self.last_idle;
        if fresh_input && cursor == self.last_cursor {
            self.last_key = Instant::now().checked_sub(std::time::Duration::from_secs_f64(idle.max(0.0)));
        }
        self.last_idle = idle;
        self.last_cursor = cursor;
        self.last_key.map_or(f64::INFINITY, |t| t.elapsed().as_secs_f64())
    }
}

/// Opens a folder in the system file browser.
pub fn open_folder(path: &std::path::Path) {
    let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(windows) { "explorer" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(path).spawn();
}

#[cfg(test)]
mod tests;
