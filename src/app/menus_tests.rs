//! Unit tests for app/menus.rs.

use super::*;
use crate::domain::{Coat, Species};
use crate::testing::{pet_file, skin, skin_with};

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
    let items = import_items(Some(&skin(&["walk"])));
    assert_eq!(items.len(), ACTIONS.len());
    assert!(items.contains(&item("Walk…", Command::Import { skin: Some("Test".into()), action: "walk".into() })));
    assert_eq!(import_items(None).len(), ACTIONS.len());
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
    for s in ["Size", "Walk Area", "Free Will (auto activities)", "🙈 Avoid My Cursor", "🌙 Bedtime", "🎵 Dance to Music", "✨ Seasonal Touches", "🧘 Break Reminders", "🌇 Wrap-Up Reminder", "📊 Weekly Summary on Fridays", "📅 Calendar", "📞 Focus During Calls", "Launch at Login", "Check for Updates"] {
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
    for s in ["Heads-Ups (10, 5, 1 min before)", "☀️ Morning Agenda", "🎯 Offer Focus in Free Time", "💧 Back-to-Back Warnings", "⏰ Late & Over-Time Nudges"] {
        assert!(t.contains(&s.to_string()), "{s}");
    }
}

#[test]
fn todos_are_listed_and_tick_off_when_chosen() {
    let todos = [Todo { id: 7, text: "Send invoice".into() }, Todo { id: 8, text: "Book flights".into() }];
    assert_eq!(
        todo_items(&todos),
        vec![
            note("Choose one to tick it off"),
            check("Send invoice", false, Command::DoneTodo(7)),
            check("Book flights", false, Command::DoneTodo(8)),
        ]
    );
}

#[test]
fn special_powers_show_up_only_for_the_pets_that_have_them() {
    let fox = Pet::new(1, "Kyubi".into(), Species::Fox, Some(skin(&["walk", "transform"])));
    let dragon = Pet::new(2, "Naga".into(), Species::Dragon, Some(skin(&["walk", "firebreath"])));
    let cat = Pet::new(3, "Mochi".into(), Species::Cat, Some(skin(&["walk"])));
    let has = |p: &Pet, t: &str| titles(&activity_items(&[p])).iter().any(|x| x == t);
    assert!(has(&fox, "🦊 Transform") && !has(&fox, "🔥 Breathe Fire"));
    assert!(has(&dragon, "🔥 Breathe Fire") && !has(&dragon, "🦊 Transform"));
    assert!(titles(&activity_items(&[&cat])).iter().all(|x| !x.contains("Transform") && !x.contains("Breathe Fire")),
            "no greyed-out hint for powers a cat will never have");
}

#[test]
fn only_dragons_get_a_colour_menu_with_their_coat_ticked() {
    let colours = |p: &Pet| {
        pet_menu(p).into_iter().find_map(|n| match n {
            Node::Sub { title, children } if title == "🎨 Color" => Some(children),
            _ => None,
        })
    };
    let mut dragon = Pet::new(1, "Naga".into(), Species::Dragon, Some(skin(&["walk", "fly"])));
    dragon.coat = Coat::Pink;
    let items = colours(&dragon).expect("a dragon can change colour");
    assert_eq!(items.len(), Coat::ALL.len());
    let ticked: Vec<String> = items.iter().filter_map(|n| match n {
        Node::Item { title, checked: Some(true), .. } => Some(title.clone()),
        _ => None,
    }).collect();
    assert_eq!(ticked, vec![Coat::Pink.title().to_string()]);
    let cat = Pet::new(2, "Mochi".into(), Species::Cat, Some(skin(&["walk"])));
    assert!(colours(&cat).is_none());
}

const BUBBLES: &str = r#"{ "kind": "Axolotl", "skills": [
    { "name": "🫧 Blow Bubbles", "clip": "bubbles" },
    { "name": "🌀 Spin", "clip": "spin" },
    { "name": "🎉 Party", "clip": "meow" }
] }"#;

#[test]
fn a_pets_own_skills_end_its_activity_menu() {
    let p = Pet::new(4, "Bloop".into(), Species::Cat, Some(skin_with(&["walk", "bubbles"], pet_file(BUBBLES))));
    let items = activity_items(&[&p]);
    assert!(items.contains(&item("🫧 Blow Bubbles", Command::Do { pets: vec![4], activity: Activity::Skill(0) })));
    assert!(titles(&items).contains(&"🌀 Spin  — needs a “spin” clip".to_string()));
    let other = Pet::new(5, "Mochi".into(), Species::Cat, Some(skin(&["walk"])));
    assert!(titles(&activity_items(&[&p, &other])).iter().all(|t| !t.contains("Bubbles")), "skills aren't shared across pets");
    assert!(!activity_items(&[&other]).contains(&Node::Sep), "no separator without skills of its own");
}

#[test]
fn a_pets_own_skill_clips_can_be_imported() {
    let k = skin_with(&["walk"], pet_file(BUBBLES));
    let items = import_items(Some(&k));
    assert_eq!(items.len(), ACTIONS.len() + 3, "a separator, then bubbles and spin; meow is already there");
    assert!(items.contains(&item("🫧 Blow Bubbles…", Command::Import { skin: Some("Test".into()), action: "bubbles".into() })));
}

#[test]
fn the_kind_of_animal_shows_in_its_menu() {
    let p = Pet::new(4, "Bloop".into(), Species::Cat, Some(skin_with(&["walk"], pet_file(BUBBLES))));
    assert!(titles(&pet_menu(&p)).contains(&"🐾 Axolotl".to_string()));
}

#[test]
fn the_reminder_pose_lists_still_clips_and_ticks_the_chosen_one() {
    let pose = |clip: Option<&str>| Command::RemindPose { skin: "Test".into(), clip: clip.map(String::from) };
    let k = skin_with(&["walk", "run", "jump", "sit", "meow", "bubbles"], pet_file(BUBBLES));
    let items = remind_items(&k);
    assert_eq!(items, vec![check("Sit (usual)", true, pose(None)), check("🫧 Blow Bubbles", false, pose(Some("bubbles"))), check("Meow", false, pose(Some("meow")))]);
    let k = skin_with(&["sit", "meow"], pet_file(r#"{ "remind": "meow" }"#));
    assert_eq!(remind_items(&k), vec![check("Sit (usual)", false, pose(None)), check("Meow", true, pose(Some("meow")))]);
    let k = skin_with(&["sit"], pet_file(r#"{ "remind": "gone" }"#));
    assert_eq!(remind_items(&k)[0], check("Sit (usual)", true, pose(None)), "a missing clip counts as the usual");
}

#[test]
fn any_pet_whose_file_says_so_can_change_colour() {
    let has_colours = |p: &Pet| titles(&pet_menu(p)).contains(&"🎨 Color".to_string());
    let fox = Pet::new(1, "Teal".into(), Species::Fox, Some(skin_with(&["walk"], pet_file(r#"{ "recolor": { "hue": 180 } }"#))));
    assert!(has_colours(&fox));
    assert!(!has_colours(&Pet::new(2, "Plain".into(), Species::Fox, Some(skin(&["walk"])))));
}
