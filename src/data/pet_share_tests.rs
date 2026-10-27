//! Unit tests for data/pet_share.rs.

use super::*;

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pawpal-share-{tag}-{}", fastrand::u64(..)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A pet folder with a walk and a sit clip, a credit, and some files that aren't part of it.
fn pet_folder(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    for action in ["walk", "sit"] {
        std::fs::create_dir_all(dir.join(action)).unwrap();
        std::fs::write(dir.join(action).join("clip.json"), r#"{"fps":10,"speed":40,"height":60,"facesRight":true}"#).unwrap();
        for f in ["frame_000.png", "frame_001.png"] { std::fs::write(dir.join(action).join(f), action).unwrap() }
    }
    std::fs::write(dir.join("credit.txt"), "Filmed by me").unwrap();
    std::fs::write(dir.join("walk").join("source.mp4"), "video").unwrap();
    std::fs::create_dir_all(dir.join("notes")).unwrap();
    std::fs::write(dir.join("notes").join("todo.txt"), "x").unwrap();
    dir
}

#[test]
fn a_shared_pet_comes_back_exactly_as_it_was() {
    let (from, to) = (temp("from"), temp("to").join("Skins"));
    let file = from.join("Mochi.pawpal");
    export(&pet_folder(&from, "Mochi"), "Mochi", &file).unwrap();
    assert_eq!(import(&file, &to).unwrap(), "Mochi");
    let pet = to.join("Mochi");
    assert_eq!(std::fs::read_to_string(pet.join("sit/frame_001.png")).unwrap(), "sit");
    assert!(pet.join("walk/clip.json").is_file() && pet.join("credit.txt").is_file());
    assert!(!pet.join("walk/source.mp4").exists() && !pet.join("notes").exists(), "only the pet itself is shared");
    let leftovers: Vec<_> = std::fs::read_dir(to.parent().unwrap()).unwrap().flatten().filter(|e| e.file_name() != "Skins").collect();
    assert!(leftovers.is_empty(), "nothing left in the staging area");

    assert_eq!(import(&file, &to).unwrap(), "Mochi 2", "a second copy doesn't overwrite the first");
    assert_eq!(import(&file, &to).unwrap(), "Mochi 3");
    let _ = std::fs::remove_dir_all(from);
    let _ = std::fs::remove_dir_all(to.parent().unwrap());
}

#[test]
fn a_pet_without_clips_cant_be_shared() {
    let dir = temp("empty");
    std::fs::create_dir_all(dir.join("Nobody")).unwrap();
    assert!(export(&dir.join("Nobody"), "Nobody", &dir.join("x.pawpal")).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

/// Writes a zip with these entries, as someone else's tool might.
fn zip_with(path: &Path, entries: &[(&str, &str)]) {
    let mut z = ZipWriter::new(File::create(path).unwrap());
    for (name, body) in entries {
        z.start_file(*name, SimpleFileOptions::default()).unwrap();
        z.write_all(body.as_bytes()).unwrap();
    }
    z.finish().unwrap();
}

#[test]
fn files_outside_the_pet_are_never_written() {
    let dir = temp("evil");
    let root = dir.join("lib").join("Skins");
    let file = dir.join("evil.pawpal");
    zip_with(&file, &[
        ("pawpal.json", r#"{"name":"../../Evil","format":1}"#),
        ("../../escaped.png", "x"),
        ("walk/../../../escaped2.png", "x"),
        ("/abs.png", "x"),
        ("run/clip.json", "{}"),
        ("run/frame_000.png", "x"),
        ("run/run.sh", "x"),
        ("secret/clip.json", "{}"),
    ]);
    let name = import(&file, &root).unwrap();
    assert_eq!(name, "Evil", "slashes and dots are taken out of the name");
    assert!(root.join("Evil/run/frame_000.png").is_file());
    assert!(!root.join("Evil/run/run.sh").exists() && !root.join("Evil/secret").exists());
    assert!(!dir.join("escaped.png").exists() && !dir.join("lib/escaped.png").exists() && !dir.join("escaped2.png").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn other_files_are_refused() {
    let dir = temp("bad");
    let root = dir.join("Skins");
    std::fs::write(dir.join("plain.pawpal"), "not a zip").unwrap();
    assert!(import(&dir.join("plain.pawpal"), &root).is_err());
    zip_with(&dir.join("nomanifest.pawpal"), &[("walk/clip.json", "{}")]);
    assert!(import(&dir.join("nomanifest.pawpal"), &root).is_err());
    zip_with(&dir.join("noclips.pawpal"), &[("pawpal.json", r#"{"name":"A","format":1}"#), ("credit.txt", "x")]);
    assert!(import(&dir.join("noclips.pawpal"), &root).is_err());
    assert!(!root.join("A").exists(), "a failed import leaves nothing behind");
    assert!(std::fs::read_dir(&dir).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().starts_with(".incoming")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn names_are_safe_folder_names() {
    assert_eq!(clean_name(" Mo/chi: \\ "), "Mochi");
    assert_eq!(clean_name("..hidden"), "hidden");
    assert_eq!(clean_name("  "), "Shared Pet");
    assert_eq!(clean_name(&"x".repeat(100)).len(), 60);
}
