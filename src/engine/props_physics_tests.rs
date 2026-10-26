//! Unit tests for engine/props_physics.rs.

use super::*;
use crate::testing::world;

#[test]
fn bowls_go_down_on_the_pets_own_level() {
    let mut w = world(1);
    w.pets[0].floor = 150.0;
    w.pets[0].span = Some(Span { lo: 100.0, hi: 400.0 });
    w.feed();
    assert_eq!(w.bowls.len(), 1);
    assert_eq!(w.bowls[0].floor, 150.0);
    assert!(w.bowls[0].x >= 130.0 && w.bowls[0].x <= 370.0, "within the pet's stretch");
    assert_eq!(w.pets[0].activity, Eat);
    assert_eq!(w.pets[0].floor, 150.0, "it eats up there instead of going back to the Dock");
}

#[test]
fn balls_land_where_the_pets_are_and_only_they_play() {
    let mut w = world(2);
    w.pets[0].floor = 200.0;
    w.pets[0].span = Some(Span { lo: 100.0, hi: 300.0 });
    w.throw_ball_at(200.0, 200.0, Span { lo: 100.0, hi: 300.0 });
    assert_eq!(w.pets[0].activity, PlayBall);
    assert_ne!(w.pets[1].activity, PlayBall, "the pet on the Dock can't reach it");
    for _ in 0..600 { w.update_props(1.0 / 60.0) }
    let b = &w.balls[0];
    assert!(b.y >= 0.0 && b.x >= 107.0 && b.x <= 293.0, "it bounces on its own level");
}

#[test]
fn butterflies_flutter_above_a_pets_level() {
    let mut w = world(1);
    w.release_butterfly();
    assert_eq!(w.butterflies[0].floor, 0.0);
    assert_eq!(w.pets[0].activity, ChaseButterfly);
    for _ in 0..120 { w.update_props(1.0 / 30.0) }
    let b = &w.butterflies[0];
    assert!(b.x >= w.min_x && b.x <= w.max_x);
}

#[test]
fn props_wear_out() {
    let mut w = world(0);
    w.throw_ball();
    w.bowls.push(Bowl { x: 100.0, food: 0.0, life: 120.0, floor: 0.0 });
    w.emit("♥", pt(0.0, 0.0), 10.0);
    w.update_props(1.0);
    assert_eq!(w.bowls.len(), 1, "an empty bowl lingers for a moment");
    w.update_props(2.5);
    assert!(w.bowls.is_empty(), "then it's cleared away");
    assert!(w.particles.is_empty());
    w.update_props(60.0);
    assert!(w.balls.is_empty());
}

#[test]
fn treats_cheer_everyone_up() {
    let mut w = world(2);
    w.pets[0].hunger = 50.0;
    w.treat();
    assert_eq!(w.pets[0].hunger, 35.0);
    assert!(w.pets.iter().all(|p| p.activity == Petted && p.bubble.is_some()));
}

#[test]
fn kicked_balls_fly_up() {
    let mut w = world(0);
    w.throw_ball();
    w.balls[0].life = 1.0;
    w.kick_ball(0);
    assert!(w.balls[0].vy > 0.0 && w.balls[0].life == 60.0);
}
