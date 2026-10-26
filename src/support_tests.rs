//! Unit tests for support.rs.

use super::*;

#[test]
fn rects_intersect_only_with_shared_area() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert!(a.intersects(&Rect::new(5.0, 5.0, 10.0, 10.0)));
    assert!(!a.intersects(&Rect::new(10.0, 0.0, 5.0, 5.0)), "touching edges don't overlap");
    assert!(!a.intersects(&Rect::new(2.0, 2.0, 0.0, 5.0)), "empty rects never overlap");
}

#[test]
fn rect_contains_its_min_edge_but_not_its_max() {
    let r = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert!(r.contains(pt(0.0, 0.0)));
    assert!(!r.contains(pt(10.0, 5.0)));
    assert_eq!((r.max_x(), r.max_y()), (10.0, 10.0));
}

#[test]
fn clamp_prefers_the_upper_bound_when_bounds_cross() {
    assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
    assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
    assert_eq!(clamp(5.0, 8.0, 3.0), 3.0);
}

#[test]
fn lerp_rnd_and_pick_stay_in_range() {
    assert_eq!(lerp(2.0, 4.0, 0.5), 3.0);
    for _ in 0..100 {
        let v = rnd(3.0, 4.0);
        assert!((3.0..=4.0).contains(&v));
        assert!([1, 2, 3].contains(&pick(&[1, 2, 3])));
    }
    assert!(!chance(0.0));
    assert!(chance(1.0));
}

#[test]
fn points_add_and_spans_are_closed() {
    assert_eq!(pt(1.0, 2.0) + pt(3.0, 4.0), pt(4.0, 6.0));
    let s = Span { lo: 1.0, hi: 2.0 };
    assert!(s.contains(1.0) && s.contains(2.0) && !s.contains(2.1));
}

#[test]
fn colours_come_from_hex_and_darken_towards_black() {
    let c = Rgba::hex(0xFF8000);
    assert_eq!((c.r, c.g, c.b, c.a), (1.0, 128.0 / 255.0, 0.0, 1.0));
    let d = c.darker(0.5);
    assert_eq!((d.r, d.a), (0.5, 1.0));
    assert_eq!(Rgba::white(0.5, 0.2), Rgba { r: 0.5, g: 0.5, b: 0.5, a: 0.2 });
}
