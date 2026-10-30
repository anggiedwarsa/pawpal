//! Unit tests for domain/species.rs.

use super::*;

#[test]
fn lines_differ_for_cats_and_dogs() {
    assert_eq!(Species::Cat.line("a", "b"), "a");
    assert_eq!(Species::Dog.line("a", "b"), "b");
    assert_ne!(Species::Cat.knead(), Species::Dog.knead());
    assert_ne!(Species::Cat.hello(), Species::Dog.hello());
}

#[test]
fn each_species_has_things_to_say() {
    assert!(Species::Cat.sounds().contains(&"Meow!"));
    assert!(Species::Dog.hungry().iter().any(|s| s.contains("FOOD")));
    assert!(!Species::Dog.petted().is_empty());
}

#[test]
fn species_serialise_as_lowercase_names() {
    assert_eq!(serde_json::to_string(&Species::Dog).unwrap(), "\"dog\"");
    assert_eq!(serde_json::from_str::<Species>("\"cat\"").unwrap(), Species::Cat);
}

#[test]
fn snakes_and_foxes_speak_for_themselves() {
    assert!(Species::Snake.sounds().iter().any(|s| s.contains("Hss")));
    assert!(Species::Fox.hungry().iter().any(|s| s.contains("tofu")));
    assert_eq!(Species::Snake.line("a", "b"), "a", "snakes borrow the cat lines");
    assert_eq!(Species::Fox.line("a", "b"), "b", "foxes borrow the dog lines");
}

#[test]
fn species_names_parse_loosely() {
    assert_eq!(Species::parse(" Snake\n"), Some(Species::Snake));
    assert_eq!(Species::parse("fox"), Some(Species::Fox));
    assert_eq!(Species::parse("unicorn"), None);
}

#[test]
fn every_species_has_a_play_fight_taunt() {
    let all = [Species::Cat, Species::Dog, Species::Snake, Species::Fox, Species::Trex, Species::Dragon];
    assert!(all.iter().all(|s| !s.taunt().is_empty()));
    assert_ne!(Species::Cat.taunt(), Species::Fox.taunt());
}

#[test]
fn dinosaurs_and_dragons_have_their_own_voices() {
    assert!(Species::Trex.sounds().iter().any(|s| s.contains("RAWR")));
    assert!(Species::Dragon.hungry().iter().any(|s| s.contains("gold")));
    assert_eq!(Species::parse("trex"), Some(Species::Trex));
    assert_eq!(Species::parse("Dragon"), Some(Species::Dragon));
}
