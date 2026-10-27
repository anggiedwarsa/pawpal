//! Windows: pets stand on the taskbar; the caret comes from the focused thread (Win32 apps report it),
//! and the overlay window never takes focus.

use std::cell::RefCell;
use std::ffi::c_void;
use std::process::Command;

use image::RgbaImage;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{CloseHandle, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, MonitorFromRect, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows_sys::Win32::UI::Shell::{SHAppBarMessage, ABM_GETTASKBARPOS, APPBARDATA};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetShellWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, SetWindowLongPtrW,
    GUITHREADINFO, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use winit::window::Window;

use super::{KeyGuess, Shelf};
use crate::data::settings::WalkArea;
use crate::support::{pt, Point, Rect};

thread_local! {
    static KEYS: RefCell<KeyGuess> = const { RefCell::new(KeyGuess::new()) };
}

fn rect(r: RECT) -> Rect { Rect::new(r.left as f64, r.top as f64, (r.right - r.left) as f64, (r.bottom - r.top) as f64) }

pub fn shelf(_walk: WalkArea) -> Option<Shelf> {
    // The taskbar's monitor; its work area ends where the taskbar starts (or at the screen edge when it auto-hides).
    let mut bar: APPBARDATA = unsafe { std::mem::zeroed() };
    bar.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut bar) };
    let monitor = unsafe { MonitorFromRect(&bar.rc, MONITOR_DEFAULTTOPRIMARY) };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 { return None }
    let (screen, work) = (rect(info.rcMonitor), rect(info.rcWork));
    // Windows doesn't say where the taskbar icons are, so pets use its whole width.
    Some(Shelf { screen, top: work.min_y(), ground: work.max_y(), min_x: screen.min_x(), max_x: screen.max_x(), dock: None })
}

pub fn cursor() -> Option<Point> {
    let mut p = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut p) } != 0).then(|| pt(p.x as f64, p.y as f64))
}

/// Windows only reports the time since any input, so a key press is input without the cursor moving.
pub fn seconds_since_key() -> f64 {
    let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    if unsafe { GetLastInputInfo(&mut info) } == 0 { return f64::INFINITY }
    let idle = unsafe { GetTickCount() }.wrapping_sub(info.dwTime) as f64 / 1000.0;
    KEYS.with(|k| k.borrow_mut().update(idle, cursor()))
}

/// Terminals, where typing happens on the bottom lines of the window.
const TERMINALS: [&str; 9] = ["windowsterminal.exe", "conhost.exe", "cmd.exe", "powershell.exe", "pwsh.exe", "wezterm-gui.exe",
                              "alacritty.exe", "mintty.exe", "wt.exe"];

/// Where the user is typing. `exact` is false when it's only the front window or a terminal's prompt row.
pub fn typing_spot() -> Option<(Rect, bool)> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() { return None }
    let mut pid = 0;
    let thread = unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid == std::process::id() { return None }

    let mut gui: GUITHREADINFO = unsafe { std::mem::zeroed() };
    gui.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if unsafe { GetGUIThreadInfo(thread, &mut gui) } != 0 && !gui.hwndCaret.is_null() {
        let mut tl = POINT { x: gui.rcCaret.left, y: gui.rcCaret.top };
        let mut br = POINT { x: gui.rcCaret.right, y: gui.rcCaret.bottom };
        unsafe {
            ClientToScreen(gui.hwndCaret, &mut tl);
            ClientToScreen(gui.hwndCaret, &mut br);
        }
        let r = Rect::new(tl.x as f64, tl.y as f64, (br.x - tl.x).max(1) as f64, (br.y - tl.y).max(1) as f64);
        return Some((Rect::new(r.x - 90.0, r.y - 50.0, r.w + 180.0, r.h + 100.0), true));
    }

    let mut wr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetWindowRect(hwnd, &mut wr) } == 0 { return None }
    let w = rect(wr);
    if TERMINALS.contains(&process_name(pid).as_str()) {
        return Some((Rect::new(w.x, w.max_y() - 120.0, w.w, 120.0), false));
    }
    Some((w, false))
}

fn process_name(pid: u32) -> String {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() { return String::new() }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len) } != 0;
    unsafe { CloseHandle(handle) };
    if !ok { return String::new() }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    path.rsplit('\\').next().unwrap_or("").to_lowercase()
}

pub fn describe_typing() -> String { format!("caret={:?}", typing_spot()) }

/// Whether the foreground window covers its whole monitor, as a full-screen video or game does.
/// The desktop itself is a full-monitor window too, so it doesn't count.
pub fn fullscreen_active() -> bool {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() || hwnd == unsafe { GetShellWindow() } { return false }
    let mut class = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) };
    if matches!(String::from_utf16_lossy(&class[..n.max(0) as usize]).as_str(), "Progman" | "WorkerW") { return false }
    let mut wr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetWindowRect(hwnd, &mut wr) } == 0 { return false }
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info) } == 0 { return false }
    let m = info.rcMonitor;
    wr.left <= m.left && wr.top <= m.top && wr.right >= m.right && wr.bottom >= m.bottom
}

pub fn frontmost_app() -> String {
    let hwnd = unsafe { GetForegroundWindow() };
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    process_name(pid)
}

/// Windows needs no permission to find the caret.
pub fn accessibility_trusted() -> bool { true }
pub fn has_accessibility_setting() -> bool { false }
pub fn request_accessibility() {}

fn hwnd(window: &Window) -> Option<*mut c_void> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as *mut c_void),
        _ => None,
    }
}

/// Moving and resizing at once needs nothing special here: the app falls back to winit.
pub fn set_frame(_window: &Window, _r: Rect) -> bool { false }

/// Window see-through-ness isn't set here, so the app resizes its one window in place.
pub fn set_alpha(_window: &Window, _alpha: f64) -> bool { false }

/// Keeps the overlay out of Alt-Tab and stops clicks on pets from taking focus from the user's app.
pub fn prepare_window(window: &Window) {
    let Some(h) = hwnd(window) else { return };
    unsafe {
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        SetWindowLongPtrW(h, GWL_EXSTYLE, ex | (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) as isize);
    }
}

pub fn will_interact() {}
pub fn done_interacting() {}
pub fn note_frontmost() {}

pub fn show_context_menu(window: &Window, menu: &tray_icon::menu::Menu) {
    use tray_icon::menu::ContextMenu;
    if let Some(h) = hwnd(window) {
        unsafe { menu.show_context_menu_for_hwnd(h as isize, None) };
    }
}

/// Asks for a line of text with a native dialog; None when cancelled.
pub fn prompt_text(title: &str, message: &str, default: &str) -> Option<String> {
    let q = |s: &str| s.replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.Interaction]::InputBox('{}', '{}', '{}')",
        q(message), q(title), q(default)
    );
    let out = Command::new("powershell").args(["-NoProfile", "-Command", &script]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

pub fn lift_subject(_img: &RgbaImage) -> Option<Option<RgbaImage>> { None }

/// No calendar to read here yet (an iCal feed could be added); no heads-ups.
pub fn has_calendar() -> bool { false }
pub fn upcoming_events(_within: f64) -> Vec<crate::domain::CalendarEvent> { Vec::new() }

/// Whether some app is using the microphone or webcam right now, from the privacy settings' "in use" records:
/// each app that used the device has a key whose LastUsedTimeStop is 0 while it still has it open.
fn device_in_use(device: &str) -> bool {
    use windows_sys::Win32::System::Registry::{RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_QWORD};
    fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
    // Looks at the key's own record, then its subkeys' (desktop apps sit one level down, under NonPackaged).
    fn any_open(key: HKEY, depth: u32) -> bool {
        let (mut stop, mut size) = (u64::MAX, 8u32);
        let name = wide("LastUsedTimeStop");
        let got = unsafe { RegGetValueW(key, std::ptr::null(), name.as_ptr(), RRF_RT_QWORD, std::ptr::null_mut(), &mut stop as *mut u64 as *mut c_void, &mut size) };
        if got == 0 && stop == 0 { return true }
        if depth == 0 { return false }
        for i in 0.. {
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let r = unsafe { RegEnumKeyExW(key, i, buf.as_mut_ptr(), &mut len, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) };
            if r != 0 { break }
            let mut sub: HKEY = std::ptr::null_mut();
            if unsafe { RegOpenKeyExW(key, buf.as_ptr(), 0, KEY_READ, &mut sub) } != 0 { continue }
            let open = any_open(sub, depth - 1);
            unsafe { RegCloseKey(sub) };
            if open { return true }
        }
        false
    }
    let path = wide(&format!("Software\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\{device}"));
    let mut key: HKEY = std::ptr::null_mut();
    if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, KEY_READ, &mut key) } != 0 { return false }
    let open = any_open(key, 2);
    unsafe { RegCloseKey(key) };
    open
}

/// Some top-level window's title is a Google Meet call.
fn meeting_window_open() -> bool {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsWindowVisible};
    unsafe extern "system" fn look(hwnd: HWND, found: LPARAM) -> BOOL {
        if IsWindowVisible(hwnd) == 0 { return 1 }
        let mut buf = [0u16; 256];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if n > 0 && super::is_meeting_title(&String::from_utf16_lossy(&buf[..n as usize])) {
            *(found as *mut bool) = true;
            return 0;
        }
        1
    }
    let mut found = false;
    // SAFETY: `look` only writes to `found`, which outlives the call.
    unsafe { EnumWindows(Some(look), &mut found as *mut bool as LPARAM) };
    found
}

/// The user is in a call: the mic or webcam is on, or a Meet call is open (Meet may leave the mic closed while muted).
pub fn in_call() -> bool { device_in_use("microphone") || device_in_use("webcam") || meeting_window_open() }

/// Some app is playing sound: the default speakers' peak level right now is above silence.
pub fn audio_playing() -> bool {
    use windows_sys::core::GUID;
    use windows_sys::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
    const MM_DEVICE_ENUMERATOR: GUID = GUID::from_u128(0xBCDE0395_E52F_467C_8E3D_C4579291692E);
    const IMM_DEVICE_ENUMERATOR: GUID = GUID::from_u128(0xA95664D2_9614_4F35_A746_DE8DB63617E6);
    const IAUDIO_METER_INFORMATION: GUID = GUID::from_u128(0xC02216F6_8C67_4B5B_9D00_D008E73E0064);
    /// The `i`th method of a COM object's vtable.
    unsafe fn method<F: Copy>(obj: *mut c_void, i: usize) -> F { std::mem::transmute_copy(&*(*(obj as *const *const usize)).add(i)) }
    unsafe fn release(obj: *mut c_void) { method::<unsafe extern "system" fn(*mut c_void) -> u32>(obj, 2)(obj); }
    // SAFETY: each call follows the interface's vtable layout (IUnknown's three methods first), and every
    // object made here is released before returning.
    unsafe {
        // Already initialised on this thread (winit does), which is fine.
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let mut enumerator: *mut c_void = std::ptr::null_mut();
        if CoCreateInstance(&MM_DEVICE_ENUMERATOR, std::ptr::null_mut(), CLSCTX_ALL, &IMM_DEVICE_ENUMERATOR, &mut enumerator) < 0 { return false }
        // IMMDeviceEnumerator::GetDefaultAudioEndpoint(eRender, eConsole)
        let mut device: *mut c_void = std::ptr::null_mut();
        let got = method::<unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut c_void) -> i32>(enumerator, 4)(enumerator, 0, 0, &mut device);
        release(enumerator);
        if got < 0 { return false }
        // IMMDevice::Activate(IAudioMeterInformation)
        let mut meter: *mut c_void = std::ptr::null_mut();
        let got = method::<unsafe extern "system" fn(*mut c_void, *const GUID, u32, *mut c_void, *mut *mut c_void) -> i32>(device, 3)(
            device, &IAUDIO_METER_INFORMATION, CLSCTX_ALL, std::ptr::null_mut(), &mut meter,
        );
        release(device);
        if got < 0 { return false }
        // IAudioMeterInformation::GetPeakValue
        let mut peak = 0f32;
        let got = method::<unsafe extern "system" fn(*mut c_void, *mut f32) -> i32>(meter, 3)(meter, &mut peak);
        release(meter);
        got >= 0 && peak > 0.001
    }
}
