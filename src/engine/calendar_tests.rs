//! Unit tests for engine/calendar.rs.

use super::*;
use crate::testing::world;

fn event(id: &str, start: f64) -> CalendarEvent { CalendarEvent { id: id.into(), title: "Standup".into(), start } }

fn run(w: &mut World, seconds: f64) {
    for _ in 0..(seconds * 30.0).round() as usize { w.with_pet(0, |w, p| w.update(p, 1.0 / 30.0)) }
}

#[test]
fn heads_ups_come_at_ten_five_and_one_minute() {
    let mut r = Reminders::default();
    let e = [event("a", 1000.0)];
    assert_eq!(r.due(&e, 1000.0 - 11.0 * 60.0), None, "too early");
    assert_eq!(r.due(&e, 1000.0 - 600.0).map(|d| d.1), Some(10));
    assert_eq!(r.due(&e, 1000.0 - 590.0), None, "each heads-up comes once");
    assert_eq!(r.due(&e, 1000.0 - 299.0).map(|d| d.1), Some(5));
    assert_eq!(r.due(&e, 1000.0 - 59.0).map(|d| d.1), Some(1));
    assert_eq!(r.due(&e, 1000.0 - 30.0), None);
    assert_eq!(r.due(&e, 1001.0), None, "started already");
}

#[test]
fn starting_late_gives_one_heads_up_not_three() {
    let mut r = Reminders::default();
    let e = [event("a", 1000.0)];
    assert_eq!(r.due(&e, 1000.0 - 180.0).map(|d| d.1), Some(3));
    assert_eq!(r.due(&e, 1000.0 - 170.0), None, "the 10 and 5 minute ones are skipped");
    assert_eq!(r.due(&e, 1000.0 - 50.0).map(|d| d.1), Some(1));
}

#[test]
fn each_event_gets_its_own_heads_ups() {
    let mut r = Reminders::default();
    let e = [event("a", 1000.0), event("b", 1000.0)];
    assert_eq!(r.due(&e, 500.0).map(|d| d.0.id), Some("a".into()));
    assert_eq!(r.due(&e, 500.0).map(|d| d.0.id), Some("b".into()));
}

#[test]
fn the_bubble_says_what_and_when() {
    assert_eq!(heads_up_text("Standup", 5), "📅 Standup in 5 minutes");
    assert_eq!(heads_up_text("Standup", 1), "📅 Standup starts in 1 minute!");
    assert_eq!(heads_up_text("  ", 10), "📅 Your event in 10 minutes");
    assert!(heads_up_text(&"x".repeat(80), 5).contains("…"));
}

#[test]
fn the_pet_goes_to_the_middle_of_the_screen_to_tell() {
    let mut w = world(1);
    w.remind("📅 Standup in 5 minutes");
    assert_eq!(w.pets[0].activity, Remind);
    run(&mut w, 12.0);
    let p = &w.pets[0];
    assert_eq!(p.step, TELL);
    assert!((p.x - 500.0).abs() < 1.0 && p.y > 150.0, "in the middle of the screen: {}, {}", p.x, p.y);
    assert_eq!(p.bubble.as_deref(), Some("📅 Standup in 5 minutes"));
    let y = p.y;
    run(&mut w, 5.0);
    assert_eq!(w.pets[0].y, y, "it stays up there while it tells");
}

#[test]
fn after_telling_it_goes_back_to_the_same_spot_level_and_activity() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        (p.x, p.y) = (300.0, 200.0);
        w.settle(p);
        w.start(p, Lounge, false);
        p.timer = 100.0;
    });
    let span = w.pets[0].span;
    w.remind("📅 Standup in 10 minutes");
    run(&mut w, 12.0 + REMIND_TIME + 3.0);
    let p = &w.pets[0];
    assert_eq!(p.activity, Lounge, "it carries on with what it was doing");
    assert_eq!((p.x, p.y, p.floor, p.span), (300.0, 200.0, 200.0, span), "at the same spot and height");
    assert!(p.timer > 50.0, "with the time it had left");
    assert!(p.resume.is_none());
}

#[test]
fn clicking_the_pet_sends_it_back_straight_away() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.start(p, Sit, false));
    w.remind("📅 Standup in 1 minute!");
    run(&mut w, 12.0);
    w.with_pet(0, |w, p| w.clicked(p, 1));
    assert_eq!(w.pets[0].bubble, None);
    run(&mut w, 2.0);
    let p = &w.pets[0];
    assert_eq!((p.activity, p.y, p.floor), (Sit, 0.0, 0.0));
    assert_eq!(p.x, 200.0);
}

#[test]
fn a_sleeping_pet_goes_back_to_bed() {
    let mut w = world(1);
    w.pets[0].energy = 5.0;
    w.with_pet(0, |w, p| w.start(p, Sleep, false));
    run(&mut w, 20.0);
    w.remind("📅 Standup in 5 minutes");
    run(&mut w, 15.0);
    w.with_pet(0, |w, p| w.clicked(p, 1));
    run(&mut w, 15.0);
    assert_eq!(w.pets[0].activity, Sleep);
}

#[test]
fn a_second_heads_up_reuses_the_pet_already_out() {
    let mut w = world(2);
    w.remind("first");
    w.remind("second");
    assert_eq!(w.pets.iter().filter(|p| p.activity == Remind).count(), 1);
    assert!(w.pets.iter().any(|p| p.bubble.as_deref() == Some("second")));
}
