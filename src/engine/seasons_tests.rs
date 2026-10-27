//! Unit tests for engine/seasons.rs.

use super::*;
use crate::testing::world;

#[test]
fn the_calendar_has_its_touches() {
    assert_eq!(touch(12, 10, 12.0), Some(Touch::Snow));
    assert_eq!(touch(1, 20, 12.0), Some(Touch::Snow));
    assert_eq!(touch(10, 27, 12.0), Some(Touch::Leaves));
    assert_eq!(touch(11, 5, 12.0), Some(Touch::Leaves));
    assert_eq!(touch(4, 2, 12.0), Some(Touch::Blossoms));
    assert_eq!(touch(10, 31, 9.0), Some(Touch::Halloween));
    assert_eq!(touch(7, 14, 12.0), None);
}

#[test]
fn new_year_is_the_evening_before_and_the_small_hours_after() {
    assert_eq!(touch(12, 31, 19.0), Some(Touch::Snow));
    assert_eq!(touch(12, 31, 23.5), Some(Touch::NewYear));
    assert_eq!(touch(1, 1, 0.5), Some(Touch::NewYear));
    assert_eq!(touch(1, 1, 3.0), Some(Touch::Snow));
    assert_eq!(Touch::NewYear.line(1), "🎉 Happy New Year!");
    assert_eq!(Touch::NewYear.line(12), "🎉 Almost New Year!");
}

fn run(w: &mut World, seconds: f64) {
    for _ in 0..(seconds * 30.0) as usize { w.seasonal_step(1.0 / 30.0) }
}

#[test]
fn a_flurry_drifts_down_inside_the_strip_now_and_then() {
    let mut w = world(1);
    w.touch = Some(Touch::Snow);
    run(&mut w, 29.0);
    assert!(w.particles.is_empty(), "not yet");
    run(&mut w, 2.0);
    assert!(!w.particles.is_empty());
    run(&mut w, FLURRY);
    let n = w.particles.len();
    assert!(n > 10, "a flurry: {n}");
    assert!(w.particles.iter().all(|q| q.p.y + q.size <= w.ground + 180.0 && q.v.y < 0.0), "falling, inside the compact window");
    assert!(w.flurry.left <= 0.0 && w.flurry.next >= EVERY.0 - FLURRY, "then a long pause");
}

#[test]
fn nothing_falls_out_of_season_during_films_or_focus() {
    for setup in [|w: &mut World| w.touch = None, |w: &mut World| w.fullscreen = true, |w: &mut World| w.focusing = true] {
        let mut w = world(1);
        w.touch = Some(Touch::Leaves);
        setup(&mut w);
        run(&mut w, 60.0);
        assert!(w.particles.is_empty());
    }
}
