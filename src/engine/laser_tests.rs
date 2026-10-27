//! Unit tests for engine/laser.rs.

use super::*;
use crate::testing::world;

#[test]
fn turning_the_laser_on_sets_pets_chasing_and_off_lets_them_go() {
    let mut w = world(2);
    w.with_pet(1, |w, p| w.start(p, Sleep, false));
    w.set_laser(true);
    assert_eq!(w.pets[0].activity, ChaseCursor);
    assert_eq!(w.pets[1].activity, Sleep, "a sleeping pet isn't woken for it");
    assert!(w.headroom().is_infinite() && w.is_fast(), "the dot can be anywhere, and moves fast");
    w.set_laser(false);
    assert_eq!(w.pets[0].timer, 0.0, "it gives up the chase");
}

#[test]
fn pets_run_to_the_dot_and_pounce_when_its_just_above() {
    let mut w = world(1);
    w.set_laser(true);
    w.mouse = pt(600.0, w.ground + 20.0);
    w.with_pet(0, |w, p| w.chase_laser(p, 0.03));
    assert!(w.pets[0].vx > 100.0, "it dashes over");
    w.mouse = pt(200.0, w.ground + 60.0);
    w.with_pet(0, |w, p| w.chase_laser(p, 0.03));
    assert!(w.pets[0].vy > 0.0 && w.pets[0].cooldown > 0.0, "right under it, it leaps");
}

#[test]
fn a_dot_high_up_the_screen_makes_the_pet_hop_after_it() {
    let mut w = world(1);
    w.set_laser(true);
    w.mouse = pt(260.0, w.ground + 400.0);
    w.with_pet(0, |w, p| w.chase_laser(p, 0.03));
    let p = &w.pets[0];
    assert_eq!(p.activity, Hop);
    assert!(p.leap_to.y > 300.0 && (p.leap_to.x - 260.0).abs() < 1.0);
}

#[test]
fn free_will_goes_back_to_the_dot_while_the_laser_is_on() {
    let mut w = world(1);
    w.laser = true;
    let chased = (0..50).filter(|_| { w.with_pet(0, |w, p| w.choose_next(p)); w.pets[0].activity == ChaseCursor }).count();
    assert!(chased > 30, "chased {chased} of 50 times");
}
