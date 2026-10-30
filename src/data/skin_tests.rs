//! Unit tests for data/skin.rs.

use super::*;
use crate::testing::skin;

#[test]
fn clips_fall_back_in_order() {
    let s = skin(&["lie", "sit"]);
    assert_eq!(s.clip_for(Idle).map(|c| c.0), Some("sit"));
    assert_eq!(s.clip_for(Lounge).map(|c| c.0), Some("lie"));
    assert!(s.clip_for(Jump).is_none());
}

#[test]
fn travelling_needs_walk_or_run() {
    let s = skin(&["eat", "sit"]);
    assert!(!s.supports(Eat), "eating means walking to the bowl");
    assert_eq!(s.missing_clip(Eat), "walk");
    assert!(s.supports(Dragged) && s.supports(Falling) && s.supports(Idle));
    let s = skin(&["eat", "walk"]);
    assert!(s.supports(Eat));
    assert_eq!(s.missing_clip(Sleep), "sleep");
}

#[test]
fn every_activity_has_fallbacks() {
    for a in crate::domain::Activity::MENU { assert!(!fallbacks(a).is_empty(), "{a:?}") }
    assert_eq!(ACTIONS.iter().find(|a| a.0 == "sit").map(|a| a.2), Some(74.0));
}

#[test]
fn clip_meta_matches_the_swift_json() {
    let meta: ClipMeta = serde_json::from_str(r#"{"fps":12,"speed":40,"height":62,"facesRight":true,"reach":0.4}"#).unwrap();
    assert_eq!((meta.fps, meta.faces_right, meta.reach, meta.pixel_art), (12.0, true, Some(0.4), None));
    let out = serde_json::to_string(&meta).unwrap();
    assert!(out.contains("\"facesRight\":true") && !out.contains("pixelArt"));
}

#[test]
fn loads_a_clip_folder_with_sorted_frames() {
    let dir = std::env::temp_dir().join(format!("pawpal-clip-{}", fastrand::u64(..)));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["frame_001.png", "frame_000.png", "notes.txt"] { std::fs::write(dir.join(f), b"x").unwrap() }
    assert!(load_clip(&dir).is_none(), "no clip.json yet");
    std::fs::write(dir.join("clip.json"), r#"{"fps":8,"speed":0,"height":40,"facesRight":false,"pixelArt":true}"#).unwrap();
    let clip = load_clip(&dir).unwrap();
    let names: Vec<_> = clip.files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["frame_000.png", "frame_001.png"]);
    assert!(clip.pixel_art && !clip.faces_right && clip.count() == 2);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn skins_live_under_pawpal() {
    assert!(root().ends_with("PawPal/Skins"));
}

#[test]
fn a_skill_plays_its_own_clip_and_nothing_else() {
    use crate::testing::{pet_file, skin_with};
    let k = skin_with(&["walk", "idle", "bubbles"], pet_file(r#"{ "skills": [
        { "name": "🫧 Blow Bubbles", "clip": "bubbles" }, { "name": "🌀 Spin", "clip": "spin" } ] }"#));
    assert_eq!(k.clip_for(Skill(0)).map(|c| c.0), Some("bubbles"));
    assert!(k.clip_for(Skill(1)).is_none() && !k.supports(Skill(1)), "no spin clip, and no falling back to idle");
    assert!(k.clip_for(Skill(9)).is_none());
    assert_eq!(k.missing_clip(Skill(1)), "spin");
    assert_eq!(k.missing_clip(Skill(9)), "idle");
    let skills: Vec<_> = k.skills().map(|(a, s)| (a, s.name.as_str())).collect();
    assert_eq!(skills, [(Skill(0), "🫧 Blow Bubbles")]);
}

#[test]
fn a_reminder_plays_the_chosen_clip_if_there_is_one() {
    use crate::testing::{pet_file, skin_with};
    let k = skin_with(&["sit", "meow"], pet_file(r#"{ "remind": "meow" }"#));
    assert_eq!(k.clip_for(Remind).map(|c| c.0), Some("meow"));
    let k = skin_with(&["sit"], pet_file(r#"{ "remind": "meow" }"#));
    assert_eq!(k.clip_for(Remind).map(|c| c.0), Some("sit"), "without that clip it sits as usual");
}

#[test]
fn clips_are_named_by_action_skill_or_folder() {
    use crate::testing::{pet_file, skin_with};
    let k = skin_with(&[], pet_file(r#"{ "skills": [{ "name": " 🫧 Bubbles ", "clip": "bubbles" }] }"#));
    assert_eq!(k.clip_title("meow"), "Meow");
    assert_eq!(k.clip_title("bubbles"), "🫧 Bubbles");
    assert_eq!(k.clip_title("odd"), "odd");
}

#[test]
fn loads_a_skill_clip_folder_named_in_the_pet_file() {
    let dir = std::env::temp_dir().join(format!("pawpal-skill-{}", fastrand::u64(..)));
    std::fs::create_dir_all(dir.join("bubbles")).unwrap();
    std::fs::write(dir.join("bubbles/clip.json"), r#"{"fps":8,"speed":0,"height":60,"facesRight":true}"#).unwrap();
    std::fs::write(dir.join("bubbles/frame_000.png"), "x").unwrap();
    std::fs::write(dir.join("pet.json"), r#"{ "skills": [{ "name": "Bubbles", "clip": "bubbles" }] }"#).unwrap();
    let k = load_skin(&dir).unwrap();
    assert!(k.supports(Skill(0)));
    let _ = std::fs::remove_dir_all(dir);
}
