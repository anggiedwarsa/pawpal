//! Unit tests for engine/focus.rs.

use super::*;
use crate::testing::world;

fn run(w: &mut World, seconds: f64) {
    for _ in 0..(seconds * 30.0) as usize {
        for i in 0..w.pets.len() { w.with_pet(i, |w, p| w.update(p, 1.0 / 30.0)) }
    }
}

#[test]
fn focusing_sends_every_pet_to_bed() {
    let mut w = world(2);
    w.start_focus(25);
    assert!(w.pets.iter().all(|p| p.activity == Sleep && !p.commanded));
    assert!(w.pets[0].bubble.as_deref().unwrap().contains("25 min"));
    assert!(w.pets[1].bubble.is_none(), "one pet says so");
}

#[test]
fn while_focusing_they_sleep_on() {
    let mut w = world(1);
    w.pets[0].energy = 99.0;
    w.start_focus(25);
    run(&mut w, 120.0);
    assert_eq!(w.pets[0].activity, Sleep, "full energy doesn't wake it");
}

#[test]
fn when_time_is_up_they_wake_and_one_says_well_done() {
    let mut w = world(2);
    w.start_focus(25);
    run(&mut w, 30.0);
    w.end_focus(true);
    assert!(!w.focusing);
    assert_eq!(w.pets.iter().filter(|p| p.activity == Remind).count(), 1);
    assert!(w.pets.iter().any(|p| p.activity == Stretch));
    assert!(w.pets.iter().any(|p| p.bubble.as_deref().is_some_and(|b| b.contains("Focus done"))));
}

#[test]
fn ending_early_wakes_them_without_a_fuss() {
    let mut w = world(1);
    w.start_focus(25);
    w.end_focus(false);
    assert_eq!(w.pets[0].activity, Stretch);
}

#[test]
fn at_night_ending_focus_leaves_them_asleep() {
    let mut w = world(1);
    (w.hour, w.bedtime) = (23.0, Some(22));
    w.start_focus(25);
    w.end_focus(false);
    assert_eq!(w.pets[0].activity, Sleep);
}
