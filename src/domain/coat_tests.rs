//! Unit tests for domain/coat.rs.

use super::*;

const SCALE: [u8; 3] = [152, 124, 222];
const WING: [u8; 3] = [196, 170, 246];
const BELLY: [u8; 3] = [255, 222, 140];
const HORN: [u8; 3] = [255, 244, 212];
const OUTLINE: [u8; 3] = [36, 26, 48];
const FIRE: [u8; 3] = [255, 122, 38];
const BLUSH: [u8; 3] = [250, 150, 170];

#[test]
fn the_natural_coat_leaves_the_art_alone() {
    for c in [SCALE, WING, BELLY, OUTLINE, FIRE] { assert_eq!(Coat::Natural.recolor(c), c) }
}

#[test]
fn only_the_scales_change_colour() {
    for coat in Coat::ALL {
        for keep in [BELLY, HORN, OUTLINE, FIRE, BLUSH] { assert_eq!(coat.recolor(keep), keep, "{coat:?} {keep:?}") }
    }
}

#[test]
fn a_red_dragon_has_red_scales_and_wings() {
    for c in [SCALE, WING] {
        let [r, g, b] = Coat::Red.recolor(c);
        assert!(r > g && r > b, "{c:?} → {:?}", [r, g, b]);
    }
    let [r, g, b] = Coat::Pink.recolor(SCALE);
    assert!(r > g && b > g, "pink: {:?}", [r, g, b]);
}

#[test]
fn shading_survives_a_recolour() {
    let (dark, light) = (Coat::Blue.recolor([118, 92, 192]), Coat::Blue.recolor(SCALE));
    let lum = |c: [u8; 3]| c.iter().map(|&v| v as u32).sum::<u32>();
    assert!(lum(dark) < lum(light), "the darker scale stays darker");
}

#[test]
fn coats_save_as_lowercase_names() {
    assert_eq!(serde_json::to_string(&Coat::Pink).unwrap(), "\"pink\"");
    assert_eq!(serde_json::from_str::<Coat>("\"shadow\"").unwrap(), Coat::Shadow);
}
