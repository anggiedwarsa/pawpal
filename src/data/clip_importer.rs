//! Turns a video, GIF, animated WebP or APNG into a cut-out clip.
//! Backgrounds are removed with the platform's subject lifting where there is one (Vision on macOS),
//! otherwise by flood-filling the plain background in from the frame's edges.

use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::{AnimationDecoder, Frame, RgbaImage};

use super::skin::{self, ClipMeta, ACTIONS};
use crate::support::clamp;

const MAX_FRAMES: usize = 72;
const VIDEO_FPS: f64 = 12.0;
const OUT_HEIGHT: f64 = 170.0;

/// A pixel rectangle in image coordinates (top-left origin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl PixelBox {
    pub fn max_x(&self) -> u32 { self.x + self.w }
    pub fn max_y(&self) -> u32 { self.y + self.h }
    pub fn mid_x(&self) -> f64 { self.x as f64 + self.w as f64 / 2.0 }
    pub fn union(&self, o: &PixelBox) -> PixelBox {
        let (x, y) = (self.x.min(o.x), self.y.min(o.y));
        PixelBox { x, y, w: self.max_x().max(o.max_x()) - x, h: self.max_y().max(o.max_y()) - y }
    }
}

pub fn import_clip(path: &Path, skin: &str, action: &str, progress: &dyn Fn(String)) -> Result<String, String> {
    progress("Reading frames…".into());
    let (frames, fps) = read_frames(path)?;
    if frames.is_empty() { return Err("No frames found in this file.".into()) }

    let total = frames.len();
    let mut cut: Vec<(RgbaImage, PixelBox)> = Vec::new();
    for (i, f) in frames.into_iter().enumerate() {
        progress(format!("Removing background {}/{total}", i + 1));
        let already_cut = alpha_bounds(&f).is_some_and(|b| b.w + 2 < f.width() || b.h + 2 < f.height());
        let img = if already_cut { Some(f) } else { lift_subject(&f) };
        if let Some(img) = img {
            if let Some(b) = alpha_bounds(&img).filter(|b| b.w > 8 && b.h > 8) {
                cut.push((img, b));
            }
        }
    }
    if cut.is_empty() { return Err("Couldn't find a cat in this clip. Try a clip with a plain background.".into()) }

    let union = cut.iter().skip(1).fold(cut[0].1, |u, c| u.union(&c.1));
    let max_w = cut.iter().map(|c| c.1.w).max().unwrap_or(1) as f64;
    let travel = cut.last().unwrap().1.mid_x() - cut[0].1.mid_x();
    let moving = travel.abs() > cut[0].0.width() as f64 * 0.15 && cut.len() > 4;
    let duration = cut.len() as f64 / fps;

    // Moving clips (a cat walking across the frame) are re-centred so the cat walks in place; PawPal moves it instead.
    let canvas_w = if moving { max_w } else { union.w as f64 };
    let canvas_h = union.h as f64;
    let scale = (OUT_HEIGHT / canvas_h).min(1.0);
    let (out_w, out_h) = ((canvas_w * scale).ceil() as u32, (canvas_h * scale).ceil() as u32);

    let dir = skin::root().join(skin).join(action);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let count = cut.len();
    for (i, (img, b)) in cut.iter().enumerate() {
        progress(format!("Saving frame {}/{}", i + 1, count));
        let crop_x = if moving { b.mid_x() - max_w / 2.0 } else { union.x as f64 };
        let mut canvas = RgbaImage::new(canvas_w.ceil() as u32, union.h);
        image::imageops::overlay(&mut canvas, img, -(crop_x.round() as i64), -(union.y as i64));
        let out = image::imageops::resize(&canvas, out_w.max(1), out_h.max(1), image::imageops::FilterType::Triangle);
        save_png(&out, &dir.join(format!("frame_{i:03}.png")))?;
    }

    let display_h = ACTIONS.iter().find(|a| a.0 == action).map_or(62.0, |a| a.2);
    let speed = if moving { travel.abs() / duration * (display_h / union.h as f64) } else { 0.0 };
    let meta = ClipMeta { fps, speed, height: display_h, faces_right: !moving || travel > 0.0, pixel_art: None, reach: None };
    std::fs::write(dir.join("clip.json"), serde_json::to_vec(&meta).unwrap()).map_err(|e| e.to_string())?;
    let walks = if moving { format!(" (walks in place, {} pt/s)", speed as i64) } else { String::new() };
    Ok(format!("Imported {count} frames as “{action}”{walks}"))
}

pub fn save_png(img: &RgbaImage, path: &Path) -> Result<(), String> {
    img.save_with_format(path, image::ImageFormat::Png).map_err(|e| format!("Couldn't save {}: {e}", path.display()))
}

/// Frames and their frame rate. Animated images are decoded directly; videos go through ffmpeg.
fn read_frames(path: &Path) -> Result<(Vec<RgbaImage>, f64), String> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if matches!(ext.as_str(), "gif" | "webp" | "png" | "apng") {
        let frames = decode_animation(path, &ext)?;
        let step = frames.len().div_ceil(MAX_FRAMES).max(1);
        let total: f64 = frames.iter().map(frame_delay).sum();
        let images: Vec<RgbaImage> = frames.into_iter().step_by(step).map(Frame::into_buffer).collect();
        let fps = if total > 0.0 { images.len() as f64 / total } else { 12.0 };
        return Ok((images, clamp(fps, 4.0, 30.0)));
    }
    read_video(path).map(|frames| (frames, VIDEO_FPS))
}

fn decode_animation(path: &Path, ext: &str) -> Result<Vec<Frame>, String> {
    let open = || File::open(path).map(BufReader::new).map_err(|e| e.to_string());
    let bad = |e: image::ImageError| format!("Couldn't read this file: {e}");
    match ext {
        "gif" => image::codecs::gif::GifDecoder::new(open()?).map_err(bad)?.into_frames().collect_frames().map_err(bad),
        "webp" => image::codecs::webp::WebPDecoder::new(open()?).map_err(bad)?.into_frames().collect_frames().map_err(bad),
        _ => {
            let png = image::codecs::png::PngDecoder::new(open()?).map_err(bad)?;
            if png.is_apng().map_err(bad)? {
                png.apng().map_err(bad)?.into_frames().collect_frames().map_err(bad)
            } else {
                Ok(vec![Frame::new(image::open(path).map_err(bad)?.to_rgba8())])
            }
        }
    }
}

fn frame_delay(f: &Frame) -> f64 {
    let (num, den) = f.delay().numer_denom_ms();
    let ms = if den == 0 { 0.0 } else { num as f64 / den as f64 };
    if ms > 0.0 { ms / 1000.0 } else { 0.08 }
}

/// Extracts up to MAX_FRAMES frames at VIDEO_FPS with ffmpeg, which reads every common video format on every platform.
fn read_video(path: &Path) -> Result<Vec<RgbaImage>, String> {
    let tmp = std::env::temp_dir().join(format!("pawpal-import-{}-{}", std::process::id(), fastrand::u64(..)));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-vf", &format!("fps={VIDEO_FPS},scale='min(1280,iw)':-2"), "-frames:v", &MAX_FRAMES.to_string()])
        .arg(tmp.join("f_%03d.png"))
        .status();
    let frames = match status {
        Err(_) => Err("Importing videos needs ffmpeg (https://ffmpeg.org). GIF, WebP and APNG clips work without it.".to_string()),
        Ok(s) if !s.success() => Err("This file isn't a video, GIF or WebP PawPal can read.".to_string()),
        Ok(_) => {
            let mut files: Vec<PathBuf> = std::fs::read_dir(&tmp).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).collect();
            files.sort();
            Ok(files.iter().filter_map(|f| image::open(f).ok().map(|i| i.to_rgba8())).collect())
        }
    };
    let _ = std::fs::remove_dir_all(&tmp);
    frames
}

/// Cuts the main subject out of a frame.
fn lift_subject(img: &RgbaImage) -> Option<RgbaImage> {
    match crate::platform::lift_subject(img) {
        Some(result) => result,
        None => remove_plain_background(img),
    }
}

/// Makes the background transparent by flood-filling from the edges through pixels close to the edge colour,
/// then keeps the largest remaining shape (the pet) and anything nearly as big (a tail cut off by a gap).
pub fn remove_plain_background(img: &RgbaImage) -> Option<RgbaImage> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w < 3 || h < 3 { return None }
    let px = |i: usize| img.as_raw()[i * 4..i * 4 + 3].iter().map(|&v| v as f64).collect::<Vec<f64>>();
    let border: Vec<usize> = (0..w).flat_map(|x| [x, (h - 1) * w + x]).chain((1..h - 1).flat_map(|y| [y * w, y * w + w - 1])).collect();

    // The background colour is the median of the edge pixels; the tolerance follows how noisy the edges are.
    let mut bg = [0.0; 3];
    for (c, slot) in bg.iter_mut().enumerate() {
        let mut v: Vec<f64> = border.iter().map(|&i| px(i)[c]).collect();
        v.sort_by(f64::total_cmp);
        *slot = v[v.len() / 2];
    }
    let dist: Vec<f64> = (0..w * h).map(|i| { let p = px(i); ((p[0] - bg[0]).powi(2) + (p[1] - bg[1]).powi(2) + (p[2] - bg[2]).powi(2)).sqrt() }).collect();
    let mut edge: Vec<f64> = border.iter().map(|&i| dist[i]).collect();
    edge.sort_by(f64::total_cmp);
    let thr = clamp(edge[edge.len() * 3 / 4] * 2.5 + 12.0, 20.0, 70.0);

    let mut is_bg = vec![false; w * h];
    let mut queue: VecDeque<usize> = border.iter().copied().filter(|&i| dist[i] < thr).collect();
    for &i in &queue { is_bg[i] = true }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        for n in neighbours(x, y, w, h) {
            if !is_bg[n] && dist[n] < thr { is_bg[n] = true; queue.push_back(n) }
        }
    }

    // Label the foreground shapes and keep the big ones.
    let mut label = vec![0u32; w * h];
    let mut sizes = vec![0usize];
    for start in 0..w * h {
        if is_bg[start] || label[start] != 0 { continue }
        let id = sizes.len() as u32;
        let mut n = 0;
        let mut q = VecDeque::from([start]);
        label[start] = id;
        while let Some(i) = q.pop_front() {
            n += 1;
            for j in neighbours(i % w, i / w, w, h) {
                if !is_bg[j] && label[j] == 0 { label[j] = id; q.push_back(j) }
            }
        }
        sizes.push(n);
    }
    let largest = sizes.iter().copied().max().unwrap_or(0);
    if largest < w * h / 200 || largest > w * h * 97 / 100 { return None }
    let keep: Vec<bool> = sizes.iter().map(|&n| n * 100 >= largest * 15).collect();

    let mut out = img.clone();
    for i in 0..w * h {
        let alpha = if is_bg[i] || !keep[label[i] as usize] {
            0.0
        } else if neighbours(i % w, i / w, w, h).any(|n| is_bg[n]) {
            // Soften the outline so it blends instead of looking cut with scissors.
            clamp((dist[i] - thr * 0.5) / (thr * 0.5), 0.0, 1.0) * 255.0
        } else {
            255.0
        };
        let a = &mut out.as_mut()[i * 4 + 3];
        *a = (*a as f64).min(alpha) as u8;
    }
    Some(out)
}

fn neighbours(x: usize, y: usize, w: usize, h: usize) -> impl Iterator<Item = usize> {
    let i = y * w + x;
    [(x > 0).then(|| i - 1), (x + 1 < w).then(|| i + 1), (y > 0).then(|| i - w), (y + 1 < h).then(|| i + w)].into_iter().flatten()
}

/// Bounding box of the visible pixels, or None if the image is fully transparent.
pub fn alpha_bounds(img: &RgbaImage) -> Option<PixelBox> {
    let (w, h) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0, 0);
    let mut any = false;
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 24 {
            any = true;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    any.then(|| PixelBox { x: min_x, y: min_y, w: max_x - min_x + 1, h: max_y - min_y + 1 })
}

#[cfg(test)]
#[path = "clip_importer_tests.rs"]
mod tests;
