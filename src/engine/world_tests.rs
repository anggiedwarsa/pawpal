//! Unit tests for engine/world.rs.

use super::*;
use crate::testing::world;

#[test]
fn with_pet_lifts_a_pet_out_and_puts_it_back() {
    let mut w = world(2);
    w.with_pet(0, |w, p| {
        assert_eq!(w.pets[0].id, 0, "a placeholder holds the slot");
        p.name = "Changed".into();
    });
    assert_eq!((w.pets[0].id, w.pets[0].name.as_str()), (1, "Changed"));
    assert_eq!(w.index_of(2), Some(1));
    assert_eq!(w.index_of(0), None, "placeholders are never found");
}

#[test]
fn pets_stay_within_their_range() {
    let w = world(1);
    let p = &w.pets[0];
    assert_eq!(w.clamp_x(p, -100.0, 10.0), 60.0);
    assert_eq!(w.clamp_x(p, 5000.0, 10.0), 940.0);
    for _ in 0..50 {
        let x = w.random_x_for(p);
        assert!((80.0..=920.0).contains(&x));
    }
}

#[test]
fn settling_up_high_makes_a_new_home() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        p.y = 200.0;
        w.settle(p);
        assert_eq!(p.floor, 200.0);
        assert_eq!(p.span, Some(Span { lo: 200.0 - ROAM, hi: 200.0 + ROAM }));
        p.y = 5.0;
        w.settle(p);
        assert_eq!((p.floor, p.span), (0.0, None), "dropped near the Dock, it lives on the Dock");
    });
}

#[test]
fn going_home_falls_back_to_the_dock() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        p.floor = 120.0;
        w.go_home(p, true);
        assert_eq!((p.floor, p.activity), (0.0, Falling));
    });
}

#[test]
fn hops_never_land_where_the_user_types() {
    let mut w = world(1);
    w.typing_zone = Some(Rect::new(0.0, 30.0, 1000.0, 600.0));
    for _ in 0..30 {
        let spot = w.hop_target(&w.pets[0]);
        assert_eq!(spot.y, 0.0, "the only free spot is the Dock");
    }
}

#[test]
fn geometry_follows_the_pet() {
    let mut w = world(1);
    let p = &mut w.pets[0];
    (p.x, p.y, p.size) = (100.0, 50.0, Size { width: 40.0, height: 30.0 });
    let p = &w.pets[0];
    assert_eq!(w.pet_rect(p), Rect::new(80.0, 56.0, 40.0, 30.0));
    assert_eq!(w.bubble_y(p), 6.0 + 50.0 + 30.0 + 8.0);
    assert!(w.head_point(p).x > p.x, "facing right, the head is on the right");
}

#[test]
fn perch_sits_beside_the_cursor() {
    let mut w = world(1);
    w.mouse = pt(500.0, 300.0);
    let spot = w.perch_point(&w.pets[0]);
    assert!(spot.x < 500.0, "a pet left of the cursor sits on its left");
    assert_eq!(spot.y, 300.0 - 6.0 - 10.0);
}

#[test]
fn typing_sits_pets_down_or_moves_them_out_of_the_way() {
    let mut w = world(2);
    w.typing_zone = Some(Rect::new(150.0, 0.0, 100.0, 100.0));
    w.typing_zone_is_exact = true;
    w.pets[0].size = Size { width: 40.0, height: 40.0 };
    w.watch_typing(0.1);
    assert_eq!(w.pets[0].activity, Walk, "the pet under the caret walks away");
    assert!(w.pets[0].commanded);
    assert_eq!(w.pets[1].activity, Sit, "the other pet sits and watches");
    assert!((w.pets[1].timer - (TYPING_PAUSE - 0.1)).abs() < 1e-9);
}

#[test]
fn calm_until_something_moves() {
    let mut w = world(1);
    w.pets[0].activity = Sleep;
    assert!(w.is_calm());
    w.emit("z", pt(0.0, 0.0), 10.0);
    assert!(!w.is_calm());
}

#[test]
fn stepping_tracks_the_cursor_and_visits_after_idling() {
    let mut w = world(1);
    w.step(0.1, pt(10.0, 10.0), f64::INFINITY);
    assert_eq!((w.mouse, w.mouse_still), (pt(10.0, 10.0), 0.0));
    w.step(0.1, pt(10.5, 10.0), f64::INFINITY);
    assert!(w.mouse_still > 0.0, "tiny jitters don't count as moving");
    w.step(0.1, pt(10.0, 10.0), 0.2);
    assert_eq!(w.mouse_still, 0.0, "typing counts as being around");
}

#[test]
fn headroom_covers_pets_and_their_bubbles() {
    let mut w = world(1);
    w.pets[0].size = Size { width: 60.0, height: 50.0 };
    assert_eq!(w.headroom(), w.ground + 50.0);
    w.pets[0].say("Meow!", 2.0);
    assert!(w.headroom() > w.ground + 100.0, "room for the bubble above it");
}

#[test]
fn a_pet_about_to_hop_up_needs_room_before_it_leaves() {
    let mut w = world(1);
    w.with_pet(0, |w, p| {
        w.start(p, Hop, true);
        p.leap_to = pt(p.x, 300.0);
    });
    assert!(w.headroom() >= 300.0, "the window grows before the jump, not halfway through it");
}

#[test]
fn pets_that_can_go_anywhere_need_the_whole_screen() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.start(p, Dragged, true));
    assert_eq!(w.headroom(), f64::INFINITY);
    let mut w = world(1);
    w.remind("📅 Standup in 5 minutes");
    assert_eq!(w.headroom(), f64::INFINITY);
    let mut w = world(1);
    w.pets[0].y = 200.0;
    w.with_pet(0, |w, p| w.settle(p));
    assert_eq!(w.headroom(), f64::INFINITY, "a pet living up the screen");
}
