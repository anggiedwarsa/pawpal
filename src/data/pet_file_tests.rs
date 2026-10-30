//! Unit tests for data/pet_file.rs.

use std::path::PathBuf;

use super::*;

fn temp() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pawpal-petfile-{}", fastrand::u64(..)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn reads_a_whole_pet_file() {
    let dir = temp();
    std::fs::write(dir.join(FILE), r#"{ "kind": "Axolotl", "base": "Dog", "lines": { "hello": "Blub! 👋" }, "recolor": { "hue": 330 } }"#).unwrap();
    let pet = read(&dir);
    assert_eq!(pet.kind.as_deref(), Some("Axolotl"));
    assert_eq!(pet.base, Some(Species::Dog));
    assert_eq!(pet.lines.get("hello"), Some("Blub! 👋"));
    assert_eq!(pet.recolor.map(|r| r.hue), Some(330.0));
}

#[test]
fn an_unknown_base_counts_as_left_out() {
    let dir = temp();
    std::fs::write(dir.join(FILE), r#"{ "kind": "Unicorn", "base": "unicorn" }"#).unwrap();
    let pet = read(&dir);
    assert_eq!((pet.kind.as_deref(), pet.base), (Some("Unicorn"), None));
}

#[test]
fn falls_back_to_species_txt() {
    let dir = temp();
    std::fs::write(dir.join("species.txt"), "fox\n").unwrap();
    assert_eq!(read(&dir).base, Some(Species::Fox));
    std::fs::write(dir.join(FILE), r#"{ "lines": { "yum": "Nom" } }"#).unwrap();
    assert_eq!(read(&dir).base, Some(Species::Fox), "a pet file without a base keeps the species.txt one");
    std::fs::write(dir.join(FILE), r#"{ "base": "dragon" }"#).unwrap();
    assert_eq!(read(&dir).base, Some(Species::Dragon), "pet.json wins");
}

#[test]
fn a_broken_or_missing_file_reads_as_empty() {
    let dir = temp();
    assert_eq!(read(&dir), PetFile::default());
    std::fs::write(dir.join(FILE), "{ not json").unwrap();
    assert_eq!(read(&dir), PetFile::default());
}

#[test]
fn writes_only_what_was_set_and_reads_it_back() {
    let dir = temp();
    let pet = PetFile { kind: Some("Axolotl".into()), base: Some(Species::Cat), ..PetFile::default() };
    write(&dir, &pet).unwrap();
    let json = std::fs::read_to_string(dir.join(FILE)).unwrap();
    assert!(!json.contains("lines") && !json.contains("recolor"), "{json}");
    assert_eq!(read(&dir), pet);
}

#[test]
fn writing_somewhere_missing_fails() {
    assert!(write(&temp().join("gone"), &PetFile::default()).is_err());
}

#[test]
fn an_answer_is_a_built_in_species_or_a_new_kind() {
    assert_eq!(kind_from(" T-Rex "), Some(Ok(Species::Trex)));
    assert_eq!(kind_from("Axolotl "), Some(Err("Axolotl".to_string())));
    assert_eq!(kind_from("   "), None);
}

#[test]
fn reads_skills_with_sensible_defaults() {
    let dir = temp();
    std::fs::write(dir.join(FILE), r#"{ "skills": [
        { "name": "🫧 Blow Bubbles", "clip": "bubbles", "say": "Blub~", "burst": "🫧", "startle": true },
        { "name": "🌀 Spin", "clip": "spin", "seconds": 999, "often": -1, "say": ["Wheee", "Spin!"] }
    ] }"#).unwrap();
    let skills = read(&dir).skills;
    assert_eq!(skills.len(), 2);
    let (bubbles, spin) = (&skills[0], &skills[1]);
    assert_eq!((bubbles.seconds, bubbles.often, bubbles.startle), (4.0, 0.5, true));
    assert_eq!((bubbles.say.pick(), bubbles.burst.as_deref()), (Some("Blub~"), Some("🫧")));
    assert_eq!((spin.seconds, spin.often), (60.0, 0.0), "kept within bounds");
    assert!(spin.say.pick().is_some_and(|s| s == "Wheee" || s == "Spin!"));
}

#[test]
fn skills_that_cant_work_are_dropped() {
    let dir = temp();
    std::fs::write(dir.join(FILE), r#"{ "skills": [
        { "name": "Sneaky", "clip": "../walk" },
        { "name": "Hidden", "clip": ".spin" },
        { "name": " ", "clip": "spin" },
        { "name": "Fine", "clip": "fine_one-2" }
    ] }"#).unwrap();
    let names: Vec<String> = read(&dir).skills.into_iter().map(|k| k.name).collect();
    assert_eq!(names, ["Fine"]);
}

#[test]
fn a_pet_has_at_most_so_many_skills() {
    let dir = temp();
    let skills: Vec<String> = (0..40).map(|i| format!(r#"{{ "name": "S{i}", "clip": "s{i}" }}"#)).collect();
    std::fs::write(dir.join(FILE), format!(r#"{{ "skills": [{}] }}"#, skills.join(","))).unwrap();
    assert_eq!(read(&dir).skills.len(), MAX_SKILLS);
}

#[test]
fn clip_names_stay_inside_the_pet_folder() {
    assert!(is_clip_name("bubbles") && is_clip_name("Spin_2-x"));
    for bad in ["", "a/b", "..", ".x", "a b", "é", &"x".repeat(41)] { assert!(!is_clip_name(bad), "{bad:?}") }
}

#[test]
fn updating_keeps_the_rest_of_the_file() {
    let dir = temp();
    std::fs::write(dir.join(FILE), r#"{ "kind": "Axolotl", "skills": [{ "name": "x", "clip": "../bad" }] }"#).unwrap();
    update(&dir, |p| p.remind = Some("wave".into())).unwrap();
    let raw: PetFile = serde_json::from_slice(&std::fs::read(dir.join(FILE)).unwrap()).unwrap();
    assert_eq!((raw.kind.as_deref(), raw.remind.as_deref(), raw.skills.len()), (Some("Axolotl"), Some("wave"), 1), "written as it was, not tidied");
}

#[test]
fn updating_starts_a_file_when_there_is_none() {
    let dir = temp();
    update(&dir, |p| p.base = Some(Species::Fox)).unwrap();
    assert_eq!(read(&dir).base, Some(Species::Fox));
}

#[test]
fn updating_never_overwrites_a_file_with_a_mistake_in_it() {
    let dir = temp();
    std::fs::write(dir.join(FILE), "{ \"kind\": \"Axo").unwrap();
    assert!(update(&dir, |p| p.remind = Some("wave".into())).is_err());
    assert_eq!(std::fs::read_to_string(dir.join(FILE)).unwrap(), "{ \"kind\": \"Axo");
}

#[test]
fn a_remind_clip_must_be_a_safe_name() {
    let dir = temp();
    std::fs::write(dir.join(FILE), r#"{ "remind": "wave" }"#).unwrap();
    assert_eq!(read(&dir).remind.as_deref(), Some("wave"));
    std::fs::write(dir.join(FILE), r#"{ "remind": "../wave" }"#).unwrap();
    assert_eq!(read(&dir).remind, None);
}
