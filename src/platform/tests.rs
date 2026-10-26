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
