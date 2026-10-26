//! Owns the overlay window, the frame timer, the tray icon and saving; menus, pointer input and importing
//! live in the submodules.

mod import_flow;
mod menus;
mod pointer;

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use tray_icon::menu::{MenuEvent, MenuId};
use tray_icon::{TrayIcon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId, WindowLevel};

use import_flow::ImportMsg;
use menus::{Command, Node};
use pointer::Pointer;

use crate::data::settings::{self, PetSave};
use crate::data::skin::{self, Skin};
use crate::domain::Activity::*;
use crate::domain::{Pet, Species};
use crate::engine::{World, TYPING_PAUSE};
use crate::platform;
use crate::render::Stage;
use crate::support::*;

const FRAME: Duration = Duration::from_micros(33_333);

/// The window with its OpenGL surface and renderer.
struct Gfx {
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    stage: Stage,
}

pub struct App {
    world: World,
    skins: BTreeMap<String, Rc<Skin>>,
    gfx: Option<Gfx>,
    tray: Option<TrayIcon>,
    tray_nodes: Vec<Node>,
    commands: HashMap<MenuId, Command>,
    pointer: Pointer,
    /// Last cursor position reported by the window (stage points), for systems that can't report it globally.
    window_cursor: Option<Point>,
    /// Where the window is, in physical pixels (top-left origin).
    window_px: Rect,
    hittest: bool,
    frame_count: u64,
    next_frame: Instant,
    last_tick: Instant,
    logged_typing: bool,
    import_rx: Option<Receiver<ImportMsg>>,
    login_enabled: Option<bool>,
    next_id: u64,
}

pub fn run() {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        builder.with_activation_policy(ActivationPolicy::Accessory).with_default_menu(false);
    }
    #[cfg(target_os = "linux")]
    {
        // Wayland doesn't let windows place themselves, so use X11 (XWayland) whenever it's there.
        use winit::platform::x11::EventLoopBuilderExtX11;
        if std::env::var_os("DISPLAY").is_some() { builder.with_x11(); }
    }
    let event_loop = builder.build().expect("PawPal couldn't start its event loop");
    let mut app = App {
        world: World::new(),
        skins: skin::load_all(),
        gfx: None,
        tray: None,
        tray_nodes: Vec::new(),
        commands: HashMap::new(),
        pointer: Pointer::default(),
        window_cursor: None,
        window_px: Rect::default(),
        hittest: true,
        frame_count: 0,
        next_frame: Instant::now(),
        last_tick: Instant::now(),
        logged_typing: false,
        import_rx: None,
        login_enabled: login_item().and_then(|l| l.is_enabled().ok()),
        next_id: 1,
    };
    let _ = event_loop.run_app(&mut app);
    app.save_pets();
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() { return }
        match create_gfx(el) {
            Ok(gfx) => self.gfx = Some(gfx),
            Err(e) => {
                eprintln!("PawPal couldn't open its window: {e}");
                el.exit();
                return;
            }
        }
        self.layout();
        self.load_pets();
        self.build_tray();
        self.next_frame = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => self.layout(),
            WindowEvent::CursorMoved { position, .. } => {
                let at = self.window_to_stage(position.x, position.y);
                self.window_cursor = Some(at);
                self.mouse_moved(at);
            }
            WindowEvent::CursorLeft { .. } => self.window_cursor = None,
            WindowEvent::MouseInput { state, button, .. } => {
                let at = self.window_cursor.unwrap_or_else(|| self.mouse_in_stage());
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => self.mouse_down(at),
                    (MouseButton::Left, ElementState::Released) => self.mouse_up(),
                    (MouseButton::Right, ElementState::Pressed) => self.pet_context_menu(at),
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => self.draw(0.0),
            WindowEvent::CloseRequested => el.exit(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            self.frame(el);
            self.next_frame += FRAME;
            if self.next_frame < now { self.next_frame = now + FRAME }
        }
        el.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }

    fn exiting(&mut self, _el: &ActiveEventLoop) { self.save_pets() }
}

fn create_gfx(el: &ActiveEventLoop) -> Result<Gfx, Box<dyn std::error::Error>> {
    #[allow(unused_mut)]
    let mut attrs = Window::default_attributes()
        .with_title("PawPal")
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_active(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_inner_size(PhysicalSize::new(800, 300));
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::WindowAttributesExtMacOS;
        attrs = attrs.with_has_shadow(false).with_accepts_first_mouse(true);
    }
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs = attrs.with_skip_taskbar(true);
    }
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::WindowAttributesExtX11;
        attrs = attrs.with_override_redirect(true);
    }

    let template = ConfigTemplateBuilder::new().with_alpha_size(8).with_transparency(true);
    let (window, config) = DisplayBuilder::new().with_window_attributes(Some(attrs)).build(el, template, pick_config)?;
    let window = window.ok_or("no window")?;
    let raw = window.window_handle()?.as_raw();
    let display = config.display();
    let context = unsafe {
        display
            .create_context(&config, &ContextAttributesBuilder::new().build(Some(raw)))
            .or_else(|_| display.create_context(&config, &ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None)).build(Some(raw))))?
    };
    let surface_attrs = window.build_surface_attributes(Default::default())?;
    let surface = unsafe { display.create_window_surface(&config, &surface_attrs)? };
    let context = context.make_current(&surface)?;
    let _ = surface.set_swap_interval(&context, SwapInterval::DontWait);
    let renderer = unsafe { femtovg::renderer::OpenGl::new_from_function_cstr(|s| display.get_proc_address(s).cast()) }?;
    let canvas = femtovg::Canvas::new(renderer)?;
    platform::prepare_window(&window);
    window.set_cursor_hittest(false).ok();
    Ok(Gfx { window, surface, context, stage: Stage::new(canvas) })
}

/// Prefers a config that can be see-through, then the simplest one.
fn pick_config(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs
        .reduce(|best, c| {
            let (bt, ct) = (best.supports_transparency().unwrap_or(false), c.supports_transparency().unwrap_or(false));
            if (ct && !bt) || (ct == bt && c.num_samples() < best.num_samples()) { c } else { best }
        })
        .expect("no OpenGL config")
}

fn login_item() -> Option<auto_launch::AutoLaunch> {
    let exe = std::env::current_exe().ok()?;
    let mut b = auto_launch::AutoLaunchBuilder::new();
    b.set_app_name("PawPal");
    #[cfg(target_os = "macos")]
    {
        // Inside PawPal.app, register the bundle like the Swift version did; a bare binary gets a launch agent.
        let in_bundle = exe.to_string_lossy().contains(".app/Contents/MacOS/");
        b.set_macos_launch_mode(if in_bundle { auto_launch::MacOSLaunchMode::SMAppService } else { auto_launch::MacOSLaunchMode::LaunchAgent });
    }
    b.set_app_path(&exe.to_string_lossy());
    b.build().ok()
}

impl App {
    fn stage_mut(&mut self, f: impl FnOnce(&mut Stage)) {
        if let Some(g) = &mut self.gfx { f(&mut g.stage) }
    }

    fn scale(&self) -> f64 { self.gfx.as_ref().map_or(1.0, |g| g.window.scale_factor()) }

    /// Converts a window position (physical pixels, y down) to stage points (y up).
    fn window_to_stage(&self, x: f64, y: f64) -> Point {
        let k = self.scale();
        pt(x / k, (self.window_px.h - y) / k)
    }

    /// Converts a global rectangle (physical pixels, y down) to stage points (y up).
    fn global_to_stage(&self, r: Rect) -> Rect {
        let k = self.scale();
        Rect::new((r.x - self.window_px.x) / k, (self.window_px.max_y() - r.max_y()) / k, r.w / k, r.h / k)
    }

    fn mouse_in_stage(&self) -> Point {
        match platform::cursor() {
            Some(c) => self.window_to_stage(c.x - self.window_px.x, c.y - self.window_px.y),
            None => self.window_cursor.unwrap_or_default(),
        }
    }

    fn frame(&mut self, el: &ActiveEventLoop) {
        self.frame_count += 1;
        if self.frame_count % 60 == 0 { self.layout() }
        if self.frame_count % 900 == 0 { self.save_pets() }
        if self.frame_count % 30 == 0 { self.build_tray() }
        self.poll_import();
        self.handle_menu_events(el);
        if self.frame_count % 10 == 0 { self.watch_typing() }
        // Quiet pets are animated at half rate to save battery.
        if self.frame_count % 2 == 1 && self.world.is_calm() && !self.wants_mouse() { return }

        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f64().min(0.1);
        self.last_tick = now;
        let mouse = self.mouse_in_stage();
        self.world.step(dt, mouse, platform::seconds_since_key());
        self.draw(dt);

        let hot = self.wants_mouse();
        if hot != self.hittest {
            self.hittest = hot;
            if let Some(g) = &self.gfx { let _ = g.window.set_cursor_hittest(hot); }
        }
    }

    fn draw(&mut self, dt: f64) {
        let Some(g) = &mut self.gfx else { return };
        g.stage.render(&mut self.world, dt);
        let _ = g.surface.swap_buffers(&g.context);
    }

    fn watch_typing(&mut self) {
        platform::note_frontmost();
        let fullscreen = platform::fullscreen_active();
        if fullscreen != self.world.fullscreen {
            self.world.set_fullscreen(fullscreen);
            self.layout();
        }
        if self.world.since_key < 1.0 { self.ask_for_accessibility_once() }
        let spot = if self.world.since_key < TYPING_PAUSE { platform::typing_spot() } else { None };
        self.world.typing_zone = spot.map(|(r, _)| self.global_to_stage(r));
        self.world.typing_zone_is_exact = spot.is_some_and(|s| s.1);
        if let (Some((rect, exact)), true, false) = (spot, self.world.since_key < 0.4, self.logged_typing) {
            self.logged_typing = true;
            let line = format!("app={} exact={exact} rect={rect:?} {}\n", platform::frontmost_app(), platform::describe_typing());
            eprintln!("PawPal typing spot: {line}");
            if let Some(dir) = skin::root().parent() { let _ = std::fs::write(dir.join("typing-debug.txt"), line); }
        }
        if self.world.since_key > TYPING_PAUSE { self.logged_typing = false }
    }

    /// The first time the user types, asks for Accessibility so pets can find the text cursor.
    /// Asked once per build: an ad-hoc signed rebuild makes macOS forget the permission.
    fn ask_for_accessibility_once(&mut self) {
        if !platform::has_accessibility_setting() || platform::accessibility_trusted() { return }
        let build = std::env::current_exe()
            .and_then(std::fs::metadata)
            .and_then(|m| m.modified())
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|_| "?".into());
        if settings::asked_accessibility().as_deref() == Some(build.as_str()) { return }
        settings::set_asked_accessibility(build);
        platform::request_accessibility();
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        let k = self.scale();
        let Some(g) = &mut self.gfx else { return };
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        g.surface.resize(&g.context, w, h);
        g.stage.resize(size.width, size.height, k);
    }

    /// Sizes the window to cover the screen from the menu bar down to just over the Dock or taskbar.
    pub fn layout(&mut self) {
        let Some(g) = &self.gfx else { return };
        let k = g.window.scale_factor();
        let shelf = platform::shelf(settings::walk_area()).or_else(|| {
            let m = g.window.current_monitor().or_else(|| g.window.primary_monitor())?;
            let (p, s) = (m.position(), m.size());
            let screen = Rect::new(p.x as f64, p.y as f64, s.width as f64, s.height as f64);
            Some(platform::Shelf { screen, top: screen.y, ground: screen.max_y(), min_x: screen.min_x(), max_x: screen.max_x(), dock: None })
        });
        let Some(mut shelf) = shelf else { return };
        if self.world.fullscreen {
            // The Dock or taskbar is hidden, so the corner pets peek from is the screen's own.
            (shelf.ground, shelf.min_x, shelf.max_x, shelf.dock) = (shelf.screen.max_y(), shelf.screen.min_x(), shelf.screen.max_x(), None);
        }
        // Where the Dock doesn't span the whole screen, the window reaches down to the screen's bottom edge:
        // the Dock is a step up, and beside it pets stand on the real bottom of the screen.
        let corner_free = shelf.dock.is_some_and(|d| d.hi < shelf.screen.max_x() - 1.0 || d.lo > shelf.screen.min_x() + 1.0);
        let bottom = if corner_free { shelf.screen.max_y() } else { shelf.ground + 6.0 * k };
        let frame = Rect::new(shelf.screen.x, shelf.top, shelf.screen.w, bottom - shelf.top);
        if frame != self.window_px {
            self.window_px = frame;
            g.window.set_outer_position(PhysicalPosition::new(frame.x.round() as i32, frame.y.round() as i32));
            if let Some(size) = g.window.request_inner_size(PhysicalSize::new(frame.w.round() as u32, frame.h.round() as u32)) {
                self.resize(size);
            }
        }
        let w = &mut self.world;
        w.ground = (bottom - shelf.ground) / k;
        w.below = if corner_free { (w.ground - 2.0).max(0.0) } else { 0.0 };
        (w.dock_start, w.dock_end) = match shelf.dock.filter(|_| corner_free) {
            Some(d) => ((d.lo - shelf.screen.x) / k, (d.hi - shelf.screen.x) / k),
            None => (f64::NEG_INFINITY, f64::INFINITY),
        };
        w.min_x = (shelf.min_x - shelf.screen.x) / k;
        w.max_x = (shelf.max_x - shelf.screen.x) / k;
        w.width = frame.w / k;
        w.height = frame.h / k;
        for i in 0..w.pets.len() {
            w.with_pet(i, |w, p| if p.activity != VisitCursor { p.x = w.clamp_x(p, p.x, 20.0) });
        }
        if let Some(g) = &self.gfx { g.window.request_redraw() }
    }

    // Tray and menus

    fn build_tray(&mut self) {
        let nodes = self.tray_menu();
        if self.tray.is_some() && nodes == self.tray_nodes { return }
        let (menu, cmds) = menus::materialize(&nodes, "t");
        self.commands.retain(|id, _| !id.as_ref().starts_with('t'));
        self.commands.extend(cmds);
        self.tray_nodes = nodes;
        match &self.tray {
            Some(tray) => tray.set_menu(Some(Box::new(menu))),
            None => {
                let mut b = TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip("PawPal");
                if cfg!(target_os = "macos") { b = b.with_title("🐾") } else { b = b.with_icon(paw_icon()) }
                self.tray = b.build().map_err(|e| eprintln!("PawPal tray icon: {e}")).ok();
            }
        }
    }

    fn set_status(&self, text: &str) {
        let Some(tray) = &self.tray else { return };
        if cfg!(target_os = "macos") { tray.set_title(Some(text)) } else { let _ = tray.set_tooltip(Some(format!("PawPal {text}"))); }
    }

    fn pet_context_menu(&mut self, at: Point) {
        let Some(i) = self.pet_at(at).and_then(|id| self.world.index_of(id)) else { return };
        let (menu, cmds) = menus::materialize(&menus::pet_menu(&self.world.pets[i]), "c");
        self.commands.retain(|id, _| !id.as_ref().starts_with('c'));
        self.commands.extend(cmds);
        if let Some(g) = &self.gfx { platform::show_context_menu(&g.window, &menu) }
        platform::done_interacting();
    }

    fn handle_menu_events(&mut self, el: &ActiveEventLoop) {
        let events: Vec<MenuEvent> = MenuEvent::receiver().try_iter().collect();
        for e in events {
            if let Some(cmd) = self.commands.get(&e.id).cloned() {
                self.run_command(cmd, el);
                self.build_tray();
            }
        }
    }

    fn run_command(&mut self, cmd: Command, el: &ActiveEventLoop) {
        let w = &mut self.world;
        match cmd {
            Command::Do { pets, activity } => {
                if activity == Eat {
                    let only = (pets.len() == 1).then(|| pets[0]);
                    match only {
                        Some(id) => w.with_id(id, |w, p| w.feed_one(p)),
                        None => w.feed(),
                    }
                    return;
                }
                // Commanded play brings a ball or butterfly to each pet's own level when there isn't one.
                for id in pets { w.with_id(id, |w, p| w.start(p, activity, true)) }
            }
            Command::ComeHome(id) => w.with_id(id, |w, p| w.go_home(p, true)),
            Command::FeedPet(id) => w.with_id(id, |w, p| w.feed_one(p)),
            Command::FeedAll => w.feed(),
            Command::Treat => w.treat(),
            Command::ThrowBall => w.throw_ball(),
            Command::ReleaseButterfly => w.release_butterfly(),
            Command::Rename(id) => self.rename(id),
            Command::Flip(name) => settings::toggle_flipped(&name),
            Command::Import { skin, action } => self.import_clip_ui(skin, action),
            Command::Remove(id) => self.remove_pet(id),
            Command::Adopt(name) => self.adopt(&name),
            Command::OpenClips => platform::open_folder(&skin::root()),
            Command::Scale(v) => settings::set_scale(v),
            Command::Walk(v) => {
                settings::set_walk_area(v);
                self.layout();
            }
            Command::AllowAccessibility => platform::request_accessibility(),
            Command::ToggleVisit => settings::set_visit_idle_cursor(!settings::visit_idle_cursor()),
            Command::ToggleAuto => settings::set_auto(!settings::auto()),
            Command::ToggleLogin => {
                if let Some(l) = login_item() {
                    let on = l.is_enabled().unwrap_or(false);
                    let r = if on { l.disable() } else { l.enable() };
                    if let Err(e) = r { eprintln!("PawPal login item: {e}") }
                    self.login_enabled = l.is_enabled().ok();
                }
            }
            Command::Quit => {
                self.save_pets();
                el.exit();
            }
        }
    }

    // Pets

    fn load_pets(&mut self) {
        let mut saved: Vec<PetSave> = settings::pets().into_iter().filter(|s| s.skin.as_ref().is_some_and(|k| self.skins.contains_key(k))).collect();
        if saved.is_empty() {
            if let Some(first) = self.skins.keys().next() {
                saved = vec![PetSave { name: first.clone(), species: Species::Cat, skin: Some(first.clone()), hunger: 20.0, energy: 85.0, happiness: 70.0 }];
            }
        }
        for s in saved { self.add_pet(s); }
        self.save_pets();
    }

    fn add_pet(&mut self, s: PetSave) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut p = Pet::new(id, s.name, s.species, s.skin.and_then(|k| self.skins.get(&k).cloned()));
        p.hunger = s.hunger;
        p.energy = s.energy;
        p.happiness = s.happiness;
        p.x = self.world.random_x();
        p.facing = if chance(0.5) { 1.0 } else { -1.0 };
        self.world.pets.push(p);
        id
    }

    fn remove_pet(&mut self, id: u64) {
        self.stage_mut(|s| s.forget(id));
        self.world.pets.retain(|p| p.id != id);
        self.save_pets();
    }

    pub fn adopt(&mut self, skin: &str) {
        let id = self.add_pet(PetSave { name: skin.into(), species: Species::Cat, skin: Some(skin.into()), hunger: 10.0, energy: 90.0, happiness: 80.0 });
        self.world.with_id(id, |_, p| p.say(format!("Hi! I'm {skin}!"), 3.0));
        self.save_pets();
    }

    fn rename(&mut self, id: u64) {
        let Some(i) = self.world.index_of(id) else { return };
        let old = self.world.pets[i].name.clone();
        let Some(name) = platform::prompt_text("PawPal", &format!("Rename {old}"), &old) else { return };
        let name = name.trim().to_string();
        if name.is_empty() { return }
        if let Some(i) = self.world.index_of(id) {
            let p = &mut self.world.pets[i];
            p.say(format!("I'm {name} now!"), 2.6);
            p.name = name;
            self.save_pets();
        }
    }

    fn save_pets(&self) {
        if self.gfx.is_none() { return } // nothing was loaded, so don't overwrite what's saved
        settings::set_pets(
            self.world
                .pets
                .iter()
                .map(|p| PetSave {
                    name: p.name.clone(),
                    species: p.species,
                    skin: p.skin.as_ref().map(|s| s.name.clone()),
                    hunger: p.hunger,
                    energy: p.energy,
                    happiness: p.happiness,
                })
                .collect(),
        );
    }
}

/// The app icon, for trays that need a picture (Windows and Linux).
fn paw_icon() -> tray_icon::Icon {
    let img = image::load_from_memory(include_bytes!("../../assets/icon.png")).expect("bundled icon").to_rgba8();
    let img = image::imageops::resize(&img, 64, 64, image::imageops::FilterType::Triangle);
    tray_icon::Icon::from_rgba(img.into_raw(), 64, 64).expect("valid icon")
}
