//! Unit tests for engine/battery.rs.

use super::*;

#[test]
fn heads_ups_come_at_ten_and_five_percent_once_each() {
    let mut b = BatteryWatch::default();
    assert_eq!(b.due(40.0, false), None);
    assert_eq!(b.due(10.0, false), Some(10));
    assert_eq!(b.due(9.0, false), None, "once");
    assert_eq!(b.due(5.0, false), Some(5));
    assert_eq!(b.due(3.0, false), None);
}

#[test]
fn starting_low_gives_one_heads_up() {
    let mut b = BatteryWatch::default();
    assert_eq!(b.due(4.0, false), Some(5));
    assert_eq!(b.due(4.0, false), None, "the 10% one is skipped");
}

#[test]
fn plugging_in_quiets_it_and_resets_it() {
    let mut b = BatteryWatch::default();
    assert_eq!(b.due(8.0, true), None, "charging");
    assert_eq!(b.due(8.0, false), Some(10), "unplugged again");
    b.due(30.0, true);
    assert_eq!(b.due(10.0, false), Some(10), "a fresh charge, fresh heads-ups");
}

#[test]
fn a_little_wobble_does_not_repeat_it() {
    let mut b = BatteryWatch::default();
    b.due(10.0, false);
    assert_eq!(b.due(12.0, false), None);
    assert_eq!(b.due(10.0, false), None);
}

#[test]
fn the_pet_asks_for_the_charger() {
    assert_eq!(battery_text(10), "🔋 Battery's at 10%… plug me in?");
    assert!(battery_text(5).contains("5%"));
}
