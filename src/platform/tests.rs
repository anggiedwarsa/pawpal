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
fn terminal_caret_needs_text() {
    use crate::support::Rect;
    assert!(terminal_caret("", Rect::new(0.0, 0.0, 10.0, 10.0)).is_none());
    assert!(terminal_caret("  \n \n", Rect::new(0.0, 0.0, 10.0, 10.0)).is_none());
}
