//! macOS: the Dock through Accessibility (or its preferences), the caret through Accessibility,
//! and an overlay window that sits above the Dock on every Space.

use std::cell::Cell;
use std::ffi::c_void;
use std::process::Command;
use std::time::Instant;

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFGetTypeID, CFRelease, CFType, CFTypeID, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringGetTypeID, CFStringRef};
use image::RgbaImage;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplicationActivationOptions, NSApplicationActivationPolicy, NSEvent, NSRunningApplication, NSScreen, NSView, NSWindowCollectionBehavior, NSWorkspace};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::Shelf;
use crate::data::settings::WalkArea;
use crate::support::{pt, Point, Rect, Span};

type AXUIElementRef = CFTypeRef;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct CGRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(el: AXUIElementRef, attr: CFStringRef, value: *mut CFTypeRef) -> i32;
    fn AXUIElementSetAttributeValue(el: AXUIElementRef, attr: CFStringRef, value: CFTypeRef) -> i32;
    fn AXUIElementCopyParameterizedAttributeValue(el: AXUIElementRef, attr: CFStringRef, param: CFTypeRef, value: *mut CFTypeRef) -> i32;
    fn AXUIElementGetTypeID() -> CFTypeID;
    fn AXValueGetTypeID() -> CFTypeID;
    fn AXValueGetValue(value: CFTypeRef, kind: u32, out: *mut c_void) -> bool;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state: i32, kind: u32) -> f64;
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFArrayRef;
    fn CGRectMakeWithDictionaryRepresentation(dict: CFDictionaryRef, rect: *mut CGRect) -> bool;
}

#[repr(C)]
struct AudioAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyData(id: u32, addr: *const AudioAddress, qual_size: u32, qual: *const c_void, size: *mut u32, data: *mut c_void) -> i32;
}

// Core Media IO (cameras) uses the same property addresses as Core Audio.
#[link(name = "CoreMediaIO", kind = "framework")]
extern "C" {
    fn CMIOObjectGetPropertyDataSize(id: u32, addr: *const AudioAddress, qual_size: u32, qual: *const c_void, size: *mut u32) -> i32;
    fn CMIOObjectGetPropertyData(id: u32, addr: *const AudioAddress, qual_size: u32, qual: *const c_void, size: u32, used: *mut u32, data: *mut c_void) -> i32;
}

const AX_POINT: u32 = 1;
const AX_SIZE: u32 = 2;
const AX_RECT: u32 = 3;

thread_local! {
    /// Backing scale of the Dock's screen: points times this are the "physical" pixels the app expects.
    static SCALE: Cell<f64> = const { Cell::new(2.0) };
    /// The app that was in front before the user grabbed a pet, so it can get focus back.
    static PREVIOUS_APP: Cell<i32> = const { Cell::new(0) };
    /// Apps already asked to expose their web content to Accessibility (see `wake_web_accessibility`).
    static WOKEN: std::cell::RefCell<Vec<i32>> = const { std::cell::RefCell::new(Vec::new()) };
    static DOCK_ESTIMATE: Cell<Option<(Instant, Option<f64>)>> = const { Cell::new(None) };
    /// Ghostty doesn't report its caret, so it is followed from how the screen text changes.
    static TERMINAL: std::cell::RefCell<super::TerminalTracker> =
        std::cell::RefCell::new(super::TerminalTracker::with_cell(crate::data::settings::terminal_cell()));
}

fn scale() -> f64 { SCALE.with(Cell::get) }

fn mtm() -> MainThreadMarker { MainThreadMarker::new().expect("PawPal's platform calls run on the main thread") }

/// Height of the main screen; Cocoa measures up from its bottom, everything else down from its top.
fn primary_height() -> f64 {
    NSScreen::screens(mtm()).iter().next().map_or(0.0, |s| s.frame().size.height)
}

/// Converts a top-left rect in points to the app's physical pixels.
fn physical(r: CGRect) -> Rect {
    let k = scale();
    Rect::new(r.x * k, r.y * k, r.w * k, r.h * k)
}

pub fn shelf(walk: WalkArea) -> Option<Shelf> {
    let screens = NSScreen::screens(mtm());
    let screen = screens
        .iter()
        .find(|s| s.visibleFrame().origin.y - s.frame().origin.y > 10.0)
        .or_else(|| screens.iter().next())?;
    let (f, v) = (screen.frame(), screen.visibleFrame());
    let k = screen.backingScaleFactor();
    SCALE.with(|c| c.set(k));
    let ph = primary_height();
    // Flip Cocoa's bottom-left frames into top-left coordinates.
    let frame = Rect::new(f.origin.x, ph - f.origin.y - f.size.height, f.size.width, f.size.height);
    let top = ph - (v.origin.y + v.size.height); // stay below the menu bar, or macOS shifts the whole window down
    let dock_h = v.origin.y - f.origin.y;
    let mut out = Shelf { screen: frame, top, ground: frame.max_y(), min_x: frame.min_x(), max_x: frame.max_x(), dock: None };
    if dock_h > 10.0 {
        out.ground = frame.max_y() - dock_h + 3.0;
        let span = exact_dock_span().filter(|s| s.w > 40.0).map(|s| (s.x, s.x + s.w)).or_else(|| {
            estimated_dock_width().filter(|&w| w < frame.w - 40.0).map(|w| (frame.x + (frame.w - w) / 2.0, frame.x + (frame.w + w) / 2.0))
        });
        if let Some((lo, hi)) = span {
            out.dock = Some(Span { lo, hi });
            if walk == WalkArea::Dock { (out.min_x, out.max_x) = (lo, hi) }
        }
    } // otherwise the Dock is hidden or on the side, and pets walk along the bottom of the screen
    Some(Shelf {
        screen: Rect::new(out.screen.x * k, out.screen.y * k, out.screen.w * k, out.screen.h * k),
        top: out.top * k,
        ground: out.ground * k,
        min_x: out.min_x * k,
        max_x: out.max_x * k,
        dock: out.dock.map(|d| Span { lo: d.lo * k, hi: d.hi * k }),
    })
}

/// The Dock's real icon strip, read through Accessibility (only when the user has allowed it).
fn exact_dock_span() -> Option<CGRect> {
    if !accessibility_trusted() { return None }
    let dock = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str("com.apple.dock"));
    let pid = dock.iter().next()?.processIdentifier();
    let app = Owned(unsafe { AXUIElementCreateApplication(pid) });
    let children = app.attr("AXChildren")?;
    let list: CFArray<CFType> = unsafe { CFArray::wrap_under_get_rule(children.0 as CFArrayRef) };
    let first = list.iter().next()?;
    let el = first.as_CFTypeRef();
    let [x, y] = ax_value::<[f64; 2]>(attr(el, "AXPosition")?.0, AX_POINT)?;
    let [w, h] = ax_value::<[f64; 2]>(attr(el, "AXSize")?.0, AX_SIZE)?;
    Some(CGRect { x, y, w, h })
}

/// Estimates the Dock's width from its preferences, for when Accessibility isn't allowed.
fn estimated_dock_width() -> Option<f64> {
    // Reading the preferences file costs a little, and the Dock rarely changes; check every few seconds.
    if let Some((at, w)) = DOCK_ESTIMATE.with(Cell::get) {
        if at.elapsed().as_secs_f64() < 5.0 { return w }
    }
    let w = read_dock_width();
    DOCK_ESTIMATE.with(|c| c.set(Some((Instant::now(), w))));
    w
}

fn read_dock_width() -> Option<f64> {
    let file = dirs::home_dir()?.join("Library/Preferences/com.apple.dock.plist");
    let plist::Value::Dictionary(d) = plist::Value::from_file(file).ok()? else { return None };
    let mut tile = d.get("tilesize").and_then(|v| v.as_real().or_else(|| v.as_signed_integer().map(|i| i as f64))).unwrap_or(0.0);
    if tile <= 0.0 { tile = 48.0 }
    let apps = d.get("persistent-apps").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let others = d.get("persistent-others").and_then(|v| v.as_array()).map_or(0, |a| a.len());
    let ids: Vec<String> = apps
        .iter()
        .filter_map(|a| a.as_dictionary()?.get("tile-data")?.as_dictionary()?.get("bundle-identifier")?.as_string().map(String::from))
        .collect();
    let running = NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .filter(|a| {
            let id = a.bundleIdentifier().map(|s| s.to_string()).unwrap_or_default();
            a.activationPolicy() == NSApplicationActivationPolicy::Regular && !ids.contains(&id) && id != "com.apple.finder"
        })
        .count();
    let show_recents = d.get("show-recents").and_then(|v| v.as_boolean()).unwrap_or(true);
    let icons = 1 + apps.len() + running + others + 1;
    let separators = 1 + usize::from(show_recents && running > 0);
    Some(icons as f64 * tile + separators as f64 * 12.0 + 16.0)
}

pub fn cursor() -> Option<Point> {
    let m = NSEvent::mouseLocation();
    Some(pt(m.x * scale(), (primary_height() - m.y) * scale()))
}

/// Only the time since the last key press is read, never which key, so no permission is needed.
pub fn seconds_since_key() -> f64 {
    unsafe { CGEventSourceSecondsSinceLastEventType(0, 10) }
}

/// Terminals, where typing happens on the bottom lines of the window.
const TERMINALS: [&str; 7] = ["com.mitchellh.ghostty", "com.apple.Terminal", "com.googlecode.iterm2", "dev.warp.Warp-Stable",
                              "com.github.wez.wezterm", "net.kovidgoyal.kitty", "org.alacritty"];

/// Where the user is typing. `exact` is false when it's only a guess
/// (somewhere in the front window, or along a terminal's prompt row), as when Accessibility isn't allowed.
pub fn typing_spot() -> Option<(Rect, bool)> {
    let k = scale();
    if let Some(r) = caret() {
        let r = physical(r);
        return Some((Rect::new(r.x - 90.0 * k, r.y - 50.0 * k, r.w + 180.0 * k, r.h + 100.0 * k), true));
    }
    let w = physical(front_window()?);
    if TERMINALS.contains(&frontmost_app().as_str()) {
        // The prompt sits just above the bottom of what's visible of the window.
        let ph = primary_height();
        let visible_bottom = NSScreen::screens(mtm())
            .iter()
            .find(|s| { let f = s.frame(); physical(CGRect { x: f.origin.x, y: ph - f.origin.y - f.size.height, w: f.size.width, h: f.size.height }).intersects(&w) })
            .map_or(f64::INFINITY, |s| (ph - s.visibleFrame().origin.y) * k);
        let bottom = (w.y + w.h).min(visible_bottom);
        return Some((Rect::new(w.x, bottom - 120.0 * k, w.w, 120.0 * k), false));
    }
    Some((w, false))
}

/// The caret, or the focused text field when the app can't report a caret.
fn caret() -> Option<CGRect> {
    if !accessibility_trusted() { return None }
    let element = focused_element()?;
    if let Some(range) = element.attr("AXSelectedTextRange") {
        let mut bounds: CFTypeRef = std::ptr::null();
        let attr = CFString::from_static_string("AXBoundsForRange");
        let err = unsafe { AXUIElementCopyParameterizedAttributeValue(element.0, attr.as_concrete_TypeRef(), range.0, &mut bounds) };
        if err == 0 && !bounds.is_null() {
            let bounds = Owned(bounds);
            if let Some(r) = ax_value::<CGRect>(bounds.0, AX_RECT).filter(|r| r.h > 0.0) { return Some(r) }
        }
    }
    let [x, y] = ax_value::<[f64; 2]>(element.attr("AXPosition")?.0, AX_POINT)?;
    let [w, h] = ax_value::<[f64; 2]>(element.attr("AXSize")?.0, AX_SIZE)?;
    // Terminals that can't place the caret still show their screen text: guess from that.
    if TERMINALS.contains(&frontmost_app().as_str()) {
        let text = element.attr("AXValue").filter(|v| unsafe { CFGetTypeID(v.0) == CFStringGetTypeID() });
        if let Some(text) = text {
            let text = unsafe { CFString::wrap_under_get_rule(text.0 as CFStringRef) }.to_string();
            let guess = TERMINAL.with(|t| {
                let mut t = t.borrow_mut();
                let learned = t.cell();
                let guess = t.update(&text, Rect::new(x, y, w, h));
                if t.cell() != learned { crate::data::settings::set_terminal_cell(t.cell()) }
                guess
            });
            if let Some(r) = guess { return Some(CGRect { x: r.x, y: r.y, w: r.w, h: r.h }) }
        }
    }
    // A small text field is fine to avoid whole; a big text view isn't a useful spot.
    (h < 200.0).then_some(CGRect { x, y, w, h })
}

fn focused_element() -> Option<Owned> {
    let system = Owned(unsafe { AXUIElementCreateSystemWide() });
    let el = system.attr("AXFocusedUIElement")?;
    (unsafe { CFGetTypeID(el.0) } == unsafe { AXUIElementGetTypeID() }).then_some(el)
}

/// What Accessibility reports for the focused element, for checking why a caret wasn't found.
pub fn describe_typing() -> String {
    if !accessibility_trusted() { return "accessibility: not allowed".into() }
    let Some(el) = focused_element() else { return "accessibility: allowed, no focused element".into() };
    let role = el.attr("AXRole").map(|r| unsafe { CFString::wrap_under_get_rule(r.0 as CFStringRef) }.to_string()).unwrap_or("?".into());
    let range = el.attr("AXSelectedTextRange").is_some();
    let caret = caret();
    let cell = TERMINAL.with(|t| t.borrow().cell());
    format!("accessibility: allowed, role={role}, selectedRange={range}, caret={caret:?}, terminal cell={cell:?}")
}

/// The frontmost app's front window; its bounds are readable without any permission.
/// The on-screen, normal-level windows of the front app (not ours), front to back.
fn front_windows() -> Vec<CGRect> {
    let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else { return vec![] };
    let pid = app.processIdentifier();
    if pid == std::process::id() as i32 { return vec![] }
    let list = unsafe { CGWindowListCopyWindowInfo(1 | 1 << 4, 0) };
    if list.is_null() { return vec![] }
    let list: CFArray<CFDictionary<CFString, CFType>> = unsafe { CFArray::wrap_under_create_rule(list) };
    let num = |d: &CFDictionary<CFString, CFType>, k: &'static str| {
        d.find(&CFString::from_static_string(k)).and_then(|v| v.downcast::<CFNumber>()).and_then(|n| n.to_i64())
    };
    let mut out = vec![];
    for d in list.iter() {
        if num(&d, "kCGWindowOwnerPID") != Some(pid as i64) || num(&d, "kCGWindowLayer") != Some(0) { continue }
        let Some(b) = d.find(&CFString::from_static_string("kCGWindowBounds")) else { continue };
        let mut r = CGRect::default();
        if unsafe { CGRectMakeWithDictionaryRepresentation(b.as_CFTypeRef() as CFDictionaryRef, &mut r) } { out.push(r) }
    }
    out
}

fn front_window() -> Option<CGRect> { front_windows().into_iter().next() }

/// Screen frames in window-list coordinates (top-left origin).
fn screen_rects() -> Vec<CGRect> {
    let ph = primary_height();
    NSScreen::screens(mtm()).iter().map(|s| {
        let f = s.frame();
        CGRect { x: f.origin.x, y: ph - f.origin.y - f.size.height, w: f.size.width, h: f.size.height }
    }).collect()
}

/// Whether any of the front app's windows covers a whole screen, as a full-screen video or app does.
/// Browsers list small helper windows too, so every window counts, not just the first.
pub fn fullscreen_active() -> bool {
    let (windows, screens) = (front_windows(), screen_rects());
    let covers = |w: &CGRect, f: &CGRect| {
        w.x <= f.x + 2.0 && w.y <= f.y + 2.0 && w.x + w.w >= f.x + f.w - 2.0 && w.y + w.h >= f.y + f.h - 2.0
    };
    let on = windows.iter().any(|w| screens.iter().any(|f| covers(w, f)));
    log_fullscreen(on, &windows, &screens);
    on
}

/// Writes what the full-screen check saw when the front app changes, for debugging.
fn log_fullscreen(on: bool, windows: &[CGRect], screens: &[CGRect]) {
    thread_local! { static LAST: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) }; }
    let line = format!("app={} fullscreen={on} windows={windows:?} screens={screens:?}\n", frontmost_app());
    if LAST.with(|l| std::mem::replace(&mut *l.borrow_mut(), line.clone())) == line { return }
    if let Some(dir) = crate::data::skin::root().parent() { let _ = std::fs::write(dir.join("fullscreen-debug.txt"), line); }
}

pub fn frontmost_app() -> String {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .and_then(|a| a.bundleIdentifier())
        .map_or_else(|| "-".into(), |s| s.to_string())
}

pub fn accessibility_trusted() -> bool { unsafe { AXIsProcessTrusted() } }

/// Whether this OS has a permission the menu should offer to turn on.
pub fn has_accessibility_setting() -> bool { true }

pub fn request_accessibility() {
    let options = CFDictionary::from_CFType_pairs(&[(CFString::from_static_string("AXTrustedCheckOptionPrompt"), CFBoolean::true_value())]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };
}

/// Puts the window just above the Dock, on every Space, without a shadow.
pub fn prepare_window(window: &Window) {
    let Some(view) = ns_view(window) else { return };
    let view: &NSView = unsafe { view.cast::<NSView>().as_ref() };
    let Some(win) = view.window() else { return };
    win.setLevel(20 + 1); // kCGDockWindowLevel + 1
    win.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );
    win.setHidesOnDeactivate(false);
    win.setHasShadow(false);
}

/// Moves and resizes the window in one step, keeping its bottom edge put; winit does it in two
/// (move, then resize), and the screen can show the old picture shifted in between.
pub fn set_frame(window: &Window, r: Rect) -> bool {
    let Some(view) = ns_view(window) else { return false };
    let view: &NSView = unsafe { view.cast::<NSView>().as_ref() };
    let Some(win) = view.window() else { return false };
    pin_contents_bottom_left(view);
    let k = scale();
    let frame = NSRect::new(NSPoint::new(r.x / k, primary_height() - r.max_y() / k), NSSize::new(r.w / k, r.h / k));
    win.setFrame_display(frame, false);
    true
}

/// Until the next frame is drawn at the new size, macOS shows the old one stretched to fit the window,
/// which squashes the pets for a moment. Everything is drawn up from the window's bottom edge, so
/// keeping the old frame unscaled at the bottom-left shows it exactly where it belongs. The GL picture
/// lives in a sublayer, which gets the same treatment.
fn pin_contents_bottom_left(view: &NSView) {
    view.setLayerContentsPlacement(objc2_app_kit::NSViewLayerContentsPlacement::BottomLeft);
    let gravity = NSString::from_str("bottomLeft");
    unsafe {
        let layer: *mut objc2::runtime::AnyObject = objc2::msg_send![view, layer];
        let Some(layer) = layer.as_ref() else { return };
        let _: () = objc2::msg_send![layer, setContentsGravity: &*gravity];
        let sublayers: *mut objc2_foundation::NSArray<objc2::runtime::AnyObject> = objc2::msg_send![layer, sublayers];
        if let Some(sublayers) = sublayers.as_ref() {
            for sub in sublayers.iter() {
                let _: () = objc2::msg_send![&*sub, setContentsGravity: &*gravity];
            }
        }
    }
}

/// Fades the window fully in (1) or out (0); an invisible window can be resized without anyone seeing it.
pub fn set_alpha(window: &Window, alpha: f64) -> bool {
    let Some(view) = ns_view(window) else { return false };
    let view: &NSView = unsafe { view.cast::<NSView>().as_ref() };
    let Some(win) = view.window() else { return false };
    win.setAlphaValue(alpha);
    true
}

fn ns_view(window: &Window) -> Option<std::ptr::NonNull<c_void>> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::AppKit(h) => Some(h.ns_view),
        _ => None,
    }
}

/// Called when the user presses on a pet: clicking our window makes PawPal the active app.
pub fn will_interact() {}

/// Gives focus back to the app the user was in, so petting a cat doesn't interrupt their typing.
pub fn done_interacting() {
    let pid = PREVIOUS_APP.with(Cell::get);
    if pid == 0 { return }
    if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        #[allow(deprecated)]
        app.activateWithOptions(NSApplicationActivationOptions::empty());
    }
}

/// Remembers the app in front (called regularly), unless it's PawPal itself.
pub fn note_frontmost() {
    if let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() {
        let pid = app.processIdentifier();
        if pid != std::process::id() as i32 {
            PREVIOUS_APP.with(|c| c.set(pid));
            wake_web_accessibility(pid);
        }
    }
}

/// Chrome and Electron apps (WhatsApp Web, Slack…) build the Accessibility tree of their web pages, and so
/// report the text cursor, only once an assistive app asks with AXManualAccessibility. Other apps ignore it.
fn wake_web_accessibility(pid: i32) {
    if WOKEN.with(|w| w.borrow().contains(&pid)) || !accessibility_trusted() { return }
    WOKEN.with(|w| w.borrow_mut().push(pid));
    let app = Owned(unsafe { AXUIElementCreateApplication(pid) });
    let attr = CFString::from_static_string("AXManualAccessibility");
    unsafe { AXUIElementSetAttributeValue(app.0, attr.as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef()) };
}

pub fn show_context_menu(window: &Window, menu: &tray_icon::menu::Menu) {
    use tray_icon::menu::ContextMenu;
    if let Some(view) = ns_view(window) {
        unsafe { menu.show_context_menu_for_nsview(view.as_ptr(), None) };
    }
}

/// Asks for a line of text with a native dialog; None when cancelled.
pub fn prompt_text(title: &str, message: &str, default: &str) -> Option<String> {
    let q = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "text returned of (display dialog \"{}\" with title \"{}\" default answer \"{}\" buttons {{\"Cancel\", \"OK\"}} default button \"OK\")",
        q(message), q(title), q(default)
    );
    let out = Command::new("osascript").args(["-e", &script]).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string())
}

/// Subject lifting isn't wired up natively yet; the importer falls back to removing plain backgrounds.
pub fn lift_subject(_img: &RgbaImage) -> Option<Option<RgbaImage>> { None }

/// A Core Foundation object this code owns (released on drop).
struct Owned(CFTypeRef);

impl Owned {
    fn attr(&self, name: &'static str) -> Option<Owned> { attr(self.0, name) }
}

/// Copies an Accessibility attribute of an element.
fn attr(el: CFTypeRef, name: &'static str) -> Option<Owned> {
    if el.is_null() { return None }
    let mut v: CFTypeRef = std::ptr::null();
    let key = CFString::from_static_string(name);
    let err = unsafe { AXUIElementCopyAttributeValue(el, key.as_concrete_TypeRef(), &mut v) };
    (err == 0 && !v.is_null()).then(|| Owned(v))
}

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() { unsafe { CFRelease(self.0) } }
    }
}

fn ax_value<T: Default>(v: CFTypeRef, kind: u32) -> Option<T> {
    if v.is_null() || unsafe { CFGetTypeID(v) } != unsafe { AXValueGetTypeID() } { return None }
    let mut out = T::default();
    unsafe { AXValueGetValue(v, kind, &mut out as *mut T as *mut c_void) }.then_some(out)
}

// Calendar

thread_local! {
    static EVENTS: std::cell::OnceCell<Option<objc2::rc::Retained<objc2_event_kit::EKEventStore>>> = const { std::cell::OnceCell::new() };
}

/// Calendar heads-ups work here: events come from the Calendar app's accounts (iCloud, Google, Exchange…).
pub fn has_calendar() -> bool { true }

/// macOS ends any app that asks for calendar access without saying why in its Info.plist,
/// so a bare binary (cargo run) never asks.
fn can_ask_for_calendar() -> bool {
    objc2_foundation::NSBundle::mainBundle()
        .objectForInfoDictionaryKey(&NSString::from_str("NSCalendarsFullAccessUsageDescription"))
        .is_some()
}

/// Events starting in the next `within` seconds. The first call asks for calendar access; until it's
/// granted (or if it's refused) there are none.
pub fn upcoming_events(within: f64) -> Vec<crate::domain::CalendarEvent> {
    use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEventStore};
    use objc2_foundation::{NSDate, NSObjectProtocol};
    EVENTS.with(|cell| {
        let Some(store) = cell.get_or_init(|| can_ask_for_calendar().then(|| unsafe { EKEventStore::new() })) else { return vec![] };
        let status = unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
        if status == EKAuthorizationStatus::NotDetermined {
            let done = block2::RcBlock::new(|_granted: objc2::runtime::Bool, _err: *mut objc2_foundation::NSError| {});
            let ptr = block2::RcBlock::as_ptr(&done);
            unsafe {
                if store.respondsToSelector(objc2::sel!(requestFullAccessToEventsWithCompletion:)) {
                    store.requestFullAccessToEventsWithCompletion(ptr);
                } else {
                    // macOS 13, which has only the older request.
                    #[allow(deprecated)]
                    store.requestAccessToEntityType_completion(EKEntityType::Event, ptr);
                }
            }
            return vec![];
        }
        if status != EKAuthorizationStatus::FullAccess { return vec![] }
        unsafe {
            let (from, to) = (NSDate::now(), NSDate::dateWithTimeIntervalSinceNow(within));
            let query = store.predicateForEventsWithStartDate_endDate_calendars(&from, &to, None);
            store
                .eventsMatchingPredicate(&query)
                .iter()
                .filter(|e| !e.isAllDay())
                .map(|e| {
                    let start = e.startDate().timeIntervalSince1970();
                    let id = e.eventIdentifier().map_or_else(String::new, |s| s.to_string());
                    crate::domain::CalendarEvent { id: format!("{id}@{start}"), title: e.title().to_string(), start }
                })
                .collect()
        }
    })
}

/// A 32-bit Core Audio property of an audio object (1 is the system object).
fn audio_u32(id: u32, selector: &[u8; 4]) -> Option<u32> {
    let addr = address(selector);
    let (mut value, mut size) = (0u32, 4u32);
    // SAFETY: the property is a UInt32 and `value` has room for it.
    let err = unsafe { AudioObjectGetPropertyData(id, &addr, 0, std::ptr::null(), &mut size, &mut value as *mut u32 as *mut c_void) };
    (err == 0).then_some(value)
}

/// Whether some app is using the default input ("dIn ") or output ("dOut") device right now.
fn audio_busy(default_device: &[u8; 4]) -> bool {
    audio_u32(1, default_device).filter(|&d| d != 0).and_then(|d| audio_u32(d, b"gone")).is_some_and(|on| on != 0)
}

/// An app has the microphone open, as during a call (even muted).
fn mic_in_use() -> bool { audio_busy(b"dIn ") }

fn address(selector: &[u8; 4]) -> AudioAddress { AudioAddress { selector: u32::from_be_bytes(*selector), scope: u32::from_be_bytes(*b"glob"), element: 0 } }

/// Some app has a camera running.
fn camera_in_use() -> bool {
    let devices = address(b"dev#");
    let mut size = 0u32;
    // SAFETY: the device list is an array of UInt32 ids, read into a buffer of the size asked for.
    if unsafe { CMIOObjectGetPropertyDataSize(1, &devices, 0, std::ptr::null(), &mut size) } != 0 || size == 0 { return false }
    let mut ids = vec![0u32; size as usize / 4];
    let mut used = 0u32;
    if unsafe { CMIOObjectGetPropertyData(1, &devices, 0, std::ptr::null(), size, &mut used, ids.as_mut_ptr() as *mut c_void) } != 0 { return false }
    ids.truncate(used as usize / 4);
    ids.iter().any(|&id| {
        let (mut on, mut used) = (0u32, 0u32);
        // SAFETY: "is running somewhere" is a UInt32.
        let err = unsafe { CMIOObjectGetPropertyData(id, &address(b"gone"), 0, std::ptr::null(), 4, &mut used, &mut on as *mut u32 as *mut c_void) };
        err == 0 && on != 0
    })
}

/// Browsers (and web apps installed from them) whose window titles show the current tab.
const BROWSERS: [&str; 6] = ["com.google.Chrome", "com.microsoft.edgemac", "com.brave.Browser", "company.thebrowser.Browser", "org.chromium.Chromium", "com.apple.Safari"];

/// A browser window is showing a Google Meet call (needs Accessibility to read the titles).
fn meeting_window_open() -> bool {
    if !accessibility_trusted() { return false }
    NSWorkspace::sharedWorkspace().runningApplications().iter().any(|a| {
        let id = a.bundleIdentifier().map(|s| s.to_string()).unwrap_or_default();
        if !BROWSERS.iter().any(|b| id.starts_with(b)) { return false }
        let app = Owned(unsafe { AXUIElementCreateApplication(a.processIdentifier()) });
        let Some(windows) = app.attr("AXWindows") else { return false };
        let windows: CFArray<CFType> = unsafe { CFArray::wrap_under_get_rule(windows.0 as CFArrayRef) };
        windows.iter().any(|w| {
            attr(w.as_CFTypeRef(), "AXTitle").is_some_and(|t| {
                let is_text = unsafe { CFGetTypeID(t.0) == CFStringGetTypeID() };
                is_text && super::is_meeting_title(&unsafe { CFString::wrap_under_get_rule(t.0 as CFStringRef) }.to_string())
            })
        })
    })
}

/// The user is in a call: the mic or a camera is on, or a Meet call is open in a browser (Meet may leave
/// the mic closed while muted).
pub fn in_call() -> bool { mic_in_use() || camera_in_use() || meeting_window_open() }

/// Some app is playing sound.
pub fn audio_playing() -> bool { audio_busy(b"dOut") }
