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

#[test]
fn a_pet_says_its_own_lines_and_borrows_the_rest_from_its_species() {
    use crate::testing::{pet_file, skin_with};
    let own = pet_file(r#"{ "lines": { "hello": "Blub! 👋", "sounds": ["Blub"], "yum": "Nom" } }"#);
    let p = Pet::new(1, "Bloop".into(), Species::Dog, Some(skin_with(&["walk"], own)));
    assert_eq!(p.line_or("hello", "Hi"), "Blub! 👋");
    assert_eq!(p.line_or("taunt", "Grr"), "Grr");
    assert_eq!(p.one_of("sounds", &["Woof"]), "Blub");
    assert_eq!(p.one_of("hungry", &["Food?"]), "Food?");
    assert_eq!(p.line("yum", "cat", "dog"), "Nom");
    assert_eq!(p.line("cuddle", "cat", "dog"), "dog", "the dog line, since it's based on a dog");
    let bare = Pet::new(2, "Mochi".into(), Species::Cat, None);
    assert_eq!(bare.line("yum", "cat", "dog"), "cat");
}

#[test]
fn which_colours_change_comes_from_the_pet_file_or_dragon_scales() {
    use crate::testing::{pet_file, skin_with};
    let teal = pet_file(r#"{ "recolor": { "hue": 180, "band": 12 } }"#);
    let p = Pet::new(1, "Teal".into(), Species::Cat, Some(skin_with(&["walk"], teal)));
    assert_eq!(p.recolor(), Some(Recolor { hue: 180.0, band: 12.0 }));
    assert_eq!(Pet::new(2, "Naga".into(), Species::Dragon, Some(skin(&["walk"]))).recolor(), Some(Recolor::LAVENDER));
    assert_eq!(Pet::new(3, "Mochi".into(), Species::Cat, Some(skin(&["walk"]))).recolor(), None);
}
