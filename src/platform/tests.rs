//! Unit tests for platform/mod.rs.

use super::*;
use crate::support::pt;

#[test]
fn input_without_cursor_movement_counts_as_a_key_press() {
    let mut k = KeyGuess::new();
    assert_eq!(k.update(30.0, Some(pt(1.0, 1.0))), f64::INFINITY, "nothing typed yet");
    assert!(k.update(0.0, Some(pt(1.0, 1.0))) < 1.0, "input while the cursor sat still");
}

#[test]
fn moving_the_mouse_is_not_typing() {
    let mut k = KeyGuess::new();
    k.update(30.0, Some(pt(1.0, 1.0)));
    assert_eq!(k.update(0.0, Some(pt(50.0, 1.0))), f64::INFINITY);
    assert_eq!(k.update(0.5, Some(pt(50.0, 1.0))), f64::INFINITY, "idle time growing isn't new input");
}

#[test]
fn terminal_caret_follows_the_last_text_on_screen() {
    use crate::support::Rect;
    let screen = "$ ls\nfoo  bar\n$ git st\n\n";
    let c = terminal_caret(screen, Rect::new(100.0, 50.0, 80.0, 100.0)).unwrap();
    // 5 rows of 20, 8 columns of 10: the cursor sits after "$ git st" on the third row.
    assert_eq!((c.x, c.y, c.w, c.h), (180.0, 90.0, 10.0, 20.0));
}

#[test]
fn terminal_tracker_follows_the_row_being_typed_on() {
    use crate::support::Rect;
    let frame = Rect::new(0.0, 0.0, 100.0, 40.0);
    let mut t = TerminalTracker::default();
    // A prompt with a status line under it, like Claude Code: the first look can only guess the last line.
    let first = t.update("> hi      \n\n>> status\n", frame).unwrap();
    assert_eq!(first.y, 20.0);
    // 4 rows of 10, 10 columns of 10: typing "!" after "> hi" puts the caret after it on the top row.
    let c = t.update("> hi!     \n\n>> status\n", frame).unwrap();
    assert_eq!((c.x, c.y), (50.0, 0.0));
    // Nothing changed since: it stays put rather than jumping back to the status line.
    assert_eq!(t.update("> hi!     \n\n>> status\n", frame), Some(c));
}

#[test]
fn terminal_tracker_starts_over_when_the_window_changes() {
    use crate::support::Rect;
    let mut t = TerminalTracker::default();
    t.update("> a\n\n$ x\n", Rect::new(0.0, 0.0, 30.0, 40.0));
    t.update("> ab\n\n$ x\n", Rect::new(0.0, 0.0, 30.0, 40.0));
    let c = t.update("> ab\n\n$ x\n", Rect::new(0.0, 0.0, 40.0, 40.0)).unwrap();
    assert_eq!(c.y, 20.0, "a resized window is guessed afresh from its last line");
}

#[test]
fn terminal_caret_needs_text() {
    use crate::support::Rect;
    assert!(terminal_caret("", Rect::new(0.0, 0.0, 10.0, 10.0)).is_none());
    assert!(terminal_caret("  \n \n", Rect::new(0.0, 0.0, 10.0, 10.0)).is_none());
}

#[test]
fn terminal_tracker_learns_the_cell_size_from_a_full_screen() {
    use crate::support::Rect;
    let frame = Rect::new(0.0, 0.0, 40.0, 40.0);
    let mut t = TerminalTracker::default();
    // A full 4x4 screen: cells are 10x10.
    t.update("aaaa\nbbbb\ncccc\n$ x ", frame);
    // A fresh shell only reports its one line, which on its own would look like a 20x40 cell.
    let c = t.update("$ ab", frame).unwrap();
    assert_eq!((c.x, c.y, c.w, c.h), (40.0, 0.0, 10.0, 10.0));
}

#[test]
fn terminal_tracker_starts_from_a_cell_size_learned_before() {
    use crate::support::Rect;
    let mut t = TerminalTracker::with_cell(Some((10.0, 10.0)));
    let c = t.update("$ ab", Rect::new(0.0, 0.0, 40.0, 40.0)).unwrap();
    assert_eq!((c.x, c.y, c.w, c.h), (40.0, 0.0, 10.0, 10.0));
    assert_eq!(t.cell(), Some((10.0, 10.0)));
}

#[test]
fn the_local_clock_reads_a_time_of_day() {
    let h = local_hour();
    assert!((0.0..24.0).contains(&h), "{h}");
}

#[test]
#[ignore = "prints what the audio devices report, for checking by hand"]
fn audio_state() { eprintln!("AUDIO ax={} in_call={} playing={} battery={:?}", accessibility_trusted(), in_call(), audio_playing(), battery()) }

#[test]
fn meet_calls_are_told_from_meets_home_page() {
    assert!(is_meeting_title("Meet - abc-defg-hij"));
    assert!(is_meeting_title("Meet – abc-defg-hij - Google Chrome - Work"));
    assert!(!is_meeting_title("Google Meet"));
    assert!(!is_meeting_title("Meet - Google Chrome"));
    assert!(!is_meeting_title("Meet - abc-defg"));
    assert!(!is_meeting_title("Meeting notes - abc-defg-hij"));
}
