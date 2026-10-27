//! Unit tests for data/settings.rs.

use super::*;

#[test]
fn missing_keys_take_defaults() {
    let p: Prefs = serde_json::from_str(r#"{"walkArea":"screen"}"#).unwrap();
    assert_eq!((p.scale, p.auto, p.walk_area), (1.1, true, WalkArea::Screen));
    assert!(p.pets.is_empty() && p.asked_accessibility.is_none());
    assert!(!p.avoid_cursor, "pets don't run from the cursor unless asked");
    assert!(p.check_updates, "updates are looked for unless turned off");
}

#[test]
fn pets_round_trip_in_the_swift_format() {
    let json = r#"[{"name":"Mochi","species":"cat","skin":"Mochi","hunger":20,"energy":85,"happiness":70}]"#;
    let pets: Vec<PetSave> = serde_json::from_str(json).unwrap();
    assert_eq!((pets[0].name.as_str(), pets[0].species, pets[0].hunger), ("Mochi", Species::Cat, 20.0));
    assert!(serde_json::to_string(&pets).unwrap().contains("\"skin\":\"Mochi\""));
}

#[test]
fn settings_save_to_a_test_file() {
    assert!(path().to_string_lossy().contains("pawpal-test-"), "tests must not touch real settings");
    toggle_flipped("settings-test-skin");
    assert!(flipped("settings-test-skin"));
    toggle_flipped("settings-test-skin");
    assert!(!flipped("settings-test-skin"));
    set_asked_accessibility("build-1".into());
    assert_eq!(asked_accessibility().as_deref(), Some("build-1"));
    assert!(path().exists());
    let saved: Prefs = serde_json::from_str(&std::fs::read_to_string(path()).unwrap()).unwrap();
    assert_eq!(saved.asked_accessibility.as_deref(), Some("build-1"));
    set_terminal_cell(Some((8.5, 19.0)));
    assert_eq!(terminal_cell(), Some((8.5, 19.0)));
}

#[test]
fn scale_falls_back_when_unset() {
    assert!(scale() > 0.0);
}
