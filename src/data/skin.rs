//! The clip library: real pet footage, cut out from its background and stored as frame sequences.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::domain::Activity::{self, *};
use super::pet_file::{self, PetFile};

/// One cut-out animation (walk, sit, sleep…) of a realistic pet. Frames are decoded by the renderer.
pub struct Clip {
    /// The clip's folder; the renderer caches decoded frames under it.
    pub dir: PathBuf,
    pub files: Vec<PathBuf>,
    pub fps: f64,
    /// Points per second the pet travels when this clip plays at normal speed, at size 1 (0 for stationary clips).
    pub speed: f64,
    /// Display height in points at size 1.
    pub height: f64,
    pub faces_right: bool,
    pub pixel_art: bool,
    /// How far ahead of the frame's centre the paws and face reach (the mouth, in an eat clip),
    /// as a fraction of the frame width (None: at the front edge).
    pub reach: Option<f64>,
}

impl Clip {
    pub fn count(&self) -> usize { self.files.len() }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipMeta {
    pub fps: f64,
    pub speed: f64,
    pub height: f64,
    pub faces_right: bool,
    /// Pixel art is scaled up with hard edges instead of being smoothed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixel_art: Option<bool>,
    /// Where the pet's front is, so it can touch balls and reach its bowl (see Clip::reach).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reach: Option<f64>,
}

/// A realistic pet: a folder of clips, one per action.
pub struct Skin {
    pub name: String,
    pub clips: BTreeMap<String, Clip>,
    /// Who made the art, shown in the pet's menu (from credit.txt in the skin folder).
    pub credit: Option<String>,
    /// What animal the art shows, what it says and which colours can change (from pet.json, or
    /// species.txt in older skins), so adopting it needn't ask.
    pub pet: PetFile,
}

/// Actions a clip can be imported as, with the pet's typical height (points, size 1) in that pose.
pub const ACTIONS: [(&str, &str, f64); 22] = [
    ("walk", "Walk", 62.0), ("run", "Run", 56.0), ("idle", "Stand / Idle", 62.0), ("sit", "Sit", 74.0), ("lie", "Lie Down", 40.0),
    ("sleep", "Sleep", 34.0), ("stretch", "Stretch", 52.0), ("groom", "Groom", 70.0), ("play", "Play / Pounce", 62.0),
    ("eat", "Eat", 54.0), ("jump", "Jump", 80.0), ("knead", "Make Biscuits", 42.0), ("meow", "Meow", 70.0),
    ("petted", "Being Petted", 60.0), ("hunt", "Stalk / Watch", 54.0), ("transform", "Transform", 70.0),
    ("firebreath", "Breathe Fire", 70.0), ("fly", "Fly", 60.0), ("grow", "Grow Giant", 80.0), ("herd", "Herd", 62.0),
    ("vanish", "Vanish", 70.0), ("stomp", "Stomp", 70.0),
];

/// Which clips to try for each activity, best first.
pub fn fallbacks(a: Activity) -> &'static [&'static str] {
    match a {
        Idle => &["idle", "sit", "lie"],
        Walk | Sniff => &["walk"],
        Run => &["run", "walk"],
        Sit => &["sit", "idle"],
        Lounge => &["lie", "sit"],
        Knead => &["knead", "lie", "sit"],
        Sleep => &["sleep", "lie"],
        Groom => &["groom", "sit"],
        Stretch => &["stretch", "idle", "sit"],
        Jump => &["jump", "play"],
        Pounce => &["play", "jump", "idle"],
        Eat => &["eat", "sit"],
        PlayBall => &["play", "idle"],
        ChaseButterfly => &["hunt", "play", "idle"],
        ChaseCursor => &["idle", "sit"],
        VisitCursor => &["sit", "lie", "idle"],
        Dance => &["play", "idle"],
        Wave | Meow => &["meow", "sit"],
        Peekaboo | Hide => &["sit", "idle"],
        Remind => &["sit", "idle"],
        PlayFriend => &["play", "run"],
        PlayFight => &["play", "hunt", "idle"],
        Transform => &["transform"],
        BreatheFire => &["firebreath"],
        Fly => &["fly"],
        Grow => &["grow"],
        Herd => &["herd"],
        Vanish => &["vanish"],
        Stomp => &["stomp"],
        // Its own clip, named in pet.json (see Skin::clip_for).
        Skill(_) => &[],
        Petted => &["petted", "lie", "sit"],
        Hop => &["jump", "play", "idle"],
        Dragged | Falling => &["jump", "idle", "sit"],
    }
}

/// Activities that need the pet to travel along the Dock.
fn travelling(a: Activity) -> bool {
    matches!(a, Walk | Sniff | Run | Eat | PlayBall | ChaseButterfly | ChaseCursor | PlayFriend | PlayFight | Herd)
}

impl Skin {
    /// The clip that best shows an activity, or None if this pet has no clip that fits it.
    pub fn clip_for(&self, a: Activity) -> Option<(&str, &Clip)> {
        let own = match a {
            Skill(i) => Some(&self.pet.skills.get(i as usize)?.clip),
            Remind => self.pet.remind.as_ref(),
            _ => None,
        };
        if let Some((n, c)) = own.and_then(|name| self.clips.get_key_value(name)) { return Some((n.as_str(), c)) }
        if let Skill(_) = a { return None }
        fallbacks(a).iter().find_map(|n| self.clips.get(*n).map(|c| (*n, c)))
    }

    /// What to call one of its clips in a menu: the action's title, its skill's name, or the folder name.
    pub fn clip_title(&self, clip: &str) -> String {
        if let Some(a) = ACTIONS.iter().find(|a| a.0 == clip) { return a.1.to_string() }
        self.pet.skills.iter().find(|k| k.clip == clip).map_or_else(|| clip.to_string(), |k| k.name.trim().to_string())
    }

    /// Its own skills (from pet.json) that it has the clip for, with their activities.
    pub fn skills(&self) -> impl Iterator<Item = (Activity, &pet_file::Skill)> {
        self.pet.skills.iter().enumerate().map(|(i, k)| (Skill(i as u8), k)).filter(|(a, _)| self.supports(*a))
    }

    fn can_travel(&self) -> bool { self.clips.contains_key("walk") || self.clips.contains_key("run") }

    /// Whether this pet can believably do an activity with the clips it has.
    pub fn supports(&self, a: Activity) -> bool {
        if matches!(a, Dragged | Falling | Idle) { return true }
        if travelling(a) && !self.can_travel() { return false }
        self.clip_for(a).is_some()
    }

    /// What to import so an unsupported activity starts working.
    pub fn missing_clip(&self, a: Activity) -> &str {
        if let Skill(i) = a { return self.pet.skills.get(i as usize).map_or("idle", |k| k.clip.as_str()) }
        if travelling(a) && !self.can_travel() { return "walk" }
        fallbacks(a).first().copied().unwrap_or("idle")
    }

    pub fn flip(&self) -> bool { super::settings::flipped(&self.name) }
}

/// Where skins live; on macOS this is the same folder the Swift build used.
pub fn root() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("PawPal").join("Skins")
}

pub fn load_all() -> BTreeMap<String, Rc<Skin>> {
    let root = root();
    let _ = std::fs::create_dir_all(&root);
    let mut out = BTreeMap::new();
    let Ok(dirs) = std::fs::read_dir(&root) else { return out };
    for dir in dirs.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        if let Some(skin) = load_skin(&dir) { out.insert(skin.name.clone(), Rc::new(skin)); }
    }
    out
}

/// One pet's folder: its clips (the pet.json's own ones too), credit and pet.json. None if it has no clips.
fn load_skin(dir: &Path) -> Option<Skin> {
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut skin = Skin { name, clips: BTreeMap::new(), credit: None, pet: pet_file::read(dir) };
    let own = skin.pet.skills.iter().map(|k| k.clip.as_str()).chain(skin.pet.remind.as_deref());
    let folders: Vec<String> = ACTIONS.iter().map(|a| a.0).chain(own).map(String::from).collect();
    for action in folders {
        if skin.clips.contains_key(&action) { continue }
        if let Some(clip) = load_clip(&dir.join(&action)) {
            skin.clips.insert(action, clip);
        }
    }
    skin.credit = std::fs::read_to_string(dir.join("credit.txt")).ok();
    (!skin.clips.is_empty()).then_some(skin)
}

fn load_clip(dir: &Path) -> Option<Clip> {
    let meta: ClipMeta = serde_json::from_slice(&std::fs::read(dir.join("clip.json")).ok()?).ok()?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect();
    files.sort_by_key(|p| p.file_name().map(|n| n.to_os_string()));
    if files.is_empty() { return None }
    Some(Clip {
        dir: dir.to_path_buf(),
        files,
        fps: meta.fps,
        speed: meta.speed,
        height: meta.height,
        faces_right: meta.faces_right,
        pixel_art: meta.pixel_art.unwrap_or(false),
        reach: meta.reach,
    })
}

#[cfg(test)]
#[path = "skin_tests.rs"]
mod tests;
