//! Unit tests for engine/behavior.rs.

use super::*;
use crate::testing::{skin, world};

fn run(w: &mut World, i: usize, seconds: f64) {
    for _ in 0..(seconds * 30.0) as usize { w.with_pet(i, |w, p| w.update(p, 1.0 / 30.0)) }
}

#[test]
fn jumping_pets_land_again() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.start(p, Jump, true));
    assert!(w.pets[0].vy > 0.0);
    run(&mut w, 0, 0.3);
    assert!(w.pets[0].y > 0.0, "it is in the air");
    // Checked as soon as it lands: what it does next is random (and may be another hop).
    for _ in 0..60 {
        if w.pets[0].activity != Jump { break }
        run(&mut w, 0, 1.0 / 30.0);
    }
    assert_ne!(w.pets[0].activity, Jump, "it moves on once landed");
    assert_eq!(w.pets[0].floor, 0.0);
}

#[test]
fn walking_reaches_its_target() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        w.start(p, Walk, true);
        p.target_x = Some(400.0);
        p.timer = 100.0;
    });
    // Checked on arrival, when the walk (and its target) ends: next it may hop or walk off somewhere else.
    for _ in 0..300 {
        if w.pets[0].activity != Walk || w.pets[0].target_x != Some(400.0) { break }
        run(&mut w, 0, 1.0 / 30.0);
    }
    assert!((w.pets[0].x - 400.0).abs() < 5.0, "{}", w.pets[0].x);
}

#[test]
fn eating_empties_the_bowl_and_fills_the_pet() {
    let mut w = world(1);
    w.pets[0].hunger = 80.0;
    w.with_pet(0, |w, p| w.start(p, Eat, true));
    assert_eq!(w.bowls.len(), 1, "a commanded meal puts a bowl down");
    run(&mut w, 0, 12.0);
    assert!(w.pets[0].hunger < 80.0);
    assert!(w.bowls.is_empty() || w.bowls[0].food < 1.0);
}

#[test]
fn zoomies_dust_comes_from_the_pets_paws_up_high() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        (p.y, p.floor, p.span) = (200.0, 200.0, Some(Span { lo: 50.0, hi: 950.0 }));
        w.start(p, Run, true);
    });
    run(&mut w, 0, 1.0);
    let dust: Vec<_> = w.particles.iter().filter(|q| q.text == "💨").collect();
    assert!(!dust.is_empty());
    assert!(dust.iter().all(|q| q.p.y > 200.0), "the dust follows the pet, not the Dock");
}

#[test]
fn sleeping_pets_wake_with_a_stretch() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        w.start(p, Sleep, false);
        (p.step, p.timer) = (1, 0.01);
    });
    run(&mut w, 0, 0.1);
    assert_eq!(w.pets[0].activity, Stretch);
    assert!(w.pets[0].bubble.as_deref().unwrap().contains("yaaawn"));
}

#[test]
fn needs_drift_over_time() {
    let mut w = world(1);
    w.with_pet(0, |w, p| { w.start(p, Sit, true); p.timer = 1000.0 });
    let (h, e) = (w.pets[0].hunger, w.pets[0].energy);
    run(&mut w, 0, 10.0);
    assert!(w.pets[0].hunger > h && w.pets[0].energy < e);
}

#[test]
fn free_will_only_picks_what_the_pet_can_show() {
    let mut w = world(1);
    w.pets[0].skin = Some(skin(&["sit"]));
    for _ in 0..50 {
        w.with_pet(0, |w, p| w.choose_next(p));
        let p = &w.pets[0];
        assert!(p.supports(p.activity), "{:?}", p.activity);
        assert!(!p.supports(Walk) && p.activity != Walk, "it never walks without a walk clip");
    }
}

#[test]
fn hungry_pets_head_for_food_on_their_level() {
    let mut w = world(1);
    w.pets[0].hunger = 50.0;
    w.bowls.push(crate::domain::Bowl { x: 500.0, food: 1.0, life: 100.0, floor: 0.0 });
    w.with_pet(0, |w, p| w.choose_next(p));
    assert_eq!(w.pets[0].activity, Eat);
    w.pets[0].floor = 300.0;
    w.with_pet(0, |w, p| w.choose_next(p));
    assert_ne!(w.pets[0].activity, Eat, "a bowl on another level is out of reach");
}

#[test]
fn pets_play_tag_with_friends() {
    let mut w = world(2);
    w.pets[1].x = 260.0;
    w.with_pet(0, |w, p| w.start(p, PlayFriend, true));
    run(&mut w, 0, 3.0);
    assert_eq!(w.pets[1].activity, Jump, "the friend got tagged");
}

#[test]
fn hopping_up_makes_a_new_home() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        w.start(p, Hop, true);
        p.leap_to = pt(p.x + 50.0, 150.0);
    });
    // Checked on landing: what it does next is random.
    for _ in 0..60 {
        if w.pets[0].activity != Hop { break }
        run(&mut w, 0, 1.0 / 30.0);
    }
    assert_eq!(w.pets[0].floor, 150.0);
}

#[test]
fn peekaboo_ducks_and_comes_back() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.start(p, Peekaboo, true));
    run(&mut w, 0, 0.7);
    assert_eq!(w.pets[0].sink, 1.0);
    run(&mut w, 0, 7.0);
    assert_eq!(w.pets[0].sink, 0.0);
    assert_eq!(w.pets[0].activity, Wave);
}

#[test]
fn visiting_pets_leap_to_the_cursor_then_settle() {
    let mut w = world(1);
    w.mouse = pt(600.0, 300.0);
    w.with_pet(0, |w, p| w.start(p, VisitCursor, false));
    run(&mut w, 0, 1.5);
    assert_eq!(w.pets[0].step, 1, "arrived by the cursor");
    w.mouse = pt(100.0, 100.0);
    run(&mut w, 0, 0.1);
    assert_eq!(w.pets[0].activity, Sit);
    assert!(w.pets[0].floor > 0.0, "it stays up there");
}

#[test]
fn falling_pets_land_and_idle() {
    let mut w = world(1);
    w.with_pet(0, |w, p| { p.y = 100.0; w.start(p, Falling, false) });
    run(&mut w, 0, 2.0);
    assert_eq!((w.pets[0].y, w.pets[0].activity == Falling), (0.0, false));
}

#[test]
fn petting_and_picking_up_lines() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.start(p, Petted, true));
    assert!(w.pets[0].happiness > 70.0);
    assert_eq!(picked_up_line(Species::Dog), "Whoa!");
}

#[test]
fn bedtime_is_in_the_bottom_right_corner() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.start(p, Sleep, false));
    assert_eq!(w.pets[0].timer, f64::INFINITY, "the nap starts at the bed");
    run(&mut w, 0, 20.0);
    let p = &w.pets[0];
    assert_eq!(p.step, 1, "it reached its bed");
    assert!(p.x > 900.0, "the bed is at the right end of the Dock, not where it was ({})", p.x);
    assert!(p.timer.is_finite());
}

#[test]
fn pets_up_the_screen_come_down_to_sleep() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        (p.y, p.floor, p.span, p.energy) = (200.0, 200.0, Some(Span { lo: 100.0, hi: 400.0 }), 5.0);
        w.start(p, Sleep, false);
    });
    assert_eq!(w.pets[0].step, HOP_DOWN, "it doesn't drop straight to the Dock");
    assert!(w.pets[0].leaping);
    run(&mut w, 0, 25.0);
    let p = &w.pets[0];
    assert_eq!((p.y, p.floor, p.step, p.activity), (0.0, 0.0, ASLEEP, Sleep));
    assert!(p.x > 900.0, "it went to the corner bed ({})", p.x);
}

#[test]
fn pets_hop_down_one_level_at_a_time() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        (p.y, p.floor, p.span) = (400.0, 400.0, Some(Span { lo: 100.0, hi: 400.0 }));
        w.start(p, Sleep, false);
    });
    let mut floors = vec![400.0];
    for _ in 0..900 {
        w.with_pet(0, |w, p| w.update(p, 1.0 / 30.0));
        let p = &w.pets[0];
        assert!(p.y <= 400.0 + 25.0 + 1.0, "hops go down, with only a little arc ({})", p.y);
        if floors.last() != Some(&p.floor) { floors.push(p.floor) }
        if p.step == ASLEEP { break }
    }
    assert!(floors.len() >= 3, "it landed on levels on the way down: {floors:?}");
    assert!(floors.windows(2).all(|f| f[1] < f[0] && f[0] - f[1] <= 150.0 + 30.0), "each hop is one level lower: {floors:?}");
    assert_eq!(w.pets[0].step, ASLEEP);
}

#[test]
fn sent_to_sleep_from_up_the_screen_goes_to_the_corner_too() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        (p.y, p.floor, p.span) = (300.0, 300.0, Some(Span { lo: 100.0, hi: 400.0 }));
        w.start(p, Sleep, true);
    });
    assert_eq!(w.pets[0].step, HOP_DOWN);
    run(&mut w, 0, 25.0);
    let p = &w.pets[0];
    assert_eq!((p.y, p.step, p.activity), (0.0, ASLEEP, Sleep));
    assert!(p.x > 900.0, "{}", p.x);
}

#[test]
fn sleepy_pets_curl_up_side_by_side() {
    let mut w = world(2);
    for i in 0..2 { w.with_pet(i, |w, p| w.start(p, Sleep, false)) }
    let (a, b) = (w.pets[0].target_x.unwrap(), w.pets[1].target_x.unwrap());
    assert!(a > b, "the second pet takes the next spot to the left");
}

#[test]
fn energy_only_comes_back_once_asleep() {
    let mut w = world(1);
    w.pets[0].energy = 50.0;
    w.with_pet(0, |w, p| w.start(p, Sleep, false));
    run(&mut w, 0, 1.0);
    assert!(w.pets[0].energy <= 50.0, "walking to bed is not resting");
}

#[test]
fn woken_pets_go_back_to_bed_after_a_while() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        p.back_to_sleep = Some(0.5);
        w.start(p, Sit, true);
        p.timer = 100.0;
    });
    run(&mut w, 0, 1.0);
    assert_eq!(w.pets[0].activity, Sleep);
    assert_eq!(w.pets[0].back_to_sleep, None);
}

#[test]
fn nothing_wanders_in_front_of_a_full_screen_video() {
    let mut w = world(1);
    w.fullscreen = true;
    w.with_pet(0, |w, p| w.choose_next(p));
    assert_eq!(w.pets[0].activity, Hide);
}

#[test]
fn past_the_end_of_the_dock_the_bed_is_down_at_the_screen_edge() {
    let mut w = world(1);
    (w.below, w.dock_start, w.dock_end, w.max_x) = (60.0, 50.0, 800.0, 800.0);
    w.pets[0].energy = 5.0;
    w.with_pet(0, |w, p| w.start(p, Sleep, false));
    assert!(w.pets[0].target_x.unwrap() > 800.0, "the bed is past the Dock's end");
    run(&mut w, 0, 25.0);
    let p = &w.pets[0];
    assert_eq!((p.step, p.floor, p.y), (ASLEEP, -60.0, -60.0), "it hopped off the Dock into the corner");
    assert!(p.x > 800.0, "{}", p.x);
}

#[test]
fn woken_in_the_corner_it_hops_back_onto_the_dock() {
    let mut w = world(1);
    (w.below, w.dock_start, w.dock_end, w.max_x) = (60.0, 50.0, 800.0, 800.0);
    w.pets[0].energy = 5.0;
    w.with_pet(0, |w, p| w.start(p, Sleep, false));
    run(&mut w, 0, 25.0);
    w.with_pet(0, |w, p| w.start(p, Walk, false));
    assert_eq!(w.pets[0].activity, Hop, "it can't walk about down there");
    for _ in 0..60 {
        if w.pets[0].activity != Hop { break }
        run(&mut w, 0, 1.0 / 30.0);
    }
    let p = &w.pets[0];
    assert_eq!((p.floor, p.y), (0.0, 0.0));
    assert!(p.x <= 800.0, "it's back on the Dock ({})", p.x);
}

#[test]
fn walking_the_whole_screen_still_puts_the_bed_down_past_the_dock() {
    let mut w = world(1);
    (w.min_x, w.max_x) = (0.0, 1000.0);
    (w.below, w.dock_start, w.dock_end) = (60.0, 50.0, 800.0);
    w.pets[0].energy = 5.0;
    w.with_pet(0, |w, p| w.start(p, Sleep, false));
    run(&mut w, 0, 25.0);
    let p = &w.pets[0];
    assert_eq!((p.step, p.floor), (ASLEEP, -60.0));
}

/// The whole screen to walk on, with the Dock from 300 to 700 standing 60 above the screen's bottom edge.
fn stepped_world() -> World {
    let mut w = world(1);
    (w.min_x, w.max_x) = (0.0, 1000.0);
    (w.below, w.dock_start, w.dock_end) = (60.0, 300.0, 700.0);
    w
}

fn walk_to(w: &mut World, x: f64) {
    w.with_pet(0, |w, p| {
        w.start(p, Walk, true);
        p.target_x = Some(x);
        p.timer = 100.0;
    });
    for _ in 0..600 {
        let p = &w.pets[0];
        if p.target_x != Some(x) && !p.climbing { break }
        run(w, 0, 1.0 / 30.0);
    }
}

#[test]
fn walking_off_the_end_of_the_dock_drops_to_the_screen_bottom() {
    let mut w = stepped_world();
    w.pets[0].x = 600.0;
    walk_to(&mut w, 900.0);
    let p = &w.pets[0];
    assert_eq!((p.floor, p.y), (-60.0, -60.0), "beside the Dock the ground is the screen's bottom edge");
}

#[test]
fn walking_into_the_dock_from_beside_it_hops_up_onto_it() {
    let mut w = stepped_world();
    w.with_pet(0, |w, p| {
        (p.x, p.y) = (100.0, -60.0);
        w.go_home(p, false);
    });
    assert_eq!(w.pets[0].floor, -60.0);
    w.with_pet(0, |w, p| {
        w.start(p, Walk, true);
        p.target_x = Some(500.0);
        p.timer = 100.0;
    });
    for _ in 0..600 {
        if w.pets[0].climbing { break }
        run(&mut w, 0, 1.0 / 30.0);
    }
    let p = &w.pets[0];
    assert!(p.climbing && p.leap_to.x > p.leap_from.x, "it climbs on rightwards");
    assert_eq!(p.facing, 1.0, "coming from the left, it faces right as it climbs");
    walk_to(&mut w, 500.0);
    let p = &w.pets[0];
    assert_eq!((p.floor, p.y), (0.0, 0.0), "it climbed onto the Dock");
    assert!(p.x > 300.0, "{}", p.x);
}

#[test]
fn climbing_on_from_the_right_faces_left() {
    let mut w = stepped_world();
    w.with_pet(0, |w, p| {
        (p.x, p.y) = (900.0, -60.0);
        w.go_home(p, false);
        w.start(p, Walk, true);
        p.target_x = Some(500.0);
        p.timer = 100.0;
    });
    for _ in 0..600 {
        if w.pets[0].climbing { break }
        run(&mut w, 0, 1.0 / 30.0);
    }
    let p = &w.pets[0];
    assert!(p.climbing && p.leap_to.x < p.leap_from.x);
    assert_eq!(p.facing, -1.0);
}
