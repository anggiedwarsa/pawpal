//! Unit tests for domain/voice.rs.

use super::*;

fn voice(json: &str) -> Voice { serde_json::from_str(json).unwrap() }

#[test]
fn a_key_can_hold_one_line_or_a_list() {
    let v = voice(r#"{ "hello": "Blub! 👋", "sounds": ["Blub", "Blub"] }"#);
    assert_eq!(v.get("hello"), Some("Blub! 👋"));
    assert_eq!(v.get("sounds"), Some("Blub"));
}

#[test]
fn missing_and_blank_lines_fall_through() {
    let v = voice(r#"{ "hello": "  ", "sounds": [], "petted": ["", "♥"] }"#);
    assert_eq!(v.get("hello"), None);
    assert_eq!(v.get("sounds"), None);
    assert_eq!(v.get("hungry"), None);
    assert_eq!(v.get("petted"), Some("♥"));
}

#[test]
fn an_empty_voice_is_empty() {
    assert!(Voice::default().is_empty());
    assert!(!voice(r#"{ "yum": "Nom" }"#).is_empty());
}
