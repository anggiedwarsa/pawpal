//! Linux (X11, including XWayland): pets stand on the bottom panel, read from the window manager's work area.
//! Wayland doesn't let apps place windows or read the cursor, so PawPal runs through XWayland there.

use std::cell::RefCell;
use std::process::Command;

use image::RgbaImage;
use winit::window::Window;
use x11rb::connection::Connection;
use x11rb::protocol::screensaver::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window as XWindow};
use x11rb::rust_connection::RustConnection;

use super::{KeyGuess, Shelf};
use crate::data::settings::WalkArea;
use crate::support::{pt, Point, Rect};

struct X {
    conn: RustConnection,
    root: XWindow,
    width: u16,
    height: u16,
}

thread_local! {
    static X11: Option<X> = x11rb::connect(None).ok().map(|(conn, n)| {
        let s = &conn.setup().roots[n];
        let (root, width, height) = (s.root, s.width_in_pixels, s.height_in_pixels);
        X { conn, root, width, height }
    });
    static KEYS: RefCell<KeyGuess> = const { RefCell::new(KeyGuess::new()) };
}

fn with_x<T>(f: impl FnOnce(&X) -> Option<T>) -> Option<T> { X11.with(|x| x.as_ref().and_then(f)) }

fn atom(x: &X, name: &str) -> Option<u32> { Some(x.conn.intern_atom(false, name.as_bytes()).ok()?.reply().ok()?.atom) }

fn cardinals(x: &X, win: XWindow, name: &str, kind: AtomEnum) -> Option<Vec<u32>> {
    let prop = x.conn.get_property(false, win, atom(x, name)?, kind, 0, 64).ok()?.reply().ok()?;
    let values: Vec<u32> = prop.value32()?.collect();
    Some(values)
}

pub fn shelf(_walk: WalkArea) -> Option<Shelf> {
    with_x(|x| {
        let screen = Rect::new(0.0, 0.0, x.width as f64, x.height as f64);
        // _NET_WORKAREA is the desktop minus panels; the space below it is the bottom panel.
        let work = cardinals(x, x.root, "_NET_WORKAREA", AtomEnum::CARDINAL)
            .filter(|v| v.len() >= 4)
            .map_or(screen, |v| Rect::new(v[0] as f64, v[1] as f64, v[2] as f64, v[3] as f64));
        Some(Shelf { screen, top: work.min_y(), ground: work.max_y(), min_x: screen.min_x(), max_x: screen.max_x(), dock: None })
    })
}

pub fn cursor() -> Option<Point> {
    with_x(|x| {
        let p = x.conn.query_pointer(x.root).ok()?.reply().ok()?;
        Some(pt(p.root_x as f64, p.root_y as f64))
    })
}

/// X11 only reports the time since any input, so a key press is input without the cursor moving.
pub fn seconds_since_key() -> f64 {
    let idle = with_x(|x| Some(x.conn.screensaver_query_info(x.root).ok()?.reply().ok()?.ms_since_user_input as f64 / 1000.0));
    match idle {
        Some(idle) => KEYS.with(|k| k.borrow_mut().update(idle, cursor())),
        None => f64::INFINITY,
    }
}

/// Terminals (by WM_CLASS), where typing happens on the bottom lines of the window.
const TERMINALS: [&str; 12] = ["gnome-terminal-server", "konsole", "kitty", "alacritty", "org.wezfurlong.wezterm", "xterm", "tilix",
                               "terminator", "xfce4-terminal", "com.mitchellh.ghostty", "foot", "ptyxis"];

struct Front {
    rect: Rect,
    class: String,
}

fn front_window() -> Option<Front> {
    with_x(|x| {
        let win = *cardinals(x, x.root, "_NET_ACTIVE_WINDOW", AtomEnum::WINDOW)?.first()?;
        if win == 0 { return None }
        let pid = cardinals(x, win, "_NET_WM_PID", AtomEnum::CARDINAL).and_then(|v| v.first().copied());
        if pid == Some(std::process::id()) { return None }
        let geo = x.conn.get_geometry(win).ok()?.reply().ok()?;
        let at = x.conn.translate_coordinates(win, x.root, 0, 0).ok()?.reply().ok()?;
        let class = x.conn.get_property(false, win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256).ok()?.reply().ok()?;
        let class = String::from_utf8_lossy(&class.value).split('\0').nth(1).unwrap_or("").to_lowercase();
        Some(Front { rect: Rect::new(at.dst_x as f64, at.dst_y as f64, geo.width as f64, geo.height as f64), class })
    })
}

/// Where the user is typing. X11 has no caret position, so this is always a guess: the front window,
/// or a terminal's prompt row.
pub fn typing_spot() -> Option<(Rect, bool)> {
    let w = front_window()?;
    if TERMINALS.contains(&w.class.as_str()) {
        return Some((Rect::new(w.rect.x, w.rect.max_y() - 120.0, w.rect.w, 120.0), false));
    }
    Some((w.rect, false))
}

pub fn describe_typing() -> String { format!("x11: front window={:?}", typing_spot()) }

/// Whether the active window asked the window manager to be full screen.
pub fn fullscreen_active() -> bool {
    with_x(|x| {
        let win = *cardinals(x, x.root, "_NET_ACTIVE_WINDOW", AtomEnum::WINDOW)?.first()?;
        let full = atom(x, "_NET_WM_STATE_FULLSCREEN")?;
        Some(cardinals(x, win, "_NET_WM_STATE", AtomEnum::ATOM)?.contains(&full))
    })
    .unwrap_or(false)
}

pub fn frontmost_app() -> String { front_window().map_or_else(|| "-".into(), |w| w.class) }

pub fn accessibility_trusted() -> bool { true }
pub fn has_accessibility_setting() -> bool { false }
pub fn request_accessibility() {}

/// Moving and resizing at once needs nothing special here: the app falls back to winit.
pub fn set_frame(_window: &Window, _r: Rect) -> bool { false }

/// Window see-through-ness isn't set here, so the app resizes its one window in place.
pub fn set_alpha(_window: &Window, _alpha: f64) -> bool { false }

/// The window is created override-redirect (see the app), so the window manager leaves it alone.
pub fn prepare_window(_window: &Window) {}
pub fn will_interact() {}
pub fn done_interacting() {}
pub fn note_frontmost() {}

/// Context menus need GTK on Linux; each pet's menu is in the tray menu instead.
pub fn show_context_menu(_window: &Window, _menu: &tray_icon::menu::Menu) {}

/// Asks for a line of text with zenity or kdialog; None when cancelled or neither is installed.
pub fn prompt_text(title: &str, message: &str, default: &str) -> Option<String> {
    let zenity = Command::new("zenity").args(["--entry", "--title", title, "--text", message, "--entry-text", default]).output();
    let out = match zenity {
        Ok(out) => out,
        Err(_) => Command::new("kdialog").args(["--title", title, "--inputbox", message, default]).output().ok()?,
    };
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string())
}

pub fn lift_subject(_img: &RgbaImage) -> Option<Option<RgbaImage>> { None }

/// No calendar to read here yet (an iCal feed could be added); no heads-ups.
pub fn has_calendar() -> bool { false }
pub fn upcoming_events(_within: f64) -> Vec<crate::domain::CalendarEvent> { Vec::new() }

/// Whether a sound card has a capture ('c') or playback ('p') stream running, from /proc/asound. PulseAudio and
/// PipeWire keep a device running while any app uses it and suspend it soon after.
fn alsa_running(kind: char) -> bool {
    let Ok(cards) = std::fs::read_dir("/proc/asound") else { return false };
    cards.flatten().filter(|c| c.file_name().to_string_lossy().starts_with("card")).any(|card| {
        let Ok(pcms) = std::fs::read_dir(card.path()) else { return false };
        pcms.flatten().filter(|p| p.file_name().to_string_lossy().starts_with("pcm") && p.file_name().to_string_lossy().ends_with(kind)).any(|pcm| {
            let Ok(subs) = std::fs::read_dir(pcm.path()) else { return false };
            subs.flatten().any(|s| std::fs::read_to_string(s.path().join("status")).is_ok_and(|t| t.contains("state: RUNNING")))
        })
    })
}

/// Some window's title is a Google Meet call.
fn meeting_window_open() -> bool {
    with_x(|x| {
        let (name, utf8) = (atom(x, "_NET_WM_NAME")?, atom(x, "UTF8_STRING")?);
        let windows = x.conn.get_property(false, x.root, atom(x, "_NET_CLIENT_LIST")?, AtomEnum::WINDOW, 0, 1024).ok()?.reply().ok()?;
        let open = windows.value32()?.any(|w| {
            let title = x.conn.get_property(false, w, name, utf8, 0, 256).ok().and_then(|c| c.reply().ok());
            title.is_some_and(|t| super::is_meeting_title(&String::from_utf8_lossy(&t.value)))
        });
        Some(open)
    })
    .unwrap_or(false)
}

/// The user is in a call: the mic is open, or a Meet call is open (Meet may leave the mic closed while muted).
pub fn in_call() -> bool { alsa_running('c') || meeting_window_open() }

/// Some app is playing sound.
pub fn audio_playing() -> bool { alsa_running('p') }
