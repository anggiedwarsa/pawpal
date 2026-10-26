//! Unit tests for engine/interaction.rs.

use super::*;
use crate::testing::world;

fn sleeping(w: &mut World) {
    w.with_pet(0, |w, p| {
        w.start(p, Sleep, false);
        p.step = 1;
    });
}

#[test]
fn clicking_a_sleeping_pet_gets_a_sleepy_answer() {
    let mut w = world(1);
    sleeping(&mut w);
    w.with_pet(0, |w, p| w.clicked(p, 1));
    let p = &w.pets[0];
    assert_eq!(p.activity, Sleep, "it stays asleep");
    assert_eq!(p.bubble.as_deref(), Some("What's up? I'm so sleepy… 😴"));
}

#[test]
fn double_clicking_wakes_it_for_a_while() {
    let mut w = world(1);
    sleeping(&mut w);
    w.with_pet(0, |w, p| w.clicked(p, 2));
    let p = &w.pets[0];
    assert_eq!(p.activity, Jump);
    assert_eq!(p.back_to_sleep, Some(WAKE_TIME));
}

#[test]
fn double_clicking_an_awake_pet_doesnt_send_it_to_bed_later() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.clicked(p, 2));
    assert_eq!((w.pets[0].activity, w.pets[0].back_to_sleep), (Jump, None));
}

#[test]
fn clicking_an_awake_pet_pets_it() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.clicked(p, 1));
    assert_eq!(w.pets[0].activity, Petted);
}

#[test]
fn clicking_a_visiting_pet_purrs_or_sends_it_home() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        w.start(p, VisitCursor, false);
        w.clicked(p, 1);
    });
    assert_eq!(w.pets[0].activity, VisitCursor);
    w.with_pet(0, |w, p| w.clicked(p, 2));
    assert_eq!(w.pets[0].activity, Falling);
}

#[test]
fn dropped_pets_land_or_settle() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        p.y = 300.0;
        w.dropped(p, 9000.0);
    });
    assert_eq!((w.pets[0].floor, w.pets[0].vx), (300.0, 500.0), "up high it makes a home, and throws are capped");
}

#[test]
fn full_screen_sends_pets_to_peek_from_the_corner() {
    let mut w = world(2);
    w.set_fullscreen(true);
    assert!(w.pets.iter().all(|p| p.activity == Hide));
    for _ in 0..60 {
        for i in 0..2 { w.with_pet(i, |w, p| w.update(p, 1.0 / 30.0)) }
    }
    let (a, b) = (&w.pets[0], &w.pets[1]);
    assert_eq!((a.sink, a.y, a.facing), (0.5, 0.0, -1.0), "half its face shows, looking at the screen");
    assert!(a.x > 900.0 && b.x < a.x, "both wait in the bottom-right corner, side by side");
}

#[test]
fn leaving_full_screen_lets_pets_carry_on() {
    let mut w = world(1);
    w.set_fullscreen(true);
    w.set_fullscreen(false);
    assert_eq!(w.pets[0].activity, Idle);
    assert_eq!(w.pets[0].sink, 0.0);
    assert!(w.pets[0].bubble.is_some());
}

#[test]
fn a_carried_pet_isnt_snatched_away_by_full_screen() {
    let mut w = world(1);
    w.pets[0].activity = Dragged;
    w.set_fullscreen(true);
    assert_eq!(w.pets[0].activity, Dragged);
}

#[test]
fn hiding_pets_peek_from_the_real_screen_corner() {
    let mut w = world(1);
    (w.below, w.dock_start, w.dock_end, w.max_x) = (60.0, 50.0, 800.0, 800.0);
    w.set_fullscreen(true);
    for _ in 0..30 { w.with_pet(0, |w, p| w.update(p, 1.0 / 30.0)) }
    let p = &w.pets[0];
    assert_eq!((p.floor, p.y), (-60.0, -60.0));
}

#[test]
fn put_down_past_the_dock_it_stays_in_the_corner() {
    let mut w = world(1);
    (w.below, w.dock_start, w.dock_end, w.max_x) = (60.0, 50.0, 800.0, 800.0);
    w.with_pet(0, |w, p| {
        (p.x, p.y) = (900.0, -20.0);
        w.dropped(p, 0.0);
    });
    assert_eq!(w.pets[0].floor, -60.0);
}
