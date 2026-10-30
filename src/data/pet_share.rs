//! Sharing pets: a pet's clips packed into one `.pawpal` file (a zip) that a friend can import.
//!
//! Inside: `pawpal.json` naming the pet, `credit.txt`, `species.txt` and `pet.json` if it has them, and
//! `<action>/clip.json` plus `<action>/*.png` for each clip (its own skills' too), laid out exactly as in
//! the clip library.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::pet_file;
use super::skin::ACTIONS;

pub const EXTENSION: &str = "pawpal";
/// Largest shared pet accepted, unpacked, so a bad file can't fill the disk.
const MAX_TOTAL: u64 = 512 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Manifest {
    name: String,
    format: u32,
}

/// Packs the pet in `skin_dir` into a `.pawpal` file at `out`.
pub fn export(skin_dir: &Path, name: &str, out: &Path) -> Result<(), String> {
    let fail = |e: &dyn std::fmt::Display| format!("Couldn't save the pet: {e}");
    let mut zip = ZipWriter::new(File::create(out).map_err(|e| fail(&e))?);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let manifest = serde_json::to_vec_pretty(&Manifest { name: name.to_string(), format: 1 }).map_err(|e| fail(&e))?;
    zip.start_file("pawpal.json", opts).map_err(|e| fail(&e))?;
    zip.write_all(&manifest).map_err(|e| fail(&e))?;
    let mut files: Vec<String> = Vec::new();
    for extra in ["credit.txt", "species.txt", pet_file::FILE] {
        if skin_dir.join(extra).is_file() { files.push(extra.into()) }
    }
    let own: Vec<String> = pet_file::read(skin_dir).skills.into_iter().map(|k| k.clip).collect();
    let mut folders: Vec<&str> = ACTIONS.iter().map(|a| a.0).collect();
    for clip in &own {
        if !folders.contains(&clip.as_str()) { folders.push(clip) }
    }
    for action in folders {
        let Ok(entries) = std::fs::read_dir(skin_dir.join(action)) else { continue };
        let mut names: Vec<String> = entries.flatten().filter_map(|e| e.file_name().to_str().map(String::from)).filter(|n| allowed_in_clip(n)).collect();
        names.sort();
        files.extend(names.into_iter().map(|n| format!("{action}/{n}")));
    }
    if !files.iter().any(|f| f.ends_with("/clip.json")) { return Err(format!("{name} has no clips to share yet.")) }
    for f in files {
        let bytes = std::fs::read(skin_dir.join(&f)).map_err(|e| fail(&e))?;
        zip.start_file(f, opts).map_err(|e| fail(&e))?;
        zip.write_all(&bytes).map_err(|e| fail(&e))?;
    }
    zip.finish().map_err(|e| fail(&e))?;
    Ok(())
}

/// Unpacks a `.pawpal` file into the clip library at `root`, under the pet's name (or "Name 2" and so on
/// when that's taken). Returns the name it got.
pub fn import(file: &Path, root: &Path) -> Result<String, String> {
    let not_a_pet = || "This isn't a PawPal pet file.".to_string();
    let mut zip = ZipArchive::new(File::open(file).map_err(|e| format!("Couldn't open the file: {e}"))?).map_err(|_| not_a_pet())?;
    let manifest: Manifest = {
        let mut m = zip.by_name("pawpal.json").map_err(|_| not_a_pet())?;
        let mut text = String::new();
        m.by_ref().take(64 * 1024).read_to_string(&mut text).map_err(|_| not_a_pet())?;
        serde_json::from_str(&text).map_err(|_| not_a_pet())?
    };
    let name = unique_name(root, &clean_name(&manifest.name));
    // The pet's own skills say which other clip folders belong to it.
    let own: Vec<String> = match zip.by_name(pet_file::FILE) {
        Ok(f) => {
            let mut text = String::new();
            let _ = f.take(256 * 1024).read_to_string(&mut text);
            serde_json::from_str::<pet_file::PetFile>(&text).map(|p| p.skills.into_iter().map(|k| k.clip).collect()).unwrap_or_default()
        }
        Err(_) => Vec::new(),
    };

    // Unpacked next to the library first, and moved in only once complete, so a bad file leaves nothing behind.
    let staging = root.parent().unwrap_or(root).join(format!(".incoming-{}", fastrand::u64(..)));
    let result = unpack(&mut zip, &staging, &own);
    let result = result.and_then(|()| {
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        std::fs::rename(&staging, root.join(&name)).map_err(|e| e.to_string())
    });
    if result.is_err() { let _ = std::fs::remove_dir_all(&staging); }
    result.map(|()| name)
}

fn unpack(zip: &mut ZipArchive<File>, into: &Path, own: &[String]) -> Result<(), String> {
    let mut total = 0u64;
    let mut clips = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("The file is damaged: {e}"))?;
        if entry.is_dir() { continue }
        let Some(path) = entry.enclosed_name().and_then(|p| place(&p, own)) else { continue };
        total += entry.size();
        if total > MAX_TOTAL { return Err("This pet file is too big.".into()) }
        let dest = into.join(&path);
        if let Some(dir) = dest.parent() { std::fs::create_dir_all(dir).map_err(|e| e.to_string())? }
        let mut out = File::create(&dest).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry.by_ref().take(MAX_TOTAL), &mut out).map_err(|e| format!("The file is damaged: {e}"))?;
        if path.ends_with("clip.json") { clips += 1 }
    }
    if clips == 0 { return Err("This pet file has no clips in it.".into()) }
    Ok(())
}

/// Where an entry goes inside the pet's folder, or None for anything that isn't part of a pet:
/// only `credit.txt`, `species.txt`, `pet.json` and the frames and settings of clips (built-in actions, or
/// the `own` clips of its skills, if their folder names are safe), never a path leading elsewhere.
fn place(p: &Path, own: &[String]) -> Option<PathBuf> {
    let parts: Vec<&str> = p.components().map(|c| match c { Component::Normal(s) => s.to_str(), _ => None }).collect::<Option<_>>()?;
    match parts.as_slice() {
        [f @ ("credit.txt" | "species.txt" | pet_file::FILE)] => Some(PathBuf::from(f)),
        [action, file] if allowed_in_clip(file) && (ACTIONS.iter().any(|a| a.0 == *action) || (pet_file::is_clip_name(action) && own.iter().any(|o| o == action))) => {
            Some(Path::new(action).join(file))
        }
        _ => None,
    }
}

fn allowed_in_clip(file: &str) -> bool { file == "clip.json" || (file.ends_with(".png") && !file.starts_with('.')) }

/// A pet name that's safe as a folder name.
pub fn clean_name(name: &str) -> String {
    let n: String = name.chars().filter(|c| !matches!(c, '/' | ':' | '\\') && !c.is_control()).collect();
    let n = n.trim().trim_start_matches('.').trim();
    if n.is_empty() { "Shared Pet".into() } else { n.chars().take(60).collect() }
}

/// `name`, or "name 2", "name 3"… if a pet by that name is already in the library.
fn unique_name(root: &Path, name: &str) -> String {
    if !root.join(name).exists() { return name.to_string() }
    (2..).map(|n| format!("{name} {n}")).find(|n| !root.join(n).exists()).unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
#[path = "pet_share_tests.rs"]
mod tests;
