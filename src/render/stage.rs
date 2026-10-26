//! Draws the world into the transparent overlay window with OpenGL (through femtovg).
//! Drawing uses stage points with y going up, like the Swift version's Core Animation layers.

use std::collections::HashMap;
use std::path::PathBuf;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight};
use femtovg::imgref::ImgRef;
use femtovg::renderer::OpenGl;
use femtovg::rgb::RGBA8;
use femtovg::{Canvas, Color, ImageFlags, ImageId, ImageSource, Paint, Path};

use crate::data::skin::{Clip, Skin};
use crate::domain::Activity::*;
use crate::domain::Pet;
use crate::engine::World;
use crate::support::*;

/// Keeps only the few most recently played clips decoded, to keep memory small.
const CLIP_CACHE_LIMIT: usize = 4;

#[derive(Clone, Copy)]
pub struct Img {
    pub id: ImageId,
    /// Size in stage points.
    pub w: f64,
    pub h: f64,
}

/// Which clip a pet is showing and how far into it.
#[derive(Default)]
struct Anim {
    clip: String,
    time: f64,
}

/// One pet, ready to draw.
struct PetDraw {
    frame: Img,
    rect: Rect,
    flip: bool,
    sink: f64,
    shadow: Option<(Rect, f32)>,
}

pub struct Stage {
    pub canvas: Canvas<OpenGl>,
    fonts: FontSystem,
    swash: SwashCache,
    text_cache: HashMap<String, Img>,
    clips: HashMap<PathBuf, Vec<Option<Img>>>,
    recent: Vec<PathBuf>,
    anims: HashMap<u64, Anim>,
    /// Physical pixels per point.
    pub dpi: f64,
    /// Drawable size in points.
    pub width: f64,
    pub height: f64,
}

impl Stage {
    pub fn new(canvas: Canvas<OpenGl>) -> Stage {
        Stage {
            canvas,
            fonts: FontSystem::new(),
            swash: SwashCache::new(),
            text_cache: HashMap::new(),
            clips: HashMap::new(),
            recent: Vec::new(),
            anims: HashMap::new(),
            dpi: 1.0,
            width: 1.0,
            height: 1.0,
        }
    }

    pub fn resize(&mut self, px_w: u32, px_h: u32, dpi: f64) {
        if dpi != self.dpi {
            self.drop_text_cache();
        }
        self.dpi = dpi;
        self.width = px_w as f64 / dpi;
        self.height = px_h as f64 / dpi;
        self.canvas.set_size(px_w, px_h, dpi as f32);
    }

    pub fn forget(&mut self, pet_id: u64) { self.anims.remove(&pet_id); }

    /// Forgets decoded frames, so re-imported clips are read again.
    pub fn reload_clips(&mut self) {
        for (_, frames) in self.clips.drain() {
            for f in frames.into_iter().flatten() {
                self.canvas.delete_image(f.id);
            }
        }
        self.recent.clear();
    }

    /// Animates every pet's clip (updating its size and reach), then draws the frame.
    pub fn render(&mut self, world: &mut World, dt: f64) {
        let mut pets = Vec::with_capacity(world.pets.len());
        let ids: Vec<u64> = world.pets.iter().map(|p| p.id).collect();
        self.anims.retain(|id, _| ids.contains(id));
        for i in 0..world.pets.len() {
            let ground = world.ground;
            let s = world.s;
            if let Some(d) = self.sync(&mut world.pets[i], ground, s, dt) {
                pets.push(d);
            }
        }

        let (w, h) = ((self.width * self.dpi) as u32, (self.height * self.dpi) as u32);
        self.canvas.clear_rect(0, 0, w, h, Color::rgbaf(0.0, 0.0, 0.0, 0.0));
        self.canvas.reset_transform();
        self.canvas.scale(self.dpi as f32, self.dpi as f32);
        self.canvas.translate(0.0, self.height as f32);
        self.canvas.scale(1.0, -1.0);

        // Toys, particles and speech bubbles sit behind the pets, as they did in the Swift version's view.
        for b in &world.balls { self.draw_ball(b, world) }
        for b in &world.butterflies { self.draw_butterfly(b, world) }
        for q in &world.particles {
            let Some(img) = self.text_image(&q.text, q.size, 700, Rgba::white(0.35, 1.0)) else { continue };
            let alpha = (q.ttl / (q.life * 0.5)).min(1.0) as f32;
            self.draw_image(img, q.p.x - img.w / 2.0, q.p.y - img.h / 2.0, alpha, false);
        }
        for p in &world.pets {
            if let (Some(text), true) = (&p.bubble, p.sink < 0.5) {
                self.draw_bubble(text, pt(p.x, world.bubble_y(p)));
            }
        }

        for d in &pets {
            if let Some((r, alpha)) = d.shadow {
                let mut path = Path::new();
                path.rounded_rect(r.x as f32, r.y as f32, r.w as f32, r.h as f32, (r.h / 2.0) as f32);
                self.canvas.fill_path(&path, &Paint::color(Color::rgbaf(0.0, 0.0, 0.0, alpha)));
            }
            // Ducking into the Dock (peekaboo) slides the clip down inside a box that clips at the Dock's top edge.
            self.canvas.save();
            if d.sink > 0.0 {
                self.canvas.scissor(d.rect.x as f32, d.rect.y as f32, d.rect.w as f32, d.rect.h as f32);
            }
            let img = Img { w: d.rect.w, h: d.rect.h, ..d.frame };
            self.draw_image(img, d.rect.x, d.rect.y - d.sink * d.rect.h, 1.0, d.flip);
            self.canvas.restore();
        }

        // Bowls are drawn in front of the pets, so an eating pet's face dips into its bowl.
        for b in &world.bowls { self.draw_bowl(b, world) }
        self.canvas.flush();
    }

    fn sync(&mut self, p: &mut Pet, ground: f64, s: f64, dt: f64) -> Option<PetDraw> {
        let skin = p.skin.clone()?;
        let (name, clip, rate) = choose(p, &skin, s)?;
        let anim = self.anims.entry(p.id).or_default();
        if anim.clip != name {
            anim.clip = name.to_string();
            anim.time = 0.0;
        }
        anim.time += dt * rate;
        let idx = match jump_progress(p, s).filter(|_| name == "jump") {
            Some(k) => ((k * clip.count() as f64) as usize).min(clip.count().max(1) - 1),
            None => (anim.time * clip.fps) as usize % clip.count().max(1),
        };
        let frame = self.frame(clip, idx)?;

        let h = clip.height * s;
        let w = h * frame.w / frame.h;
        p.size = Size { width: w, height: h };
        p.reach = clip.reach.unwrap_or(0.5) * w;

        let lift = ((p.y - p.floor) / (150.0 * s)).min(0.6);
        let sw = w * 0.6 * (1.0 - lift);
        let shadow = (p.sink <= 0.05).then(|| (Rect::new(p.x - sw / 2.0, ground + p.floor - 2.0 * s, sw, 5.0 * s), (0.16 * (1.0 - lift)) as f32));
        Some(PetDraw {
            frame,
            rect: Rect::new(p.x - w / 2.0, ground + p.y - 1.0, w, h),
            flip: ((p.facing > 0.0) != clip.faces_right) != skin.flip(),
            sink: p.sink,
            shadow,
        })
    }

    /// Frames are decoded only while the clip is in use; clips that haven't played recently are dropped.
    fn frame(&mut self, clip: &Clip, i: usize) -> Option<Img> {
        if !self.clips.contains_key(&clip.dir) {
            let flags = if clip.pixel_art { ImageFlags::NEAREST } else { ImageFlags::empty() };
            let frames = clip.files.iter().map(|f| self.load_image(f, flags)).collect();
            self.clips.insert(clip.dir.clone(), frames);
        }
        if self.recent.last() != Some(&clip.dir) {
            self.recent.retain(|d| d != &clip.dir);
            self.recent.push(clip.dir.clone());
            while self.recent.len() > CLIP_CACHE_LIMIT {
                let old = self.recent.remove(0);
                for f in self.clips.remove(&old).into_iter().flatten().flatten() {
                    self.canvas.delete_image(f.id);
                }
            }
        }
        let frames = self.clips.get(&clip.dir)?;
        if frames.is_empty() { return None }
        frames[i % frames.len()]
    }

    fn load_image(&mut self, file: &std::path::Path, flags: ImageFlags) -> Option<Img> {
        let img = image::open(file).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        let px: Vec<RGBA8> = img.pixels().map(|p| RGBA8::new(p[0], p[1], p[2], p[3])).collect();
        let id = self.canvas.create_image(ImageSource::Rgba(ImgRef::new(&px, w as usize, h as usize)), flags).ok()?;
        Some(Img { id, w: w as f64, h: h as f64 })
    }

    /// Draws an image with its bottom-left corner at (x, y), optionally mirrored left to right.
    pub fn draw_image(&mut self, img: Img, x: f64, y: f64, alpha: f32, flip: bool) {
        let (w, h) = (img.w as f32, img.h as f32);
        self.canvas.save();
        self.canvas.translate(x as f32, (y + img.h) as f32);
        self.canvas.scale(1.0, -1.0);
        if flip {
            self.canvas.translate(w, 0.0);
            self.canvas.scale(-1.0, 1.0);
        }
        let mut path = Path::new();
        path.rect(0.0, 0.0, w, h);
        self.canvas.fill_path(&path, &Paint::image(img.id, 0.0, 0.0, w, h, 0.0, alpha));
        self.canvas.restore();
    }

    fn drop_text_cache(&mut self) {
        for (_, img) in self.text_cache.drain() {
            self.canvas.delete_image(img.id);
        }
    }

    /// Text rendering is costly, so each string is rasterised once and reused.
    pub fn text_image(&mut self, text: &str, size: f64, weight: u16, color: Rgba) -> Option<Img> {
        let key = format!("{text}|{size}|{weight}");
        if let Some(img) = self.text_cache.get(&key) { return Some(*img) }
        if self.text_cache.len() > 200 { self.drop_text_cache() }

        let px = (size * self.dpi) as f32;
        let mut buf = Buffer::new(&mut self.fonts, Metrics::new(px, px * 1.2));
        buf.set_size(None, None);
        buf.set_text(text, &Attrs::new().family(Family::SansSerif).weight(Weight(weight)), Shaping::Advanced, None);
        buf.shape_until_scroll(&mut self.fonts, false);
        let w = buf.layout_runs().map(|r| r.line_w).fold(0.0, f32::max).ceil().max(1.0) as usize;
        let h = (buf.layout_runs().count().max(1) as f32 * px * 1.2).ceil() as usize;

        // Glyphs arrive as coverage-weighted colours; blend them into premultiplied pixels.
        let mut out = vec![RGBA8::new(0, 0, 0, 0); w * h];
        let ink = cosmic_text::Color::rgba((color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, 255);
        buf.draw(&mut self.fonts, &mut self.swash, ink, |x, y, gw, gh, c| {
            for yy in y.max(0)..(y + gh as i32).min(h as i32) {
                for xx in x.max(0)..(x + gw as i32).min(w as i32) {
                    let d = &mut out[yy as usize * w + xx as usize];
                    let a = c.a() as u32;
                    let blend = |s: u8, d: u8| ((s as u32 * a + d as u32 * (255 - a)) / 255) as u8;
                    *d = RGBA8::new(blend(c.r(), d.r), blend(c.g(), d.g), blend(c.b(), d.b), (a + d.a as u32 * (255 - a) / 255) as u8);
                }
            }
        });
        let id = self.canvas.create_image(ImageSource::Rgba(ImgRef::new(&out, w, h)), ImageFlags::PREMULTIPLIED).ok()?;
        let img = Img { id, w: w as f64 / self.dpi, h: h as f64 / self.dpi };
        self.text_cache.insert(key, img);
        Some(img)
    }
}

/// How far through its jump a pet in the air is (0 take-off, 0.5 the top, 1 landing), so the jump clip
/// matches the motion: a hop down shows only the falling half, not a leap upwards.
fn jump_progress(p: &Pet, s: f64) -> Option<f64> {
    if p.leaping {
        let down = p.leap_to.y < p.leap_from.y - 1.0;
        return Some(if down { 0.5 + 0.5 * p.leap_k } else { p.leap_k });
    }
    (!p.on_ground()).then(|| clamp(0.5 - p.vy / (760.0 * s), 0.0, 0.999))
}

/// Picks the clip that fits what the pet is doing; walk and run clips play faster or slower to match its speed.
fn choose<'a>(p: &Pet, skin: &'a Skin, s: f64) -> Option<(&'a str, &'a Clip, f64)> {
    let speed = p.vx.abs();
    if (!p.on_ground() || p.leaping) && !matches!(p.activity, Dragged | VisitCursor) {
        if let Some(c) = skin.clips.get("jump") { return Some(("jump", c, 1.0)) }
    }
    if p.on_ground() && speed > 1.0 && p.activity != Falling {
        let order = if speed > 110.0 * s { ["run", "walk"] } else { ["walk", "run"] };
        for n in order {
            if let Some(c) = skin.clips.get(n) {
                let rate = if c.speed > 0.0 { clamp(speed / (c.speed * s), 0.4, 3.0) } else { 1.0 };
                return Some((n, c, rate));
            }
        }
    }
    if let Some((n, c)) = skin.clip_for(p.activity).or_else(|| skin.clip_for(Idle)) { return Some((n, c, 1.0)) }
    skin.clips.iter().next().map(|(n, c)| (n.as_str(), c, 1.0))
}

#[cfg(test)]
#[path = "stage_tests.rs"]
mod tests;
