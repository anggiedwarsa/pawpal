//! Unit tests for app/menus.rs.

use super::*;
use crate::domain::Species;
use crate::testing::skin;

fn titles(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Item { title, .. } | Node::Sub { title, .. } => Some(title.clone()),
            Node::Sep => None,
        })
        .collect()
}

#[test]
fn bars_show_ten_steps() {
    assert_eq!(bar(100.0), "■■■■■■■■■■");
    assert_eq!(bar(0.0), "□□□□□□□□□□");
    assert_eq!(bar(44.0), "■■■■□□□□□□");
}

#[test]
fn pet_menu_shows_stats_and_actions() {
    let mut p = Pet::new(7, "Mochi".into(), Species::Cat, Some(skin(&["sit", "walk"])));
    p.happiness = 90.0;
    let t = titles(&pet_menu(&p));
    assert_eq!(t[0], "Mochi · 😊 Happy");
    assert!(t.contains(&"Moves: sit, walk".to_string()));
    assert!(t.contains(&"🍗 Feed".to_string()) && !t.contains(&"🏠 Come Home".to_string()));
    p.floor = 100.0;
    assert!(titles(&pet_menu(&p)).contains(&"🏠 Come Home".to_string()), "a pet up the screen can be called home");
}

#[test]
fn missing_clips_are_greyed_out_with_a_hint() {
    let p = Pet::new(1, "A".into(), Species::Cat, Some(skin(&["sit"])));
    let items = activity_items(&[&p]);
    let walk = items.iter().find_map(|n| match n {
        Node::Item { title, cmd, .. } if title.starts_with("🚶") => Some((title.clone(), cmd.clone())),
        _ => None,
    });
    assert_eq!(walk, Some(("🚶 Walk  — needs a “walk” clip".to_string(), None)));
    assert!(items.contains(&item("🪑 Sit", Command::Do { pets: vec![1], activity: Sit })));
}

#[test]
fn every_action_can_be_imported() {
    let items = import_items(Some("Mochi"));
    assert_eq!(items.len(), ACTIONS.len());
    assert!(items.contains(&item("Walk…", Command::Import { skin: Some("Mochi".into()), action: "walk" })));
}

#[test]
fn tricks_are_in_the_pet_menu_and_greyed_out_without_a_clip() {
    let p = Pet::new(3, "A".into(), Species::Cat, Some(skin(&["sit"])));
    assert!(titles(&pet_menu(&p)).contains(&"🎾 Tricks".to_string()));
    let items = trick_items(&p);
    assert_eq!(items.len(), Trick::ALL.len());
    assert!(items.contains(&item("🪑 Sit", Command::Trick { pet: 3, trick: Trick::Sit })));
    assert!(items.contains(&note("💃 Dance  — needs a “play” clip")));
}

#[test]
fn upcoming_reminders_are_listed_soonest_first_and_cancel_when_chosen() {
    let a = |id, text: &str, at| Alarm { id, text: text.into(), at };
    let items = alarm_items(vec![a(2, "Laundry", 7200.0), a(1, " Call mum ", 3600.0)], 0.0, 14.0);
    assert_eq!(
        items,
        vec![
            note("Choose one to cancel it"),
            item("✖ 3:00 PM — Call mum", Command::CancelAlarm(1)),
            item("✖ 4:00 PM — Laundry", Command::CancelAlarm(2)),
        ]
    );
}

#[test]
fn settings_are_all_in_one_submenu() {
    let t = titles(&settings_items(Some(true)));
    for s in ["Size", "Walk Area", "Free Will (auto activities)", "🙈 Avoid My Cursor", "🌙 Bedtime", "🎵 Dance to Music", "✨ Seasonal Touches", "🧘 Break Reminders", "🌇 Wrap-Up Reminder", "📅 Calendar", "📞 Focus During Calls", "Launch at Login", "Check for Updates"] {
        assert!(t.contains(&s.to_string()), "{s} is in Settings");
    }
    assert!(!titles(&settings_items(None)).contains(&"Launch at Login".to_string()), "only where it can be set");
}

#[test]
fn pets_can_be_hidden_for_a_while_and_brought_back() {
    let Node::Sub { title, children } = hide_items(false, Some("⌃⌥P")) else { panic!("a submenu of how long") };
    assert_eq!(title, "🙈 Hide Pets  (⌃⌥P)");
    assert_eq!(
        children,
        vec![
            item("For 15 Minutes", Command::HidePets(Some(15))),
            item("For 1 Hour", Command::HidePets(Some(60))),
            item("Until I Bring Them Back", Command::HidePets(None)),
        ]
    );
    assert_eq!(hide_items(true, None), item("🐾 Bring Pets Back", Command::ShowPets), "no shortcut, no hint");
}

#[test]
fn a_pet_with_clips_can_be_shared() {
    let p = Pet::new(1, "Mochi".into(), Species::Cat, Some(skin(&["sit"])));
    assert!(pet_menu(&p).contains(&item("📤 Share Test…", Command::Share("Test".into()))));
    let bare = Pet::new(2, "Ghost".into(), Species::Cat, None);
    assert!(!titles(&pet_menu(&bare)).iter().any(|t| t.starts_with("📤")));
}

#[test]
fn the_calendar_menu_asks_for_a_link_where_there_is_no_calendar() {
    let t = titles(&calendar_items(false, false));
    assert!(t.contains(&"🔗 Add Calendar Link…".to_string()));
    assert!(t.contains(&"Add your calendar's private iCal link to turn these on".to_string()));
    let t = titles(&calendar_items(true, true));
    assert!(t.contains(&"🔗 Change Calendar Link…".to_string()) && !t.iter().any(|s| s.starts_with("Add your")));
    for s in ["Heads-Ups (10, 5, 1 min before)", "☀️ Morning Agenda", "🎯 Offer Focus in Free Time", "💧 Back-to-Back Warnings"] {
        assert!(t.contains(&s.to_string()), "{s}");
    }
}
