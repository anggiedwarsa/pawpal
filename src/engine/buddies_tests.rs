//! Unit tests for engine/buddies.rs.

use super::*;
use crate::testing::world;

/// Tries free will until it picks something with a friend.
fn buddy_up_eventually(w: &mut World, i: usize) -> bool { (0..200).any(|_| w.with_pet(i, |w, p| w.buddy_up(p))) }

#[test]
fn a_tired_pet_joins_a_friend_in_bed() {
    let mut w = world(2);
    w.with_pet(1, |w, p| w.start(p, Sleep, false));
    w.pets[0].energy = 40.0;
    assert!(buddy_up_eventually(&mut w, 0));
    assert_eq!(w.pets[0].activity, Sleep);
    assert!(w.pets[0].target_x != w.pets[1].target_x, "it takes the bed beside its friend's");
}

#[test]
fn a_rested_pet_lets_a_friend_nap_alone() {
    let mut w = world(2);
    w.with_pet(1, |w, p| w.start(p, Sleep, false));
    w.pets[0].energy = 95.0;
    w.pets[0].skin = Some(crate::testing::skin(&["sleep", "sit"])); // no walking, so nothing else to do together
    assert!(!buddy_up_eventually(&mut w, 0));
}

#[test]
fn grooming_walks_over_then_both_feel_loved() {
    let mut w = world(2);
    w.with_pet(1, |w, p| w.start(p, Sit, false));
    assert!(buddy_up_eventually(&mut w, 0));
    assert_eq!((w.pets[0].activity, w.pets[0].buddy), (Groom, Some(2)));
    w.with_pet(0, |w, p| w.groom_buddy(p, 0.03));
    assert!(w.pets[0].vx > 0.0, "it walks over to its friend first");
    w.pets[0].x = w.pets[1].x - w.pets[1].size.width / 2.0 - w.pets[0].size.width * 0.3;
    w.with_pet(0, |w, p| w.groom_buddy(p, 0.03));
    let joy = w.pets[1].happiness;
    w.pets[1].timer = 0.1;
    w.with_pet(0, |w, p| w.groom_buddy(p, 0.5));
    assert_eq!((w.pets[0].step, w.pets[0].vx, w.pets[0].facing), (1, 0.0, 1.0));
    assert!(w.pets[1].happiness > joy && w.pets[1].timer >= 1.0, "the friend enjoys it and holds still");
}

#[test]
fn grooming_stops_when_the_friend_wanders_off() {
    let mut w = world(2);
    w.with_pet(0, |w, p| {
        w.start(p, Groom, false);
        p.buddy = Some(2);
    });
    w.with_pet(1, |w, p| w.start(p, Run, false));
    w.with_pet(0, |w, p| w.groom_buddy(p, 0.03));
    assert_eq!(w.pets[0].timer, 0.0);
}

#[test]
fn a_pet_tags_along_behind_a_walking_friend() {
    let mut w = world(2);
    w.with_pet(1, |w, p| {
        w.start(p, Walk, false);
        p.facing = 1.0;
        p.vx = 45.0;
    });
    w.pets[0].skin = Some(crate::testing::skin(&["walk", "sit"])); // no groom clip, so it follows
    assert!(buddy_up_eventually(&mut w, 0));
    assert_eq!((w.pets[0].activity, w.pets[0].buddy), (Walk, Some(2)));
    w.with_pet(0, |w, p| w.follow_buddy(p));
    assert!(w.pets[0].vx >= 65.0, "it hurries to catch up behind its friend");
    w.with_pet(1, |w, p| w.start(p, Sit, false));
    w.with_pet(0, |w, p| w.follow_buddy(p));
    assert_eq!(w.pets[0].timer, 0.0, "the walk together ends when the friend stops");
}

#[test]
fn friends_must_be_on_the_same_level_and_nearby() {
    let mut w = world(2);
    w.with_pet(1, |w, p| w.start(p, Sit, false));
    w.pets[1].floor = 200.0;
    assert_eq!(w.friend(&w.pets[0], |_| true), None, "up the screen");
    w.pets[1].floor = 0.0;
    w.pets[1].x = 900.0;
    assert_eq!(w.friend(&w.pets[0], |_| true), None, "too far along the Dock");
}

#[test]
fn sleeping_side_by_side_counts_as_snuggling() {
    let mut w = world(2);
    for i in 0..2 { w.with_pet(i, |w, p| w.start(p, Sleep, false)) }
    w.pets[1].x = w.pets[0].x + w.pets[0].size.width;
    assert!(w.snuggled(&w.pets[0]));
    w.pets[1].x += 200.0;
    assert!(!w.snuggled(&w.pets[0]));
}

#[test]
fn a_new_activity_drops_the_friend() {
    let mut w = world(2);
    w.with_pet(0, |w, p| {
        p.buddy = Some(2);
        w.start(p, Sit, false);
        assert_eq!(p.buddy, None);
    });
}
