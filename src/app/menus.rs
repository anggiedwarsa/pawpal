//! The 🐾 tray menu and the right-click menu on each pet, described as plain data
//! so the tray is only rebuilt when something in it actually changed.

use std::collections::HashMap;

use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

use super::App;
use crate::data::settings::{self, WalkArea};
use crate::data::skin::ACTIONS;
use crate::domain::Activity::{self, *};
use crate::domain::Pet;
use crate::engine::alarms::{clock_label, Alarm};
use crate::engine::breaks::BREAK_EVERY;
use crate::engine::focus::FOCUS_FOR;
use crate::engine::rhythm::BEDTIMES;
use crate::engine::tricks::Trick;
use crate::platform;

/// What a menu item does when chosen.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Do { pets: Vec<u64>, activity: Activity },
    ComeHome(u64),
    Trick { pet: u64, trick: Trick },
    FeedPet(u64),
    FeedAll,
    Treat,
    ThrowBall,
    ReleaseButterfly,
    ToggleLaser,
    HidePets(Option<u32>),
    ShowPets,
    RemindMe,
    Focus(u32),
    EndFocus,
    ToggleCallFocus,
    ToggleMusic,
    ToggleSeasonal,
    TogglePerch,
    CancelAlarm(u64),
    Rename(u64),
    Flip(String),
    Import { skin: Option<String>, action: &'static str },
    Remove(u64),
    Share(String),
    ImportShared,
    Adopt(String),
    OpenClips,
    Scale(f64),
    Walk(WalkArea),
    AllowAccessibility,
    ToggleVisit,
    ToggleAvoidCursor,
    ToggleAuto,
    ToggleCalendar,
    BreakEvery(u32),
    Bedtime(Option<u32>),
    ToggleLogin,
    ToggleUpdates,
    OpenUpdate(String),
    About,
    Quit,
}

#[derive(Debug, PartialEq)]
pub enum Node {
    Item { title: String, cmd: Option<Command>, checked: Option<bool> },
    Sub { title: String, children: Vec<Node> },
    Sep,
}

fn item(title: impl Into<String>, cmd: Command) -> Node { Node::Item { title: title.into(), cmd: Some(cmd), checked: None } }
fn check(title: impl Into<String>, on: bool, cmd: Command) -> Node { Node::Item { title: title.into(), cmd: Some(cmd), checked: Some(on) } }
fn note(title: impl Into<String>) -> Node { Node::Item { title: title.into(), cmd: None, checked: None } }
fn sub(title: impl Into<String>, children: Vec<Node>) -> Node { Node::Sub { title: title.into(), children } }

/// Everything set once and left: how pets look and behave, then reminders and focus, then launching.
fn settings_items(login_enabled: Option<bool>) -> Vec<Node> {
    let mut m = Vec::new();
    let scale = settings::scale();
    m.push(sub(
        "Size",
        [("Small", 0.8), ("Medium", 1.1), ("Large", 1.5), ("Huge", 2.0)]
            .into_iter()
            .map(|(name, v)| check(name, (scale - v).abs() < 0.01, Command::Scale(v)))
            .collect(),
    ));
    let walk = settings::walk_area();
    let shelf = if cfg!(target_os = "macos") { "On the Dock" } else { "On the Taskbar" };
    m.push(sub(
        "Walk Area",
        vec![
            check(shelf, walk == WalkArea::Dock, Command::Walk(WalkArea::Dock)),
            check("Whole Screen Width", walk == WalkArea::Screen, Command::Walk(WalkArea::Screen)),
        ],
    ));
    m.push(Node::Sep);
    m.push(check("Free Will (auto activities)", settings::auto(), Command::ToggleAuto));
    m.push(check("Visit Cursor After 1 Min Idle", settings::visit_idle_cursor(), Command::ToggleVisit));
    m.push(check("🙈 Avoid My Cursor", settings::avoid_cursor(), Command::ToggleAvoidCursor));
    m.push(check("🪟 Perch on Windows", settings::perch_on_windows(), Command::TogglePerch));
    let bed = settings::bedtime();
    let mut beds = vec![check("Off (no daily rhythm)", bed.is_none(), Command::Bedtime(None))];
    beds.extend(BEDTIMES.into_iter().map(|h| check(clock(h), bed == Some(h), Command::Bedtime(Some(h)))));
    m.push(sub("🌙 Bedtime", beds));
    m.push(check("🎵 Dance to Music", settings::dance_to_music(), Command::ToggleMusic));
    m.push(check("✨ Seasonal Touches", settings::seasonal(), Command::ToggleSeasonal));
    m.push(Node::Sep);
    if platform::has_calendar() {
        m.push(check("📅 Calendar Heads-Ups (10, 5, 1 min before)", settings::calendar_heads_ups(), Command::ToggleCalendar));
    }
    let every = settings::break_every();
    let mut breaks = vec![check("Off", every == 0, Command::BreakEvery(0))];
    breaks.extend(BREAK_EVERY.into_iter().map(|n| check(format!("Every {n} Minutes"), every == n, Command::BreakEvery(n))));
    m.push(sub("🧘 Break Reminders", breaks));
    m.push(check("📞 Focus During Calls", settings::focus_in_calls(), Command::ToggleCallFocus));
    m.push(Node::Sep);
    if let Some(on) = login_enabled { m.push(check("Launch at Login", on, Command::ToggleLogin)) }
    m.push(check("Check for Updates", settings::check_updates(), Command::ToggleUpdates));
    m
}

/// Putting the pets away for a while, or bringing them back; `shortcut` is the global key, if it could be set up.
fn hide_items(hidden: bool, shortcut: Option<&str>) -> Node {
    let key = shortcut.map_or(String::new(), |k| format!("  ({k})"));
    if hidden { return item(format!("🐾 Bring Pets Back{key}"), Command::ShowPets) }
    sub(
        format!("🙈 Hide Pets{key}"),
        vec![
            item("For 15 Minutes", Command::HidePets(Some(15))),
            item("For 1 Hour", Command::HidePets(Some(60))),
            item("Until I Bring Them Back", Command::HidePets(None)),
        ],
    )
}

/// Set reminders, soonest first; choosing one cancels it.
fn alarm_items(mut alarms: Vec<Alarm>, now: f64, hour: f64) -> Vec<Node> {
    alarms.sort_by(|a, b| a.at.total_cmp(&b.at));
    let mut m = vec![note("Choose one to cancel it")];
    m.extend(alarms.iter().map(|a| item(format!("✖ {} — {}", clock_label(a.at, now, hour), a.text.trim()), Command::CancelAlarm(a.id))));
    m
}

/// An hour on the clock, as a menu shows it.
fn clock(h: u32) -> String {
    match h {
        0 => "Midnight".into(),
        12 => "Noon".into(),
        h if h > 12 => format!("{} PM", h - 12),
        h => format!("{h} AM"),
    }
}

/// Activities, with ones the pet has no clip for greyed out and saying which clip to import.
fn activity_items(pets: &[&Pet]) -> Vec<Node> {
    Activity::MENU
        .iter()
        .map(|&a| {
            let able: Vec<u64> = pets.iter().filter(|p| p.supports(a)).map(|p| p.id).collect();
            if able.is_empty() {
                let missing = pets.first().and_then(|p| p.skin.as_ref()).map_or("walk", |s| s.missing_clip(a));
                return note(format!("{}  — needs a “{missing}” clip", a.title()));
            }
            item(a.title(), Command::Do { pets: able, activity: a })
        })
        .collect()
}

fn bar(v: f64) -> String {
    let n = ((v / 10.0).round() as usize).min(10);
    "■".repeat(n) + &"□".repeat(10 - n)
}

fn stats_items(p: &Pet) -> Vec<Node> {
    let mood = if p.happiness > 75.0 { "😊 Happy" } else if p.happiness > 40.0 { "🙂 Okay" } else { "😢 Sad" };
    let moves = p.skin.as_ref().map_or("none".to_string(), |s| s.clips.keys().cloned().collect::<Vec<_>>().join(", "));
    vec![
        note(format!("{} · {mood}", p.name)),
        note(format!("Food   {}", bar(100.0 - p.hunger))),
        note(format!("Energy {}", bar(p.energy))),
        note(format!("Joy    {}", bar(p.happiness))),
        note(format!("Moves: {moves}")),
    ]
}

fn import_items(skin: Option<&str>) -> Vec<Node> {
    ACTIONS.iter().map(|(name, title, _)| item(format!("{title}…"), Command::Import { skin: skin.map(String::from), action: name })).collect()
}

/// Tricks, with ones the pet has no clip for greyed out and saying which clip to import.
fn trick_items(p: &Pet) -> Vec<Node> {
    Trick::ALL
        .iter()
        .map(|&t| {
            if p.supports(t.activity()) { return item(t.title(), Command::Trick { pet: p.id, trick: t }) }
            let missing = p.skin.as_ref().map_or("walk", |s| s.missing_clip(t.activity()));
            note(format!("{}  — needs a “{missing}” clip", t.title()))
        })
        .collect()
}

pub fn pet_menu(p: &Pet) -> Vec<Node> {
    let mut m = stats_items(p);
    m.push(Node::Sep);
    m.push(sub("Do Activity", activity_items(&[p])));
    m.push(sub("🎾 Tricks", trick_items(p)));
    if p.activity == VisitCursor || p.floor > 0.0 { m.push(item("🏠 Come Home", Command::ComeHome(p.id))) }
    m.push(item("🍗 Feed", Command::FeedPet(p.id)));
    m.push(item("✏️ Rename…", Command::Rename(p.id)));
    if let Some(skin) = &p.skin {
        if let Some(credit) = &skin.credit { m.push(note(credit.trim())) }
        m.push(item("↔️ Flip Direction", Command::Flip(skin.name.clone())));
        m.push(sub(format!("➕ Add a Clip for {}", skin.name), import_items(Some(&skin.name))));
        m.push(item(format!("📤 Share {}…", skin.name), Command::Share(skin.name.clone())));
    }
    m.push(item("👋 Say Goodbye (Remove)", Command::Remove(p.id)));
    m
}

impl App {
    pub fn tray_menu(&self) -> Vec<Node> {
        let mut m = Vec::new();
        let pets: Vec<&Pet> = self.world.pets.iter().collect();
        if pets.is_empty() {
            m.push(note("No pets yet — import a clip of a real cat to start"));
        } else {
            m.push(sub("All Pets: Do Activity", activity_items(&pets)));
            m.push(item("🍗 Feed Everyone", Command::FeedAll));
            m.push(item("🐟 Give Treats", Command::Treat));
            m.push(item("⚽ Throw a Ball", Command::ThrowBall));
            m.push(item("🦋 Release a Butterfly", Command::ReleaseButterfly));
            if !self.world.tucked { m.push(check("🔴 Laser Pointer", self.world.laser, Command::ToggleLaser)) }
            m.push(hide_items(self.world.tucked, self.hotkey_label()));
            m.push(Node::Sep);
            m.push(item("⏰ Remind Me…", Command::RemindMe));
            match self.focus_until {
                Some(until) => {
                    let left = ((until - super::now_secs()) / 60.0).ceil().max(1.0);
                    m.push(item(format!("🎯 End Focus ({left} min left)"), Command::EndFocus));
                }
                None if self.call_focus => m.push(item("🎯 End Focus (in a call)", Command::EndFocus)),
                None => m.push(sub("🎯 Focus", FOCUS_FOR.into_iter().map(|n| item(format!("{n} Minutes"), Command::Focus(n))).collect())),
            }
            let alarms = settings::alarms();
            if !alarms.is_empty() {
                m.push(sub(format!("⏰ Upcoming Reminders ({})", alarms.len()), alarm_items(alarms, super::now_secs(), platform::local_hour())));
            }
            m.push(Node::Sep);
            for p in &pets {
                m.push(sub(format!("🐈 {}", p.name), pet_menu(p)));
            }
        }

        let mut adopt: Vec<Node> = self
            .skins
            .iter()
            .map(|(name, s)| item(format!("{name}  ({})", s.clips.keys().cloned().collect::<Vec<_>>().join(", ")), Command::Adopt(name.clone())))
            .collect();
        if !adopt.is_empty() { adopt.push(Node::Sep) }
        adopt.push(sub("Import Clip (video, GIF or WebP)", import_items(None)));
        adopt.push(item("Import a Shared Pet (.pawpal)…", Command::ImportShared));
        adopt.push(item("Open Clips Folder", Command::OpenClips));
        adopt.push(note("Tip: same cat, side view, plain background, 2–6 s"));
        m.push(sub("➕ Adopt / Import a Pet", adopt));
        m.push(Node::Sep);

        if platform::has_accessibility_setting() && !platform::accessibility_trusted() {
            m.push(item("Fit Dock & Avoid My Typing (allow Accessibility)…", Command::AllowAccessibility));
        }
        m.push(sub("⚙️ Settings", settings_items(self.login_enabled)));
        m.push(Node::Sep);
        if let Some(r) = &self.update { m.push(item(format!("⬆️ Update to PawPal {}…", r.version), Command::OpenUpdate(r.url.clone()))) }
        m.push(item("About PawPal…", Command::About));
        m.push(item("Quit PawPal", Command::Quit));
        m
    }
}

/// Something menu items can be added to.
trait Container {
    fn add(&self, item: &dyn IsMenuItem);
}
impl Container for Menu {
    fn add(&self, item: &dyn IsMenuItem) { let _ = self.append(item); }
}
impl Container for Submenu {
    fn add(&self, item: &dyn IsMenuItem) { let _ = self.append(item); }
}

/// Builds real menu items, recording which command each id stands for. `prefix` keeps ids unique across menus.
pub fn materialize(nodes: &[Node], prefix: &str) -> (Menu, HashMap<MenuId, Command>) {
    let menu = Menu::new();
    let mut cmds = HashMap::new();
    let mut n = 0;
    fill(&menu, nodes, prefix, &mut n, &mut cmds);
    (menu, cmds)
}

fn fill(parent: &dyn Container, nodes: &[Node], prefix: &str, n: &mut u32, cmds: &mut HashMap<MenuId, Command>) {
    for node in nodes {
        match node {
            Node::Sep => parent.add(&PredefinedMenuItem::separator()),
            Node::Sub { title, children } => {
                let s = Submenu::new(title, true);
                fill(&s, children, prefix, n, cmds);
                parent.add(&s);
            }
            Node::Item { title, cmd, checked } => {
                *n += 1;
                let id = MenuId::new(format!("{prefix}{n}"));
                if let Some(c) = cmd { cmds.insert(id.clone(), c.clone()); }
                match checked {
                    Some(on) => parent.add(&CheckMenuItem::with_id(id, title, cmd.is_some(), *on, None)),
                    None => parent.add(&MenuItem::with_id(id, title, cmd.is_some(), None)),
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "menus_tests.rs"]
mod tests;
