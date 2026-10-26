//! Unit tests for domain/pet.rs.

use super::*;
use crate::testing::skin;

#[test]
fn new_pets_start_content_on_the_dock() {
    let p = Pet::new(3, "Mochi".into(), Species::Cat, None);
    assert_eq!((p.id, p.floor, p.facing, p.activity), (3, 0.0, 1.0, Activity::Idle));
    assert_eq!((p.hunger, p.energy, p.happiness), (20.0, 85.0, 70.0));
}

#[test]
fn pets_move_only_with_a_walk_or_run_clip() {
    let mut p = Pet::new(1, "A".into(), Species::Cat, Some(skin(&["sit"])));
    assert!(!p.can_move());
    assert!(p.supports(Activity::Sit) && !p.supports(Activity::Walk));
    p.skin = Some(skin(&["run"]));
    assert!(p.can_move());
    p.skin = None;
    assert!(!p.supports(Activity::Idle), "a pet without a skin can't do anything");
}

#[test]
fn pets_are_on_the_ground_only_when_resting_on_their_floor() {
    let mut p = Pet::new(1, "A".into(), Species::Cat, None);
    assert!(p.on_ground());
    p.vy = 10.0;
    assert!(!p.on_ground());
    p.vy = 0.0;
    p.y = 80.0;
    p.floor = 50.0;
    assert!(!p.on_ground(), "above its floor it's in the air");
}

#[test]
fn saying_something_shows_a_bubble_for_a_while() {
    let mut p = Pet::new(1, "A".into(), Species::Cat, None);
    p.say("hi", 2.0);
    assert_eq!((p.bubble.as_deref(), p.bubble_ttl), (Some("hi"), 2.0));
}
