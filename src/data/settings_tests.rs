//! Unit tests for data/settings.rs.

use super::*;

#[test]
fn missing_keys_take_defaults() {
    let p: Prefs = serde_json::from_str(r#"{"walkArea":"screen"}"#).unwrap();
    assert_eq!((p.scale, p.auto, p.walk_area), (1.1, true, WalkArea::Screen));
    assert!(p.pets.is_empty() && p.asked_accessibility.is_none());
    assert!(!p.avoid_cursor, "pets don't run from the cursor unless asked");
    assert!(p.check_updates, "updates are looked for unless turned off");
    assert!(p.morning_agenda && p.focus_offers && p.back_to_back && p.calendar_url.is_none());
    assert_eq!(p.wrap_up, None, "no wrap-up unless a time is picked");
    assert!(p.late_nudges && p.weekly_summary && p.todos.is_empty() && p.days.is_empty());
}

#[test]
fn pets_round_trip_in_the_swift_format() {
    let json = r#"[{"name":"Mochi","species":"cat","skin":"Mochi","hunger":20,"energy":85,"happiness":70}]"#;
    let pets: Vec<PetSave> = serde_json::from_str(json).unwrap();
    assert_eq!((pets[0].name.as_str(), pets[0].species, pets[0].hunger), ("Mochi", Species::Cat, 20.0));
    assert_eq!(pets[0].coat, crate::domain::Coat::Natural, "pets saved before colours keep their natural colour");
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

#[test]
fn day_tallies_replace_the_same_day_and_keep_two_weeks() {
    for d in 1..=20 { save_day(DayRecord { date: format!("2026-10-{d:02}"), breaks: d, ..Default::default() }) }
    save_day(DayRecord { date: "2026-10-20".into(), breaks: 99, ..Default::default() });
    let days = days();
    assert_eq!(days.len(), KEEP_DAYS);
    assert_eq!(days.first().map(|d| d.date.as_str()), Some("2026-10-07"), "the oldest are forgotten");
    assert_eq!(days.last().map(|d| (d.date.as_str(), d.breaks)), Some(("2026-10-20", 99)), "a day is saved once, latest wins");
}

#[test]
fn todos_are_saved() {
    set_todos(vec![Todo { id: 1, text: "Send invoice".into() }]);
    assert_eq!(todos(), vec![Todo { id: 1, text: "Send invoice".into() }]);
    set_todos(Vec::new());
}
