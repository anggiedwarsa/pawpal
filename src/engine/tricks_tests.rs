//! Unit tests for engine/tricks.rs.

use super::*;
use crate::testing::{skin, world};

fn run(w: &mut World, seconds: f64) {
    for _ in 0..(seconds * 30.0) as usize { w.with_pet(0, |w, p| w.update(p, 1.0 / 30.0)) }
}

fn ask(w: &mut World, t: Trick) -> bool { w.with_pet(0, |w, p| w.trick(p, t)) }

#[test]
fn each_trick_is_shown_by_its_activity() {
    for t in Trick::ALL {
        let mut w = world(1);
        assert!(ask(&mut w, t), "{t:?}");
        let p = &w.pets[0];
        assert_eq!((p.activity, p.commanded, p.trick), (t.activity(), true, true));
        assert!(p.bubble.is_some(), "{t:?} says something");
    }
}

#[test]
fn a_finished_trick_makes_the_pet_proud() {
    let mut w = world(1);
    w.pets[0].happiness = 50.0;
    ask(&mut w, Trick::HighFive);
    run(&mut w, 3.5);
    let p = &w.pets[0];
    assert!(!p.trick);
    assert_eq!(p.happiness.round(), (50.0 + PROUD).round(), "proud, less a little drift");
    assert_eq!(p.activity, Sit, "it sits for its praise");
    assert!(p.bubble.as_deref().unwrap().contains("Ta-da"));
    assert!(w.particles.iter().any(|q| q.text == "⭐"));
}

#[test]
fn a_jump_counts_once_it_lands() {
    let mut w = world(1);
    ask(&mut w, Trick::Jump);
    run(&mut w, 2.0);
    assert!(w.pets[0].bubble.as_deref().unwrap().contains("Ta-da"));
}

#[test]
fn an_interrupted_trick_earns_nothing() {
    let mut w = world(1);
    w.pets[0].happiness = 50.0;
    ask(&mut w, Trick::Dance);
    w.with_pet(0, |w, p| w.start(p, Dragged, false));
    assert!(!w.pets[0].trick);
}

#[test]
fn grumpy_or_sleepy_pets_say_no() {
    let mut w = world(1);
    w.pets[0].happiness = GRUMPY;
    w.with_pet(0, |w, p| w.start(p, Walk, false));
    assert!(!ask(&mut w, Trick::Sit));
    assert_eq!(w.pets[0].activity, Walk, "it carries on with what it was doing");
    assert!(w.pets[0].bubble.as_deref().unwrap().contains("ignores"));

    let mut w = world(1);
    w.pets[0].energy = SLEEPY;
    assert!(!ask(&mut w, Trick::Dance));
    assert!(w.pets[0].bubble.as_deref().unwrap().contains("sleepy"));
}

#[test]
fn no_clip_no_trick() {
    let mut w = world(1);
    w.pets[0].skin = Some(skin(&["walk"]));
    assert!(!ask(&mut w, Trick::Sit));
    assert_ne!(w.pets[0].activity, Sit);
}
