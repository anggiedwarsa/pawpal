//! Unit tests for domain/coat.rs.

use super::*;

const ART: Recolor = Recolor::LAVENDER;

const SCALE: [u8; 3] = [152, 124, 222];
const WING: [u8; 3] = [196, 170, 246];
const BELLY: [u8; 3] = [255, 222, 140];
const HORN: [u8; 3] = [255, 244, 212];
const OUTLINE: [u8; 3] = [36, 26, 48];
const FIRE: [u8; 3] = [255, 122, 38];
const BLUSH: [u8; 3] = [250, 150, 170];

#[test]
fn the_natural_coat_leaves_the_art_alone() {
    for c in [SCALE, WING, BELLY, OUTLINE, FIRE] { assert_eq!(Coat::Natural.recolor(c, ART), c) }
}

#[test]
fn only_the_scales_change_colour() {
    for coat in Coat::ALL {
        for keep in [BELLY, HORN, OUTLINE, FIRE, BLUSH] { assert_eq!(coat.recolor(keep, ART), keep, "{coat:?} {keep:?}") }
    }
}

#[test]
fn a_red_dragon_has_red_scales_and_wings() {
    for c in [SCALE, WING] {
        let [r, g, b] = Coat::Red.recolor(c, ART);
        assert!(r > g && r > b, "{c:?} → {:?}", [r, g, b]);
    }
    let [r, g, b] = Coat::Pink.recolor(SCALE, ART);
    assert!(r > g && b > g, "pink: {:?}", [r, g, b]);
}

#[test]
fn shading_survives_a_recolour() {
    let (dark, light) = (Coat::Blue.recolor([118, 92, 192], ART), Coat::Blue.recolor(SCALE, ART));
    let lum = |c: [u8; 3]| c.iter().map(|&v| v as u32).sum::<u32>();
    assert!(lum(dark) < lum(light), "the darker scale stays darker");
}

#[test]
fn coats_save_as_lowercase_names() {
    assert_eq!(serde_json::to_string(&Coat::Pink).unwrap(), "\"pink\"");
    assert_eq!(serde_json::from_str::<Coat>("\"shadow\"").unwrap(), Coat::Shadow);
}

#[test]
fn any_art_hue_can_be_recoloured() {
    // A teal fox: only the teal changes, the white chest stays white.
    let teal = Recolor { hue: 180.0, band: 20.0 };
    let [r, g, b] = Coat::Red.recolor([40, 190, 190], teal);
    assert!(r > g && r > b, "teal turns red: {:?}", [r, g, b]);
    assert_eq!(Coat::Red.recolor([245, 245, 245], teal), [245, 245, 245]);
    assert_eq!(Coat::Red.recolor(SCALE, teal), SCALE, "lavender is outside a teal band");
}

#[test]
fn the_band_is_optional_in_pet_files() {
    let r: Recolor = serde_json::from_str(r#"{ "hue": 30 }"#).unwrap();
    assert_eq!((r.hue, r.band), (30.0, Recolor::LAVENDER.band));
}
