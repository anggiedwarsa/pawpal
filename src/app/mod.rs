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
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, PossiblyCurrentGlContext};
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
use crate::engine::breaks::{nudge_text, BreakTimer};
use crate::engine::calendar::{heads_up_text, Reminders, REMIND_AT};
use crate::engine::{World, TYPING_PAUSE};
use crate::platform;
use crate::render::Stage;
use crate::support::*;

const FRAME: Duration = Duration::from_micros(33_333);
/// While everything is near the Dock, the window only reaches this far above it (points, at size 1),
/// instead of covering the whole screen: a much smaller surface to keep in memory and composite.
const COMPACT_HEIGHT: f64 = 180.0;
/// How long things must stay low before the window shrinks again (it grows straight away).
const SHRINK_AFTER: Duration = Duration::from_secs(2);

/// Frames the spare window draws, unseen, at its new size before it's shown, and then side by side with the
/// old window (both showing the same picture) before that one is hidden.
const REVEAL_AFTER: u32 = 2;
const HIDE_AFTER: u32 = REVEAL_AFTER + 2;
/// And a few more before the hidden window is shrunk, so that it's surely off the screen by then. macOS
/// keeps its full-size buffers until it has drawn at the small size, so it draws twice more after that.
const SHRINK_AFTER_HIDING: u32 = HIDE_AFTER + 3;
const HANDOFF_DONE: u32 = SHRINK_AFTER_HIDING + 2;

/// The window with its OpenGL surface and renderer.
struct Gfx {
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    stage: Stage,
    /// Where the window is, in physical pixels (top-left origin).
    frame: Rect,
}

impl Gfx {
    /// Moves and sizes the window, and the drawing with it.
    fn set_frame(&mut self, frame: Rect) {
        self.frame = frame;
        let size = PhysicalSize::new(frame.w.round() as u32, frame.h.round() as u32);
        if !platform::set_frame(&self.window, frame) {
            self.window.set_outer_position(PhysicalPosition::new(frame.x.round() as i32, frame.y.round() as i32));
            let _ = self.window.request_inner_size(size);
        }
        // Size the drawing to the new window now: macOS reports the resize only later, and a frame
        // drawn at the old size gets stretched to fit.
        self.resize(size);
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        let _ = self.context.make_current(&self.surface);
        self.surface.resize(&self.context, w, h);
        self.stage.resize(size.width, size.height, self.window.scale_factor());
    }

    /// Draws the frame the stage has prepared, and puts it on screen if anything changed.
    fn render(&mut self, world: &World) {
        let _ = self.context.make_current(&self.surface);
        if self.stage.render(world) {
            let _ = self.surface.swap_buffers(&self.context);
        }
    }
}

/// Resizing a window while it's on screen shows a wrong picture for a moment: macOS stretches, drops or
/// shifts the old one until the new one is up (pets look bigger, vanish, or show twice). So on macOS the
/// window is never resized while it's seen: an invisible spare one is sized instead and draws along until
/// it has a picture, is shown, and only then is the old one hidden (and shrunk, to give its memory back).
struct Handoff {
    /// Frames drawn since the spare window was sized.
    frames: u32,
}

pub struct App {
    world: World,
    skins: BTreeMap<String, Rc<Skin>>,
    gfx: Option<Gfx>,
    /// The invisible second window (see Handoff); None where windows can't be made see-through.
    spare: Option<Gfx>,
    handoff: Option<Handoff>,
    tray: Option<TrayIcon>,
    tray_nodes: Vec<Node>,
    commands: HashMap<MenuId, Command>,
    pointer: Pointer,
    /// Last cursor position reported by the window (stage points), for systems that can't report it globally.
    window_cursor: Option<Point>,
    /// Where the window on screen is, in physical pixels (top-left origin).
    window_px: Rect,
    /// Where `layout` wants the window.
    wanted_px: Rect,
    /// A new `wanted_px` the window is moved to just before the next frame is drawn (see `draw`).
    pending_frame: Option<Rect>,
    hittest: bool,
    frame_count: u64,
    next_frame: Instant,
    last_tick: Instant,
    logged_typing: bool,
    import_rx: Option<Receiver<ImportMsg>>,
    login_enabled: Option<bool>,
    next_id: u64,
    reminders: Reminders,
    breaks: BreakTimer,
    /// The window covers only the strip above the Dock (see COMPACT_HEIGHT).
    compact: bool,
    low_since: Option<Instant>,
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
        spare: None,
        handoff: None,
        tray: None,
        tray_nodes: Vec::new(),
        commands: HashMap::new(),
        pointer: Pointer::default(),
        window_cursor: None,
        window_px: Rect::default(),
        wanted_px: Rect::default(),
        pending_frame: None,
        hittest: true,
        frame_count: 0,
        next_frame: Instant::now(),
        last_tick: Instant::now(),
        compact: false,
        low_since: None,
        reminders: Reminders::default(),
        breaks: BreakTimer::default(),
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
        self.spare = create_gfx(el).ok().filter(|g| platform::set_alpha(&g.window, 0.0));
        self.layout();
        (self.world.hour, self.world.bedtime) = (platform::local_hour(), settings::bedtime());
        self.load_pets();
        self.build_tray();
        self.next_frame = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                for g in [&mut self.gfx, &mut self.spare].into_iter().flatten() {
                    if g.window.id() == id { g.resize(size) }
                }
            }
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
            WindowEvent::RedrawRequested => {
                for g in [&mut self.gfx, &mut self.spare].into_iter().flatten() { g.stage.invalidate() }
                self.draw(0.0);
            }
            WindowEvent::CloseRequested => el.exit(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            // To save battery: 10 fps while pets are quiet, 15 while they walk about (clips play at 10-14 fps
            // anyway), and the full 30 only for fast motion and while the user holds a pet.
            let ticks = if self.pointer.pressed.is_some() || self.world.is_fast() {
                1
            } else if self.world.is_calm() && !self.wants_mouse() {
                3
            } else {
                2
            };
            self.frame(el, ticks);
            self.next_frame += FRAME * ticks as u32;
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
    Ok(Gfx { window, surface, context, stage: Stage::new(canvas), frame: Rect::default() })
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
    /// Applies a change to both windows' stages.
    fn stage_mut(&mut self, mut f: impl FnMut(&mut Stage)) {
        for g in [&mut self.gfx, &mut self.spare].into_iter().flatten() {
            let _ = g.context.make_current(&g.surface);
            f(&mut g.stage);
        }
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

    /// One step of `ticks` frames (more than one while the pets are quiet).
    fn frame(&mut self, el: &ActiveEventLoop, ticks: u64) {
        self.frame_count += ticks;
        let (now, before) = (self.frame_count, self.frame_count - ticks);
        let every = |n: u64| now / n != before / n;
        if every(60) { self.layout() }
        if every(900) { self.save_pets() }
        if every(30) {
            self.build_tray();
            (self.world.hour, self.world.bedtime) = (platform::local_hour(), settings::bedtime());
        }
        self.poll_import();
        self.handle_menu_events(el);
        if every(10) { self.watch_typing(every(30)) }
        if every(150) {
            self.check_calendar();
            self.check_breaks();
        }

        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f64().min(0.1);
        self.last_tick = now;
        let mouse = self.mouse_in_stage();
        self.world.step(dt, mouse, platform::seconds_since_key());
        self.fit_window();
        self.draw(dt);

        let hot = self.wants_mouse();
        if hot != self.hittest {
            self.hittest = hot;
            if let Some(g) = &self.gfx { let _ = g.window.set_cursor_hittest(hot); }
        }
    }

    fn draw(&mut self, dt: f64) {
        let Some(g) = &mut self.gfx else { return };
        // The slow part (decoding a clip that starts playing, rasterising a bubble) goes first, and the window
        // is moved or resized only after it, so that a window resized in place shows its new picture quickly.
        let _ = g.context.make_current(&g.surface);
        g.stage.prepare(&mut self.world, dt);
        self.apply_frame();
        if let (Some(g), Some(spare), Some(h)) = (&self.gfx, &mut self.spare, &self.handoff) {
            let _ = spare.context.make_current(&spare.surface);
            if h.frames >= SHRINK_AFTER_HIDING { spare.stage.invalidate() }
            spare.stage.match_anims(&g.stage);
            spare.stage.prepare(&mut self.world, 0.0);
            spare.render(&self.world);
        }
        if let Some(g) = &mut self.gfx { g.render(&self.world) }
        self.step_handoff();
    }

    /// Moves the window to where `layout` wants it; called by `draw` right before drawing. With a spare
    /// window, that one is sized (unseen) and takes over a few frames later (see Handoff).
    fn apply_frame(&mut self) {
        let Some(frame) = self.pending_frame else { return };
        let Some(spare) = &mut self.spare else {
            self.pending_frame = None;
            if let Some(g) = &mut self.gfx { g.set_frame(frame) }
            self.window_px = frame;
            return;
        };
        // A spare already on show can't be resized any more: the change waits until the handoff is over.
        if self.handoff.as_ref().is_some_and(|h| h.frames >= REVEAL_AFTER) { return }
        self.pending_frame = None;
        spare.set_frame(frame);
        self.handoff = Some(Handoff { frames: 0 });
    }

    /// Shows the spare window once it has drawn at its new size, then hides the old one behind it and swaps.
    fn step_handoff(&mut self) {
        let Some(h) = &mut self.handoff else { return };
        h.frames += 1;
        let frames = h.frames;
        let (Some(g), Some(spare)) = (&mut self.gfx, &mut self.spare) else { return };
        if frames == REVEAL_AFTER {
            platform::set_alpha(&spare.window, 1.0);
        } else if frames == HIDE_AFTER {
            platform::set_alpha(&g.window, 0.0);
            let _ = g.window.set_cursor_hittest(false);
            let _ = spare.window.set_cursor_hittest(self.hittest);
            std::mem::swap(g, spare);
            self.window_px = g.frame;
        } else if frames == SHRINK_AFTER_HIDING {
            let f = self.window_px;
            spare.set_frame(Rect::new(f.x, f.max_y() - 1.0, 1.0, 1.0));
        } else if frames == HANDOFF_DONE {
            let _ = spare.context.make_current(&spare.surface);
            spare.stage.release();
            self.handoff = None;
        }
    }

    /// Grows the window to the whole screen as soon as something goes (or is about to go) above the strip
    /// over the Dock, and shrinks it back once everything has stayed low for a moment.
    fn fit_window(&mut self) {
        let room = self.world.ground + COMPACT_HEIGHT * self.world.s;
        let need = if self.pointer.pressed.is_some() { f64::INFINITY } else { self.world.headroom() };
        let compact = if self.compact {
            need <= room
        } else if need <= room - 40.0 * self.world.s {
            self.low_since.get_or_insert_with(Instant::now).elapsed() >= SHRINK_AFTER
        } else {
            self.low_since = None;
            false
        };
        if compact != self.compact {
            self.compact = compact;
            self.low_since = None;
            self.layout();
        }
    }

    /// Every few seconds: sends a pet out if a calendar event starts in 10, 5 or 1 minutes.
    fn check_calendar(&mut self) {
        if !settings::calendar_heads_ups() || self.world.pets.is_empty() { return }
        let soon = REMIND_AT[0] as f64 * 60.0 + 30.0;
        let events = platform::upcoming_events(soon);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
        if let Some((event, minutes)) = self.reminders.due(&events, now) {
            self.world.remind(&heads_up_text(&event.title, minutes));
        }
    }

    /// Every few seconds: sends a pet out to suggest a break after a long while at the computer.
    /// It waits while a movie is on or another heads-up is out.
    fn check_breaks(&mut self) {
        let w = &self.world;
        let hold = w.pets.is_empty() || w.fullscreen || w.pets.iter().any(|p| p.activity == Remind);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
        if self.breaks.due(now, w.mouse_still, settings::break_every(), hold) {
            self.world.remind(nudge_text());
        }
    }

    /// `check_fullscreen` is passed about once a second: listing every window costs more than the rest.
    fn watch_typing(&mut self, check_fullscreen: bool) {
        platform::note_frontmost();
        let fullscreen = if check_fullscreen { platform::fullscreen_active() } else { self.world.fullscreen };
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

    /// Sizes the window to cover the screen from the menu bar (or, when compact, from a strip above the Dock)
    /// down to just over the Dock or taskbar.
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
        let ground = (bottom - shelf.ground) / k;
        let top = if self.compact { (bottom - (ground + COMPACT_HEIGHT * self.world.s) * k).max(shelf.top) } else { shelf.top };
        let frame = Rect::new(shelf.screen.x, top, shelf.screen.w, bottom - top);
        if frame != self.wanted_px {
            self.wanted_px = frame;
            self.pending_frame = Some(frame);
        }
        let w = &mut self.world;
        w.ground = ground;
        w.below = if corner_free { (w.ground - 2.0).max(0.0) } else { 0.0 };
        (w.dock_start, w.dock_end) = match shelf.dock.filter(|_| corner_free) {
            Some(d) => ((d.lo - shelf.screen.x) / k, (d.hi - shelf.screen.x) / k),
            None => (f64::NEG_INFINITY, f64::INFINITY),
        };
        w.min_x = (shelf.min_x - shelf.screen.x) / k;
        w.max_x = (shelf.max_x - shelf.screen.x) / k;
        w.width = frame.w / k;
        // Pets live on the whole screen even while the window is compact (stage y counts up from the bottom).
        w.height = (bottom - shelf.top) / k;
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
            Command::ToggleCalendar => settings::set_calendar_heads_ups(!settings::calendar_heads_ups()),
            Command::BreakEvery(n) => settings::set_break_every(n),
            Command::Bedtime(h) => {
                settings::set_bedtime(h);
                self.world.bedtime = h;
            }
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
