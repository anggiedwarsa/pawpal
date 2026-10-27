//! Unit tests for engine/breaks.rs.

use super::*;

/// Checks every 5 seconds from `from` for `minutes`, with the user `idle` seconds idle; returns how many nudges came.
fn work(t: &mut BreakTimer, from: f64, minutes: f64, idle: f64, every: u32) -> usize {
    (0..=(minutes * 12.0) as usize).filter(|&i| t.due(from + i as f64 * 5.0, idle, every, false)).count()
}

#[test]
fn a_nudge_comes_after_working_the_whole_time() {
    let mut t = BreakTimer::default();
    assert_eq!(work(&mut t, 0.0, 49.0, 1.0, 50), 0, "not yet");
    assert_eq!(work(&mut t, 49.0 * 60.0 + 5.0, 1.5, 1.0, 50), 1);
}

#[test]
fn after_a_nudge_the_count_starts_again() {
    let mut t = BreakTimer::default();
    assert_eq!(work(&mut t, 0.0, 60.0, 1.0, 25), 2);
}

#[test]
fn a_few_minutes_away_counts_as_a_break() {
    let mut t = BreakTimer::default();
    work(&mut t, 0.0, 40.0, 1.0, 50);
    assert!(!t.due(40.0 * 60.0 + 5.0, AWAY, 50, false), "away from the keyboard and mouse");
    assert_eq!(work(&mut t, 40.0 * 60.0 + 10.0, 40.0, 1.0, 50), 0, "the 40 minutes before don't count any more");
}

#[test]
fn reading_without_touching_anything_for_a_bit_still_counts_as_working() {
    let mut t = BreakTimer::default();
    assert_eq!(work(&mut t, 0.0, 51.0, AWAY - 1.0, 50), 1);
}

#[test]
fn the_computer_sleeping_counts_as_a_break() {
    let mut t = BreakTimer::default();
    work(&mut t, 0.0, 40.0, 1.0, 50);
    assert_eq!(work(&mut t, 40.0 * 60.0 + 3600.0, 20.0, 1.0, 50), 0);
}

#[test]
fn a_held_nudge_comes_once_the_hold_ends() {
    let mut t = BreakTimer::default();
    work(&mut t, 0.0, 49.0, 1.0, 50);
    for i in 1..=60 { assert!(!t.due(49.0 * 60.0 + i as f64 * 5.0, 1.0, 50, true), "held while a movie is on") }
    assert!(t.due(54.0 * 60.0 + 5.0, 1.0, 50, false));
}

#[test]
fn a_reset_starts_the_count_again() {
    let mut t = BreakTimer::default();
    work(&mut t, 0.0, 49.0, 1.0, 50);
    t.reset();
    assert_eq!(work(&mut t, 49.0 * 60.0 + 5.0, 10.0, 1.0, 50), 0);
}

#[test]
fn off_never_nudges() {
    let mut t = BreakTimer::default();
    assert_eq!(work(&mut t, 0.0, 300.0, 1.0, 0), 0);
}

#[test]
fn the_nudge_suggests_something() {
    assert!(NUDGES.contains(&nudge_text()));
}
