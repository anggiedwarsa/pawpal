//! User preferences, stored as JSON in the platform's config folder.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use crate::domain::{Coat, Species};
use crate::engine::alarms::Alarm;
use crate::engine::breaks::DEFAULT_BREAK_EVERY;
use crate::engine::rhythm::DEFAULT_BEDTIME;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalkArea {
    #[default]
    Dock,
    Screen,
}

/// Something the user jotted down to do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub text: String,
}

/// What got done one day (local date, `YYYY-MM-DD`), kept for the weekly summary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DayRecord {
    pub date: String,
    /// Ids of the meetings seen under way, so each counts once even across restarts.
    pub meetings: Vec<String>,
    pub focus_sessions: u32,
    pub focus_minutes: f64,
    pub breaks: u32,
    pub todos_done: u32,
}

/// Days kept for the weekly summary.
const KEEP_DAYS: usize = 14;

/// A pet as saved between launches.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PetSave {
    pub name: String,
    pub species: Species,
    /// Name of the pet's clip skin.
    pub skin: Option<String>,
    pub hunger: f64,
    pub energy: f64,
    pub happiness: f64,
    /// Scale colour (dragons); missing in pets saved before colours existed.
    #[serde(default)]
    pub coat: Coat,
}

#[derive(Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Prefs {
    scale: f64,
    auto: bool,
    visit_idle_cursor: bool,
    /// Pets run off when the cursor comes near.
    avoid_cursor: bool,
    walk_area: WalkArea,
    pets: Vec<PetSave>,
    /// Skins whose clips are shown mirrored.
    flipped: BTreeSet<String>,
    asked_accessibility: Option<String>,
    /// A pet comes out 10, 5 and 1 minute before calendar events.
    calendar_heads_ups: bool,
    /// Minutes at the computer before a pet suggests a break; 0 is off.
    break_every: u32,
    /// When pets go to bed for the night (local hour); none means no daily rhythm.
    bedtime: Option<u32>,
    /// Reminders set from the tray that haven't come up yet.
    alarms: Vec<Alarm>,
    /// Pets go to bed while the user is in a call.
    focus_in_calls: bool,
    /// Pets like to dance while sound is playing.
    dance_to_music: bool,
    /// Snow, leaves and such drift down now and then.
    seasonal: bool,
    /// Pets sometimes hop up onto the front window.
    perch_on_windows: bool,
    /// Look for a newer release on GitHub once a day.
    check_updates: bool,
    /// A private iCal link to read calendar events from, besides (or instead of) the system's calendar.
    calendar_url: Option<String>,
    /// A pet tells the day's meetings the first time the user sits down in the morning.
    morning_agenda: bool,
    /// A pet offers focus mode when there's a good while free before the next meeting.
    focus_offers: bool,
    /// Heads-ups mention when a meeting runs straight into the next.
    back_to_back: bool,
    /// When (local hour) a pet suggests wrapping up for the day; none is off.
    wrap_up: Option<u32>,
    /// A pet says when the user's late to a meeting or one is running over.
    late_nudges: bool,
    /// On Fridays, a pet sums up the week.
    weekly_summary: bool,
    /// The date of the last weekly summary, so it comes once.
    last_weekly: Option<String>,
    todos: Vec<Todo>,
    /// The last couple of weeks' tallies, oldest first.
    days: Vec<DayRecord>,
    /// Ghostty's character cell (points), learned from its screen text since it doesn't report the caret.
    terminal_cell: Option<(f64, f64)>,
}

impl Default for Prefs {
    fn default() -> Prefs {
        Prefs {
            scale: 1.1,
            auto: true,
            visit_idle_cursor: false,
            avoid_cursor: false,
            walk_area: WalkArea::Dock,
            pets: Vec::new(),
            flipped: BTreeSet::new(),
            asked_accessibility: None,
            calendar_heads_ups: true,
            break_every: DEFAULT_BREAK_EVERY,
            bedtime: Some(DEFAULT_BEDTIME),
            alarms: Vec::new(),
            focus_in_calls: true,
            dance_to_music: true,
            seasonal: true,
            perch_on_windows: true,
            check_updates: true,
            calendar_url: None,
            morning_agenda: true,
            focus_offers: true,
            back_to_back: true,
            wrap_up: None,
            late_nudges: true,
            weekly_summary: true,
            last_weekly: None,
            todos: Vec::new(),
            days: Vec::new(),
            terminal_cell: None,
        }
    }
}

static PREFS: LazyLock<Mutex<Prefs>> = LazyLock::new(|| Mutex::new(load()));

fn path() -> PathBuf {
    // Tests get a throwaway file, so they never touch the user's real settings.
    if cfg!(test) {
        return std::env::temp_dir().join(format!("pawpal-test-{}", std::process::id())).join("settings.json");
    }
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("PawPal").join("settings.json")
}

fn load() -> Prefs {
    if let Ok(text) = std::fs::read_to_string(path()) {
        if let Ok(p) = serde_json::from_str(&text) {
            return p;
        }
    }
    #[cfg(all(target_os = "macos", not(test)))]
    if let Some(p) = migrate_from_user_defaults() {
        return p;
    }
    Prefs::default()
}

fn prefs() -> MutexGuard<'static, Prefs> {
    PREFS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Changes preferences and writes them straight to disk.
fn update(f: impl FnOnce(&mut Prefs)) {
    let mut p = prefs();
    f(&mut p);
    let file = path();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(&*p) {
        let _ = std::fs::write(file, text);
    }
}

pub fn scale() -> f64 { let v = prefs().scale; if v > 0.0 { v } else { 1.1 } }
pub fn set_scale(v: f64) { update(|p| p.scale = v) }
pub fn auto() -> bool { prefs().auto }
pub fn set_auto(v: bool) { update(|p| p.auto = v) }
pub fn visit_idle_cursor() -> bool { prefs().visit_idle_cursor }
pub fn set_visit_idle_cursor(v: bool) { update(|p| p.visit_idle_cursor = v) }
pub fn avoid_cursor() -> bool { prefs().avoid_cursor }
pub fn set_avoid_cursor(v: bool) { update(|p| p.avoid_cursor = v) }
pub fn walk_area() -> WalkArea { prefs().walk_area }
pub fn set_walk_area(v: WalkArea) { update(|p| p.walk_area = v) }
pub fn pets() -> Vec<PetSave> { prefs().pets.clone() }
pub fn set_pets(v: Vec<PetSave>) { update(|p| p.pets = v) }
pub fn flipped(skin: &str) -> bool { prefs().flipped.contains(skin) }
pub fn toggle_flipped(skin: &str) {
    update(|p| {
        if !p.flipped.remove(skin) {
            p.flipped.insert(skin.to_string());
        }
    })
}
pub fn asked_accessibility() -> Option<String> { prefs().asked_accessibility.clone() }
pub fn set_asked_accessibility(v: String) { update(|p| p.asked_accessibility = Some(v)) }
pub fn calendar_heads_ups() -> bool { prefs().calendar_heads_ups }
pub fn set_calendar_heads_ups(v: bool) { update(|p| p.calendar_heads_ups = v) }
pub fn break_every() -> u32 { prefs().break_every }
pub fn set_break_every(v: u32) { update(|p| p.break_every = v) }
pub fn bedtime() -> Option<u32> { prefs().bedtime }
pub fn set_bedtime(v: Option<u32>) { update(|p| p.bedtime = v) }
pub fn alarms() -> Vec<Alarm> { prefs().alarms.clone() }
pub fn set_alarms(v: Vec<Alarm>) { update(|p| p.alarms = v) }
pub fn focus_in_calls() -> bool { prefs().focus_in_calls }
pub fn set_focus_in_calls(v: bool) { update(|p| p.focus_in_calls = v) }
pub fn dance_to_music() -> bool { prefs().dance_to_music }
pub fn set_dance_to_music(v: bool) { update(|p| p.dance_to_music = v) }
pub fn seasonal() -> bool { prefs().seasonal }
pub fn set_seasonal(v: bool) { update(|p| p.seasonal = v) }
pub fn perch_on_windows() -> bool { prefs().perch_on_windows }
pub fn set_perch_on_windows(v: bool) { update(|p| p.perch_on_windows = v) }
pub fn check_updates() -> bool { prefs().check_updates }
pub fn set_check_updates(v: bool) { update(|p| p.check_updates = v) }
pub fn calendar_url() -> Option<String> { prefs().calendar_url.clone() }
pub fn set_calendar_url(v: Option<String>) { update(|p| p.calendar_url = v) }
pub fn morning_agenda() -> bool { prefs().morning_agenda }
pub fn set_morning_agenda(v: bool) { update(|p| p.morning_agenda = v) }
pub fn focus_offers() -> bool { prefs().focus_offers }
pub fn set_focus_offers(v: bool) { update(|p| p.focus_offers = v) }
pub fn back_to_back() -> bool { prefs().back_to_back }
pub fn set_back_to_back(v: bool) { update(|p| p.back_to_back = v) }
pub fn wrap_up() -> Option<u32> { prefs().wrap_up }
pub fn set_wrap_up(v: Option<u32>) { update(|p| p.wrap_up = v) }
pub fn late_nudges() -> bool { prefs().late_nudges }
pub fn set_late_nudges(v: bool) { update(|p| p.late_nudges = v) }
pub fn weekly_summary() -> bool { prefs().weekly_summary }
pub fn set_weekly_summary(v: bool) { update(|p| p.weekly_summary = v) }
pub fn last_weekly() -> Option<String> { prefs().last_weekly.clone() }
pub fn set_last_weekly(v: String) { update(|p| p.last_weekly = Some(v)) }
pub fn todos() -> Vec<Todo> { prefs().todos.clone() }
pub fn set_todos(v: Vec<Todo>) { update(|p| p.todos = v) }
pub fn days() -> Vec<DayRecord> { prefs().days.clone() }
/// Saves one day's tally, replacing that day's earlier one and forgetting days older than two weeks.
pub fn save_day(day: DayRecord) {
    update(|p| {
        p.days.retain(|d| d.date != day.date);
        p.days.push(day);
        p.days.sort_by(|a, b| a.date.cmp(&b.date));
        let extra = p.days.len().saturating_sub(KEEP_DAYS);
        p.days.drain(..extra);
    })
}
pub fn terminal_cell() -> Option<(f64, f64)> { prefs().terminal_cell }
pub fn set_terminal_cell(v: Option<(f64, f64)>) { update(|p| p.terminal_cell = v) }

/// Carries settings over from the Swift build, which kept them in UserDefaults.
#[cfg(all(target_os = "macos", not(test)))]
fn migrate_from_user_defaults() -> Option<Prefs> {
    let file = dirs::home_dir()?.join("Library/Preferences/id.my.rihanna.pawpal.plist");
    let plist::Value::Dictionary(d) = plist::Value::from_file(file).ok()? else { return None };
    let mut p = Prefs::default();
    if let Some(v) = d.get("scale").and_then(|v| v.as_real()) { p.scale = v }
    if let Some(v) = d.get("auto").and_then(|v| v.as_boolean()) { p.auto = v }
    if let Some(v) = d.get("visitIdleCursor").and_then(|v| v.as_boolean()) { p.visit_idle_cursor = v }
    if d.get("walkArea").and_then(|v| v.as_string()) == Some("screen") { p.walk_area = WalkArea::Screen }
    if let Some(data) = d.get("pets").and_then(|v| v.as_data()) {
        p.pets = serde_json::from_slice(data).unwrap_or_default();
    }
    for (k, v) in d.iter() {
        if let (Some(name), Some(true)) = (k.strip_prefix("flip."), v.as_boolean()) {
            p.flipped.insert(name.to_string());
        }
    }
    Some(p)
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
