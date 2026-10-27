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

/// Guesses the caret of a terminal that shows its screen to Accessibility but not where the cursor is
/// (Ghostty): `text` is the visible grid, one line per row, inside `frame`. The cursor is taken to sit
/// just after the last character on screen, which is where a shell prompt leaves it.
#[allow(dead_code)]
pub fn terminal_caret(text: &str, frame: crate::support::Rect) -> Option<crate::support::Rect> {
    let lines: Vec<&str> = text.split('\n').collect();
    let (row, col) = last_text(&lines)?;
    Some(cell_rect(frame, grid_cell(&lines, frame), row, col))
}

fn last_text(lines: &[&str]) -> Option<(usize, usize)> {
    let row = lines.iter().rposition(|l| !l.trim().is_empty())?;
    Some((row, lines[row].trim_end().chars().count()))
}

/// The cell size if the text filled the frame both ways. Ghostty leaves out empty rows at the bottom and
/// spaces at line ends, so this is exact only for a full screen and too big otherwise.
fn grid_cell(lines: &[&str], frame: crate::support::Rect) -> (f64, f64) {
    let cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(1);
    (frame.w / cols as f64, frame.h / lines.len() as f64)
}

fn cell_rect(frame: crate::support::Rect, (cw, rh): (f64, f64), row: usize, col: usize) -> crate::support::Rect {
    crate::support::Rect::new(frame.x + col as f64 * cw, frame.y + row as f64 * rh, cw, rh)
}

/// Follows a terminal's caret from how its screen text changes between looks: keys land on the row that
/// changes, which in full-screen apps (Claude Code, vim) is often not the last line with text on it.
/// Until something changes, it guesses the end of the last text on screen.
#[allow(dead_code)]
#[derive(Default)]
pub struct TerminalTracker {
    text: String,
    frame: crate::support::Rect,
    caret: Option<crate::support::Rect>,
    /// The smallest cell size seen so far: each look is exact or too big, so this settles on the real
    /// size once any screen has been full of text.
    cell: Option<(f64, f64)>,
}

#[allow(dead_code)]
impl TerminalTracker {
    /// Starts from a cell size learned before, if any.
    pub fn with_cell(cell: Option<(f64, f64)>) -> TerminalTracker { TerminalTracker { cell, ..TerminalTracker::default() } }

    pub fn cell(&self) -> Option<(f64, f64)> { self.cell }

    pub fn update(&mut self, text: &str, frame: crate::support::Rect) -> Option<crate::support::Rect> {
        let (old, new): (Vec<&str>, Vec<&str>) = (self.text.split('\n').collect(), text.split('\n').collect());
        let (cw, rh) = grid_cell(&new, frame);
        let cell = self.cell.map_or((cw, rh), |(w, h)| (w.min(cw), h.min(rh)));
        self.cell = Some(cell);
        let same_grid = frame == self.frame && old.len() == new.len() && !self.text.is_empty();
        let changed = same_grid.then(|| last_change(&old, &new)).flatten();
        self.caret = match changed {
            Some((row, col)) => Some(cell_rect(frame, cell, row, col)),
            None if same_grid && self.caret.is_some() => self.caret,
            None => last_text(&new).map(|(row, col)| cell_rect(frame, cell, row, col)),
        };
        self.text = text.to_string();
        self.frame = frame;
        self.caret
    }
}

/// The last row that differs, and the column just after its last differing character.
fn last_change(old: &[&str], new: &[&str]) -> Option<(usize, usize)> {
    let row = (0..new.len()).rev().find(|&i| old[i] != new[i])?;
    let (a, b): (Vec<char>, Vec<char>) = (old[row].chars().collect(), new[row].chars().collect());
    let col = (0..a.len().max(b.len())).rev().find(|&i| a.get(i).unwrap_or(&' ') != b.get(i).unwrap_or(&' '))?;
    Some((row, col + 1))
}

/// The local time of day in hours (0–24), from the system's time zone.
#[cfg(unix)]
pub fn local_hour() -> f64 {
    // `struct tm` starts with these fields, in this order, on macOS and glibc alike.
    #[repr(C)]
    struct Tm { sec: i32, min: i32, hour: i32, rest: [i32; 6], gmtoff: std::ffi::c_long, zone: *const std::ffi::c_char }
    extern "C" {
        fn time(t: *mut i64) -> i64;
        fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm;
    }
    let mut tm = Tm { sec: 0, min: 0, hour: 12, rest: [0; 6], gmtoff: 0, zone: std::ptr::null() };
    // SAFETY: both only write to the locals passed in.
    unsafe {
        let t = time(std::ptr::null_mut());
        if localtime_r(&t, &mut tm).is_null() { return 12.0 }
    }
    tm.hour as f64 + tm.min as f64 / 60.0 + tm.sec as f64 / 3600.0
}

/// The local time of day in hours (0–24), from the system's time zone.
#[cfg(windows)]
pub fn local_hour() -> f64 {
    // SAFETY: GetLocalTime only fills in the struct passed to it.
    let t = unsafe {
        let mut t = std::mem::zeroed();
        windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut t);
        t
    };
    t.wHour as f64 + t.wMinute as f64 / 60.0 + t.wSecond as f64 / 3600.0
}

/// Opens a folder in the system file browser.
pub fn open_folder(path: &std::path::Path) {
    let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(windows) { "explorer" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(path).spawn();
}

#[cfg(test)]
mod tests;
