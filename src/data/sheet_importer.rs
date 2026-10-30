//! Imports a hand-drawn sprite sheet (one animation per row of equal cells) as a skin.
//! Frames are scaled smoothly rather than with hard pixel edges, so high-resolution art reads cleanly.
//! Every clip shares one crop box, so the cat keeps the same size and ground line whatever it is doing.

use std::path::Path;

use image::RgbaImage;

use super::clip_importer::{alpha_bounds, save_png, PixelBox};
use super::skin::{self, ClipMeta};

pub struct Row {
    pub action: String,
    pub rows: Vec<u32>,
    pub fps: f64,
    pub speed: f64,
}

/// `spec` lists clips as `action=row[+row…]@fps[/speed]`, comma-separated, e.g. `walk=9@10/30,sit=0@5`.
pub fn parse(spec: &str) -> Vec<Row> {
    spec.split(',')
        .filter_map(|part| {
            let (action, rest) = part.split_once('=')?;
            let mut rest = rest.split('@');
            let rows = rest.next().unwrap_or("").split('+').filter_map(|r| r.parse().ok()).collect();
            let tail: Vec<&str> = rest.next().map(|t| t.split('/').collect()).unwrap_or_default();
            Some(Row {
                action: action.to_string(),
                rows,
                fps: tail.first().and_then(|f| f.parse().ok()).unwrap_or(8.0),
                speed: tail.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            })
        })
        .collect()
}

pub fn import_sheet(path: &Path, skin: &str, spec: &[Row], credit: Option<&str>) -> Result<String, String> {
    const CELL: u32 = 128;
    const POINTS_PER_PIXEL: f64 = 0.625;
    let sheet = image::open(path).map_err(|_| "Couldn't read the sprite sheet.".to_string())?.to_rgba8();
    let cols = sheet.width() / CELL;
    let frame = |r: u32, c: u32| -> Option<RgbaImage> {
        if (r + 1) * CELL > sheet.height() { return None }
        Some(image::imageops::crop_imm(&sheet, c * CELL, r * CELL, CELL, CELL).to_image())
    };

    // Frames of each clip (empty cells skipped), and one crop box around everything the cat ever covers.
    let mut clips: Vec<(&Row, Vec<RgbaImage>)> = Vec::new();
    let mut union: Option<PixelBox> = None;
    for row in spec {
        let mut frames = Vec::new();
        for &r in &row.rows {
            for c in 0..cols {
                let Some(f) = frame(r, c) else { continue };
                let Some(b) = alpha_bounds(&f) else { continue };
                union = Some(union.map_or(b, |u| u.union(&b)));
                frames.push(f);
            }
        }
        if !frames.is_empty() { clips.push((row, frames)) }
    }
    let Some(union) = union.filter(|_| !clips.is_empty()) else { return Err("No frames found in this sheet.".into()) };

    let base = skin::root().join(skin);
    for (row, frames) in &clips {
        let dir = base.join(&row.action);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for (i, f) in frames.iter().enumerate() {
            let out = image::imageops::crop_imm(f, union.x, union.y, union.w, union.h).to_image();
            save_png(&out, &dir.join(format!("frame_{i:03}.png")))?;
        }
        let meta = ClipMeta {
            fps: row.fps,
            speed: row.speed,
            height: union.h as f64 * POINTS_PER_PIXEL,
            faces_right: true,
            pixel_art: Some(false),
            reach: None,
        };
        std::fs::write(dir.join("clip.json"), serde_json::to_vec(&meta).unwrap()).map_err(|e| e.to_string())?;
    }
    if let Some(credit) = credit {
        std::fs::write(base.join("credit.txt"), credit).map_err(|e| e.to_string())?;
    }
    let list: Vec<String> = clips.iter().map(|(r, f)| format!("{} ({})", r.action, f.len())).collect();
    Ok(format!("Imported {} clips: {}", clips.len(), list.join(", ")))
}

#[cfg(test)]
#[path = "sheet_importer_tests.rs"]
mod tests;
