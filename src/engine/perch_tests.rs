//! Unit tests for engine/perch.rs.

use super::*;
use crate::testing::world;

/// A 400×250 window whose top edge is 300 points above the window's bottom, in a world whose Dock line is at 6.
fn window() -> Rect { Rect::new(100.0, 50.0, 400.0, 250.0) }

fn run(w: &mut World, seconds: f64) {
    for _ in 0..(seconds * 30.0) as usize {
        w.with_pet(0, |w, p| {
            w.follow_perch(p);
            w.update(p, 1.0 / 30.0);
        });
    }
}

/// Hops pet 0 up onto the window and waits for it to land.
fn perch(w: &mut World) {
    w.perch = Some(window());
    let spot = w.with_pet(0, |w, p| w.perch_spot(p)).expect("room to perch");
    w.with_pet(0, |w, p| w.perch_on(p, spot));
    for _ in 0..60 {
        if !w.pets[0].leaping { break }
        run(w, 1.0 / 30.0);
    }
}

#[test]
fn it_hops_onto_the_window_top_and_lives_there() {
    let mut w = world(1);
    perch(&mut w);
    let p = &w.pets[0];
    assert_eq!(p.perched, Some(window()));
    assert_eq!(p.floor, 300.0 - w.ground, "standing on the top edge");
    assert_eq!(p.span, Some(Span { lo: 100.0, hi: 500.0 }), "roaming the window's width");
    assert!(p.x >= 100.0 && p.x <= 500.0);
}

#[test]
fn it_rides_along_when_the_window_moves() {
    let mut w = world(1);
    perch(&mut w);
    let (x, floor) = (w.pets[0].x, w.pets[0].floor);
    w.with_pet(0, |w, p| { w.start(p, Sit, true); p.timer = 100.0 });
    w.perch = Some(Rect::new(160.0, 20.0, 400.0, 250.0));
    run(&mut w, 0.1);
    let p = &w.pets[0];
    assert_eq!((p.x, p.floor), (x + 60.0, floor - 30.0));
    assert_eq!(p.span, Some(Span { lo: 160.0, hi: 560.0 }));
}

#[test]
fn it_jumps_down_when_the_window_goes() {
    for next in [None, Some(Rect::new(0.0, 0.0, 800.0, 500.0))] {
        let mut w = world(1);
        perch(&mut w);
        w.perch = next;
        run(&mut w, 0.1);
        assert_eq!(w.pets[0].perched, None, "{next:?}");
        assert_eq!(w.pets[0].activity, Falling);
        run(&mut w, 3.0);
        assert_eq!(w.pets[0].floor, 0.0, "back on the Dock");
    }
}

#[test]
fn leaving_the_window_on_its_own_lets_it_go() {
    let mut w = world(1);
    perch(&mut w);
    w.with_pet(0, |w, p| w.start(p, Sleep, false));
    run(&mut w, 6.0);
    assert_eq!(w.pets[0].perched, None);
    w.perch = None;
    run(&mut w, 0.1);
    assert_ne!(w.pets[0].activity, Falling, "a window closing behind it doesn't matter any more");
}

#[test]
fn no_perch_without_room_or_a_hop_clip_or_during_a_film() {
    let mut w = world(1);
    w.perch = Some(Rect::new(0.0, 0.0, 1000.0, 590.0));
    assert_eq!(w.with_pet(0, |w, p| w.perch_spot(p)), None, "a maximised window leaves no room above it");
    w.perch = Some(Rect::new(100.0, 0.0, 80.0, 300.0));
    assert_eq!(w.with_pet(0, |w, p| w.perch_spot(p)), None, "too narrow");
    w.perch = Some(Rect::new(100.0, 0.0, 400.0, 20.0));
    assert_eq!(w.with_pet(0, |w, p| w.perch_spot(p)), None, "hardly above the Dock");
    w.perch = Some(window());
    w.fullscreen = true;
    assert_eq!(w.with_pet(0, |w, p| w.perch_spot(p)), None);
    w.fullscreen = false;
    w.pets[0].skin = Some(crate::testing::skin(&["walk", "sit"]));
    assert_eq!(w.with_pet(0, |w, p| w.perch_spot(p)), None, "no jump clip");
}

#[test]
fn free_will_sometimes_perches() {
    let mut w = world(1);
    w.perch = Some(window());
    let hops = (0..400).filter(|_| {
        w.with_pet(0, |w, p| {
            (p.y, p.vy, p.floor, p.perched, p.span, p.leaping) = (0.0, 0.0, 0.0, None, None, false);
            w.choose_next(p);
        });
        w.pets[0].perch_pending.is_some()
    }).count();
    assert!((10..80).contains(&hops), "now and then: {hops}/400");
}
