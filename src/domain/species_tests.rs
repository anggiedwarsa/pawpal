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
