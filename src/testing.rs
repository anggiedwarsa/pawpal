//! Builders shared by the unit tests.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::data::pet_file::PetFile;
use crate::data::skin::{Clip, Skin};
use crate::domain::{Pet, Species};
use crate::engine::World;

pub fn clip(name: &str) -> Clip {
    Clip {
        dir: PathBuf::from(format!("/test/{name}")),
        files: vec![PathBuf::from("frame_000.png"), PathBuf::from("frame_001.png")],
        fps: 10.0,
        speed: 40.0,
        height: 60.0,
        faces_right: true,
        pixel_art: false,
        reach: None,
    }
}

pub fn skin(clips: &[&str]) -> Rc<Skin> { skin_with(clips, PetFile::default()) }

/// A skin with these clips and this pet.json.
pub fn skin_with(clips: &[&str], pet: PetFile) -> Rc<Skin> {
    let clips: BTreeMap<String, Clip> = clips.iter().map(|n| (n.to_string(), clip(n))).collect();
    Rc::new(Skin { name: "Test".into(), clips, credit: None, pet })
}

/// A pet.json from its JSON.
pub fn pet_file(json: &str) -> PetFile { serde_json::from_str(json).unwrap() }

/// A skin with every clip, so a pet can do anything.
pub fn full_skin() -> Rc<Skin> {
    skin(&["walk", "run", "idle", "sit", "lie", "sleep", "stretch", "groom", "play", "eat", "jump", "knead", "meow", "petted", "hunt"])
}

/// A 1000×600 world with a 900-point Dock from x = 50, holding `n` pets that can do anything.
pub fn world(n: usize) -> World {
    let mut w = World::new();
    w.min_x = 50.0;
    w.max_x = 950.0;
    w.width = 1000.0;
    w.height = 600.0;
    w.s = 1.0;
    for i in 0..n {
        let mut p = Pet::new(i as u64 + 1, format!("Pet {i}"), Species::Cat, Some(full_skin()));
        p.x = 200.0 + 300.0 * i as f64;
        w.pets.push(p);
    }
    w
}
