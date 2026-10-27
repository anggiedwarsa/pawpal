//! Unit tests for engine/greet.rs.

use super::*;
use crate::testing::world;

#[test]
fn coming_back_after_a_while_is_noticed_once() {
    let mut p = Presence::default();
    assert_eq!(p.back(1000.0, 0.0), None, "the first look only learns");
    assert_eq!(p.back(1100.0, 100.0), None, "still away");
    assert_eq!(p.back(1400.0, 400.0), None, "still away");
    assert_eq!(p.back(1401.0, 0.0), Some(401.0), "back");
    assert_eq!(p.back(1402.0, 0.5), None);
}

#[test]
fn a_short_pause_is_not_being_away() {
    let mut p = Presence::default();
    p.back(1000.0, 0.0);
    assert_eq!(p.back(1000.0 + AWAY - 10.0, 0.0), None);
}

#[test]
fn a_sleeping_computer_counts_as_away() {
    let mut p = Presence::default();
    p.back(1000.0, 30.0);
    // The idle counter barely moved while the computer slept, but the clock did.
    assert_eq!(p.back(1000.0 + 3.0 * 3600.0, 0.0), Some(3.0 * 3600.0 + 30.0));
}

#[test]
fn the_pet_nearest_the_cursor_says_hello() {
    let mut w = world(2);
    w.mouse.x = 520.0;
    w.welcome_back(AWAY);
    assert_eq!(w.pets[1].activity, Wave);
    assert_eq!(w.pets[1].bubble.as_deref(), Some("You're back! 🐾"));
    assert!(w.pets[0].bubble.is_none());
}

#[test]
fn after_a_long_time_it_missed_you() {
    let mut w = world(1);
    let joy = w.pets[0].happiness;
    w.welcome_back(MISSED);
    assert_eq!(w.pets[0].activity, Dance);
    assert!(w.pets[0].bubble.as_deref().unwrap().contains("missed you"));
    assert!(w.pets[0].happiness > joy);
}

#[test]
fn sleeping_pets_and_movies_are_left_alone() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.start(p, Sleep, false));
    w.welcome_back(AWAY);
    assert_eq!(w.pets[0].activity, Sleep);
    assert!(w.pets[0].bubble.is_none());

    let mut w = world(1);
    w.fullscreen = true;
    w.welcome_back(AWAY);
    assert!(w.pets[0].bubble.is_none());
}
