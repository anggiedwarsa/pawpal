//! Unit tests for render/stage.rs.

use super::*;
use crate::domain::Species;
use crate::testing::{full_skin, skin};

fn pet(clips: &[&str]) -> (Pet, Rc<Skin>) {
    let s = skin(clips);
    (Pet::new(1, "A".into(), Species::Cat, Some(s.clone())), s)
}

use std::rc::Rc;

#[test]
fn airborne_pets_show_their_jump() {
    let s = full_skin();
    let mut p = Pet::new(1, "A".into(), Species::Cat, Some(s.clone()));
    p.y = 40.0;
    p.vy = 10.0;
    assert_eq!(choose(&p, &s, 1.0).map(|c| c.0), Some("jump"));
    p.activity = VisitCursor;
    assert_eq!(choose(&p, &s, 1.0).map(|c| c.0), Some("sit"), "a pet perched by the cursor sits, even up high");
}

#[test]
fn moving_pets_walk_or_run_at_matching_speed() {
    let s = full_skin();
    let mut p = Pet::new(1, "A".into(), Species::Cat, Some(s.clone()));
    p.vx = 60.0;
    let (name, _, rate) = choose(&p, &s, 1.0).unwrap();
    assert_eq!((name, rate), ("walk", 1.5), "walk clips speed up to match the pet");
    p.vx = 200.0;
    assert_eq!(choose(&p, &s, 1.0).map(|c| c.0), Some("run"));
}

#[test]
fn standing_pets_show_their_activity_or_anything_they_have() {
    let (mut p, s) = pet(&["lie"]);
    p.activity = Sleep;
    assert_eq!(choose(&p, &s, 1.0).map(|c| c.0), Some("lie"));
    let (mut p, s) = pet(&["hunt"]);
    p.activity = Groom;
    assert_eq!(choose(&p, &s, 1.0).map(|c| c.0), Some("hunt"), "falls back to any clip at all");
}

#[test]
fn hopping_down_shows_only_the_falling_half_of_the_jump() {
    let (mut p, _) = pet(&["jump"]);
    p.leaping = true;
    (p.leap_from, p.leap_to, p.leap_k) = (pt(0.0, 200.0), pt(50.0, 100.0), 0.0);
    assert_eq!(jump_progress(&p, 1.0), Some(0.5), "it starts at the top of the jump");
    p.leap_to.y = 300.0;
    assert_eq!(jump_progress(&p, 1.0), Some(0.0), "going up starts from take-off");
    p.leaping = false;
    (p.y, p.floor, p.vy) = (50.0, 0.0, -380.0);
    assert!(jump_progress(&p, 1.0).unwrap() > 0.9, "falling fast is the landing end");
}

#[test]
fn only_going_down_tips_the_jump_nose_down() {
    let (mut p, _) = pet(&["jump"]);
    p.leaping = true;
    (p.leap_from, p.leap_to, p.leap_k) = (pt(0.0, 200.0), pt(-50.0, 100.0), 0.0);
    assert_eq!(fall_tilt(&p, 1.0), 0.0, "level at take-off");
    p.leap_k = 0.5;
    assert_eq!(fall_tilt(&p, 1.0), MAX_FALL_TILT, "fully tipped well before landing");
    p.leap_k = 1.0;
    assert_eq!(fall_tilt(&p, 1.0), 0.0, "level again on landing, so it doesn't snap upright");
    p.leap_to.y = 300.0;
    assert_eq!(fall_tilt(&p, 1.0), 0.0, "a leap up stays as drawn");
    p.leaping = false;
    (p.y, p.floor, p.vy) = (50.0, 0.0, 200.0);
    assert_eq!(fall_tilt(&p, 1.0), 0.0, "rising after a bounce");
    p.vy = -150.0;
    assert_eq!(fall_tilt(&p, 1.0), MAX_FALL_TILT / 2.0);
}

#[test]
fn a_flying_dragon_shows_its_wings_not_a_jump() {
    let (mut p, s) = pet(&["walk", "jump", "fly"]);
    p.activity = Fly;
    p.y = 120.0;
    p.vx = 160.0;
    assert_eq!(choose(&p, &s, 1.0).map(|c| c.0), Some("fly"));
}
