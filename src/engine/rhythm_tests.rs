//! Unit tests for engine/rhythm.rs.

use super::*;
use crate::testing::world;
use DayPart::*;

fn run(w: &mut World, seconds: f64) {
    for _ in 0..(seconds * 30.0) as usize { w.with_pet(0, |w, p| w.update(p, 1.0 / 30.0)) }
}

#[test]
fn the_day_has_its_parts() {
    let bed = Some(22);
    assert_eq!(day_part(8.0, bed), Morning);
    assert_eq!(day_part(12.0, bed), Midday);
    assert_eq!(day_part(15.0, bed), Afternoon);
    assert_eq!(day_part(19.5, bed), Evening);
    assert_eq!(day_part(22.0, bed), Night);
    assert_eq!(day_part(3.0, bed), Night, "after midnight it's still night");
    assert_eq!(day_part(WAKE, bed), Morning);
}

#[test]
fn a_midnight_bedtime_keeps_the_evening_going() {
    assert_eq!(day_part(23.5, Some(0)), Evening);
    assert_eq!(day_part(0.5, Some(0)), Night);
    assert_eq!(day_part(6.9, Some(0)), Night);
    assert_eq!(day_part(21.5, Some(21)), Night);
}

#[test]
fn with_no_bedtime_the_day_has_no_rhythm() {
    for h in [0.0, 3.0, 8.0, 15.0, 20.0, 23.0] { assert_eq!(day_part(h, None), Midday) }
    assert_eq!(weight(Midday, Run), 1.0);
}

#[test]
fn mornings_stretch_afternoons_play_evenings_wind_down() {
    assert!(weight(Morning, Stretch) > 1.0 && weight(Morning, Run) < 1.0);
    assert!(weight(Afternoon, Run) > 1.0 && weight(Afternoon, Sleep) < 1.0);
    assert!(weight(Evening, Lounge) > 1.0 && weight(Evening, Run) < 1.0);
}

#[test]
fn at_night_free_will_goes_to_bed() {
    let mut w = world(1);
    (w.hour, w.bedtime) = (23.0, Some(22));
    w.with_pet(0, |w, p| {
        w.start(p, Walk, false);
        w.choose_next(p);
    });
    assert_eq!(w.pets[0].activity, Sleep);
    assert!(w.pets[0].bubble.as_deref().unwrap().contains("Bedtime"));
}

#[test]
fn at_night_a_rested_pet_sleeps_on() {
    let mut w = world(1);
    (w.hour, w.bedtime) = (2.0, Some(22));
    w.pets[0].energy = 99.0;
    w.with_pet(0, |w, p| {
        w.start(p, Sleep, false);
        (p.step, p.timer) = (crate::engine::behavior::ASLEEP, 0.5);
    });
    run(&mut w, 5.0);
    assert_eq!(w.pets[0].activity, Sleep, "full energy doesn't wake it at night");
    assert!(w.pets[0].timer > 1.0, "the nap goes on");
}

#[test]
fn in_the_morning_it_wakes_and_says_so() {
    let mut w = world(1);
    (w.hour, w.bedtime) = (7.5, Some(22));
    w.with_pet(0, |w, p| {
        w.start(p, Sleep, false);
        (p.step, p.timer) = (crate::engine::behavior::ASLEEP, 0.01);
    });
    run(&mut w, 0.1);
    assert_eq!(w.pets[0].activity, Stretch);
    assert!(w.pets[0].bubble.as_deref().unwrap().contains("Good morning"));
}

#[test]
fn a_nap_it_was_sent_for_ends_even_at_night() {
    let mut w = world(1);
    (w.hour, w.bedtime) = (23.0, Some(22));
    w.with_pet(0, |w, p| {
        w.start(p, Sleep, true);
        (p.step, p.timer) = (crate::engine::behavior::ASLEEP, 0.01);
    });
    run(&mut w, 0.1);
    assert_ne!(w.pets[0].activity, Sleep);
}
