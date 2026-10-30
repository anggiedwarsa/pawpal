//! Unit tests for domain/activity.rs.

use super::*;

#[test]
fn every_activity_has_a_title() {
    assert_eq!(Walk.title(), "🚶 Walk");
    assert!(Activity::MENU.iter().all(|a| !a.title().is_empty()));
}

#[test]
fn the_menu_leaves_out_internal_states() {
    assert!(!Activity::MENU.contains(&Dragged) && !Activity::MENU.contains(&Falling));
}

#[test]
fn quiet_activities_are_calm() {
    assert!(Sleep.is_calm() && Sit.is_calm());
    assert!(!Run.is_calm() && !PlayBall.is_calm());
}

#[test]
fn special_powers_are_flagged() {
    assert!([Transform, BreatheFire, Fly, Grow, Herd, Vanish, Stomp].iter().all(|a| a.is_special()));
    assert!(!Walk.is_special() && !PlayFight.is_special());
}
