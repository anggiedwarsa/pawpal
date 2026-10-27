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
    /// Rotation about the frame's centre (radians, counterclockwise on screen).
    tilt: f64,
    shadow: Option<(Rect, f32)>,
}

pub struct Stage {
    pub canvas: Canvas<OpenGl>,
    fonts: FontSystem,
    swash: SwashCache,
    text_cache: HashMap<String, Img>,
    /// Decoded frames per clip folder, with the pixel height they were scaled to.
    clips: HashMap<PathBuf, (u32, Vec<Option<Img>>)>,
    recent: Vec<PathBuf>,
    anims: HashMap<u64, Anim>,
    /// Physical pixels per point.
    pub dpi: f64,
    /// Drawable size in points.
    pub width: f64,
    pub height: f64,
    /// What the last drawn frame showed; an identical frame isn't drawn again.
    last_scene: String,
    /// The pets as `prepare` left them, for the next `render`.
    prepared: Vec<PetDraw>,
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
            last_scene: String::new(),
            prepared: Vec::new(),
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
        self.last_scene.clear();
    }

    /// Makes the next render draw even if nothing changed (the window was exposed or resized).
    pub fn invalidate(&mut self) { self.last_scene.clear() }

    pub fn forget(&mut self, pet_id: u64) { self.anims.remove(&pet_id); }

    /// Forgets decoded frames, so re-imported clips are read again.
    pub fn reload_clips(&mut self) {
        for (_, (_, frames)) in self.clips.drain() {
            for f in frames.into_iter().flatten() {
                self.canvas.delete_image(f.id);
            }
        }
        self.recent.clear();
    }

    /// Animates every pet's clip (updating its size and reach) and gets ready everything the next `render`
    /// needs: decoding a clip's frames and rasterising text are the slow part of a frame, and are done here.
    pub fn prepare(&mut self, world: &mut World, dt: f64) {
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
        self.prepared = pets;
        for q in &world.particles { self.particle_image(&q.text, q.size); }
        for p in &world.pets {
            if let (Some(text), true) = (&p.bubble, p.sink < 0.5) { self.bubble_image(text); }
        }
    }

    /// Draws the frame `prepare` got ready.
    /// Returns false, drawing nothing, when the frame would look exactly like the last one.
    pub fn render(&mut self, world: &World) -> bool {
        let pets = std::mem::take(&mut self.prepared);

        // Moving toys and particles change every frame; otherwise compare what would be drawn.
        if world.balls.is_empty() && world.butterflies.is_empty() && world.particles.is_empty() {
            let scene = scene_key(&pets, world);
            if scene == self.last_scene { return false }
            self.last_scene = scene;
        } else {
            self.last_scene.clear();
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
            let Some(img) = self.particle_image(&q.text, q.size) else { continue };
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
            if d.tilt != 0.0 {
                let (cx, cy) = ((d.rect.x + d.rect.w / 2.0) as f32, (d.rect.y + d.rect.h / 2.0) as f32);
                self.canvas.translate(cx, cy);
                self.canvas.rotate(d.tilt as f32);
                self.canvas.translate(-cx, -cy);
            }
            let img = Img { w: d.rect.w, h: d.rect.h, ..d.frame };
            self.draw_image(img, d.rect.x, d.rect.y - d.sink * d.rect.h, 1.0, d.flip);
            self.canvas.restore();
        }

        // Bowls are drawn in front of the pets, so an eating pet's face dips into its bowl.
        for b in &world.bowls { self.draw_bowl(b, world) }
        self.canvas.flush();
        true
    }

    fn particle_image(&mut self, text: &str, size: f64) -> Option<Img> { self.text_image(text, size, 700, Rgba::white(0.35, 1.0)) }

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
        let h = clip.height * s;
        let frame = self.frame(clip, idx, (h * self.dpi).ceil() as u32)?;

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
            // Clockwise on screen tips the nose of a pet facing right downwards.
            tilt: if name == "jump" && !clip.pixel_art { -fall_tilt(p, s) * p.facing.signum() } else { 0.0 },
            shadow,
        })
    }

    /// Frames are decoded only while the clip is in use; clips that haven't played recently are dropped.
    /// They're kept at the `px` pixel height they're drawn at, not the (often much bigger) source size.
    fn frame(&mut self, clip: &Clip, i: usize, px: u32) -> Option<Img> {
        if self.clips.get(&clip.dir).is_none_or(|(at, _)| *at != px) {
            for f in self.clips.remove(&clip.dir).into_iter().flat_map(|(_, f)| f).flatten() {
                self.canvas.delete_image(f.id);
            }
            let frames = clip.files.iter().map(|f| self.load_image(f, clip.pixel_art, px)).collect();
            self.clips.insert(clip.dir.clone(), (px, frames));
        }
        if self.recent.last() != Some(&clip.dir) {
            self.recent.retain(|d| d != &clip.dir);
            self.recent.push(clip.dir.clone());
            while self.recent.len() > CLIP_CACHE_LIMIT {
                let old = self.recent.remove(0);
                for f in self.clips.remove(&old).into_iter().flat_map(|(_, f)| f).flatten() {
                    self.canvas.delete_image(f.id);
                }
            }
        }
        let (_, frames) = self.clips.get(&clip.dir)?;
        if frames.is_empty() { return None }
        frames[i % frames.len()]
    }

    /// Loads a frame scaled down to `px` pixels high (pixel art stays as it is and is scaled up with hard edges).
    /// `w` and `h` keep the source size, so the pet's proportions don't drift with rounding.
    fn load_image(&mut self, file: &std::path::Path, pixel_art: bool, px: u32) -> Option<Img> {
        let mut img = image::open(file).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        // Premultiplied, so the transparent background doesn't bleed dark edges in when scaling down.
        for p in img.pixels_mut() {
            let a = p[3] as u32;
            for c in &mut p.0[..3] { *c = (*c as u32 * a / 255) as u8 }
        }
        if !pixel_art && px > 0 && px < h {
            let pw = ((w as f64 * px as f64 / h as f64).round() as u32).max(1);
            img = image::imageops::resize(&img, pw, px, image::imageops::FilterType::Triangle);
        }
        let flags = if pixel_art { ImageFlags::NEAREST | ImageFlags::PREMULTIPLIED } else { ImageFlags::PREMULTIPLIED };
        let pixels: Vec<RGBA8> = img.pixels().map(|p| RGBA8::new(p[0], p[1], p[2], p[3])).collect();
        let id = self.canvas.create_image(ImageSource::Rgba(ImgRef::new(&pixels, img.width() as usize, img.height() as usize)), flags).ok()?;
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

/// Everything that decides how a still frame looks: each pet's image and placement, bubbles and bowls.
fn scene_key(pets: &[PetDraw], world: &World) -> String {
    use std::fmt::Write;
    let mut k = String::new();
    for d in pets {
        let r = d.rect;
        let _ = write!(k, "{:?} {:.1} {:.1} {:.1} {:.1} {} {:.3} {:.2} {:?}|", d.frame.id, r.x, r.y, r.w, r.h, d.flip, d.sink, d.tilt, d.shadow.map(|(s, a)| (s.x.round(), s.w.round(), (a * 100.0).round())));
    }
    for p in &world.pets {
        if let (Some(text), true) = (&p.bubble, p.sink < 0.5) { let _ = write!(k, "{text} {:.1} {:.1}|", p.x, world.bubble_y(p)); }
    }
    for b in &world.bowls { let _ = write!(k, "bowl {:.1} {:.1} {:.2}|", b.x, b.floor, b.food); }
    k
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

/// How far (radians) a jump clip tips nose-down while the pet goes down. A jump clip is usually a leap
/// upwards, and even its last frames still look like taking off when the pet is really dropping to a
/// lower level. Pixel art keeps its own frames: rotating it breaks its hard edges.
fn fall_tilt(p: &Pet, s: f64) -> f64 {
    let down = if p.leaping {
        if p.leap_to.y >= p.leap_from.y - 1.0 { return 0.0 }
        // Tips over early, then levels out again for the last quarter so it lands on its feet.
        clamp((p.leap_k * 2.5).min((1.0 - p.leap_k) * 4.0), 0.0, 1.0)
    } else if !p.on_ground() && p.vy < 0.0 {
        clamp(-p.vy / (300.0 * s), 0.0, 1.0)
    } else {
        return 0.0;
    };
    down * MAX_FALL_TILT
}

/// About 45°: from a leap's head-up pose to head-down.
const MAX_FALL_TILT: f64 = 0.8;

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
