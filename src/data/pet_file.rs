//! A skin's `pet.json`: what kind of animal it shows, what it says and which colours can change.
//!
//! ```json
//! {
//!   "kind": "Axolotl", "base": "cat",
//!   "lines": { "hello": "Blub! 👋" },
//!   "recolor": { "hue": 330 },
//!   "skills": [{ "name": "🫧 Blow Bubbles", "clip": "bubbles", "seconds": 4, "say": "Blub blub~", "burst": "🫧" }],
//!   "remind": "bubbles"
//! }
//! ```
//!
//! Every field is optional. `base` is the built-in species it acts like and borrows any missing
//! lines from; older skins name it in `species.txt` instead, which is still read. Each skill plays
//! a clip folder of the skin's own, imported like any other clip.

use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};

use crate::domain::{Lines, Recolor, Species, Voice};

pub const FILE: &str = "pet.json";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PetFile {
    /// What to call this kind of animal ("Axolotl"), when it isn't one of the built-in species.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The built-in species it acts like; a name that isn't one counts as left out.
    #[serde(default, deserialize_with = "lenient_species", skip_serializing_if = "Option::is_none")]
    pub base: Option<Species>,
    #[serde(default, skip_serializing_if = "Voice::is_empty")]
    pub lines: Voice,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recolor: Option<Recolor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<Skill>,
    /// The clip it plays while telling the user about a reminder ("wave", or a skill's clip); left
    /// out, it sits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remind: Option<String>,
}

/// A skill of the pet's own: a clip it plays for a while, with something to say and a flourish.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    /// The name in the Do Activity menu, emoji and all ("🫧 Blow Bubbles").
    pub name: String,
    /// The skin's clip folder that shows it ("bubbles"): letters, digits, `-` and `_` only.
    pub clip: String,
    /// How long it lasts, in seconds.
    #[serde(default = "default_seconds")]
    pub seconds: f64,
    /// What it says as it starts: one line, or a list to pick from.
    #[serde(default)]
    pub say: Lines,
    /// An emoji that bursts out around it as it starts ("🫧").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst: Option<String>,
    /// Whether other pets nearby on its level jump in surprise.
    #[serde(default)]
    pub startle: bool,
    /// How often it does this on its own, next to walking at 5 (0: only when picked from the menu).
    #[serde(default = "default_often")]
    pub often: f64,
}

fn default_seconds() -> f64 { 4.0 }
fn default_often() -> f64 { 0.5 }

/// The most skills a pet can have (they're numbered in a byte).
pub const MAX_SKILLS: usize = 32;

/// Whether a name is safe as a clip folder: no paths, nothing hidden.
pub fn is_clip_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 40 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn lenient_species<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Species>, D::Error> {
    Ok(Option::<String>::deserialize(d)?.and_then(|s| Species::parse(&s)))
}

/// Reads a skin folder's `pet.json`, falling back to `species.txt` for the base species. A file that
/// can't be read or parsed counts as empty, so a typo never stops the pet loading.
pub fn read(dir: &Path) -> PetFile {
    let mut pet: PetFile = std::fs::read(dir.join(FILE)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    if pet.base.is_none() {
        pet.base = std::fs::read_to_string(dir.join("species.txt")).ok().and_then(|s| Species::parse(&s));
    }
    pet.remind = pet.remind.filter(|c| is_clip_name(c));
    // Skills that couldn't work are dropped, and the rest kept within sensible bounds.
    pet.skills.retain(|k| is_clip_name(&k.clip) && !k.name.trim().is_empty());
    pet.skills.truncate(MAX_SKILLS);
    for k in &mut pet.skills {
        k.seconds = if k.seconds.is_finite() { k.seconds.clamp(0.5, 60.0) } else { default_seconds() };
        k.often = if k.often.is_finite() { k.often.clamp(0.0, 5.0) } else { 0.0 };
    }
    pet
}

/// Reads the answer to "what kind of animal is it?": `Ok` for a built-in species, `Err` with the name
/// of a new kind for anything else, or None if it was left blank.
pub fn kind_from(answer: &str) -> Option<Result<Species, String>> {
    let answer = answer.trim();
    if answer.is_empty() { return None }
    Some(Species::parse(answer).ok_or_else(|| answer.to_string()))
}

/// Changes a skin's pet.json, keeping everything else in it as written. Refuses if the file is there
/// but can't be read, so a mistake the user is still fixing is never overwritten.
pub fn update(dir: &Path, change: impl FnOnce(&mut PetFile)) -> Result<(), String> {
    let path = dir.join(FILE);
    let mut pet = match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("pet.json has a mistake in it: {e}"))?,
        Err(_) => PetFile::default(),
    };
    change(&mut pet);
    write(dir, &pet)
}

pub fn write(dir: &Path, pet: &PetFile) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(pet).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(FILE), json).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "pet_file_tests.rs"]
mod tests;
