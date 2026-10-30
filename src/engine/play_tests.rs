//! Unit tests for engine/play.rs.

use super::*;
use crate::domain::Species;
use crate::testing::{skin, world};

/// Steps every pet, the way the world does each frame.
fn run_all(w: &mut World, seconds: f64) {
    for _ in 0..(seconds * 30.0) as usize {
        for i in 0..w.pets.len() { w.with_pet(i, |w, p| w.update(p, 1.0 / 30.0)) }
    }
}

fn emitted(w: &World, text: &str) -> bool { count(w, text) > 0 }

fn count(w: &World, text: &str) -> usize { w.particles.iter().filter(|q| q.text == text).count() }

#[test]
fn tag_turns_both_pets_into_players_and_swaps_who_is_it() {
    let mut w = world(2);
    w.pets[1].x = 300.0;
    w.with_pet(0, |w, p| w.start(p, PlayFriend, true));
    run_all(&mut w, 1.5);
    let (a, b) = (&w.pets[0], &w.pets[1]);
    assert_eq!((a.activity, b.activity), (PlayFriend, PlayFriend), "the friend joined in");
    assert_eq!((a.buddy, b.buddy), (Some(b.id), Some(a.id)));
    assert_eq!((a.step, b.step), (RUN_AWAY, CHASE), "the one who tagged runs; the friend is it");

    let mut swapped = false;
    for _ in 0..(10.0 * 30.0) as usize {
        run_all(&mut w, 1.0 / 30.0);
        if w.pets[0].step == CHASE { swapped = true; break }
    }
    assert!(swapped, "whoever's it catches up and passes it on");
}

#[test]
fn a_game_ends_for_both_with_a_happy_word() {
    let mut w = world(2);
    w.pets[1].x = 300.0;
    w.with_pet(0, |w, p| w.start(p, PlayFriend, true));
    run_all(&mut w, GAME_TIME + 0.5);
    // (Free will may well pick another game straight away, so check the making-up, not what's next.)
    assert_eq!(count(&w, "💖"), 2, "both make up");
}

#[test]
fn play_fights_pounce_tumble_and_make_up() {
    let mut w = world(2);
    w.pets[1].x = 320.0;
    w.with_pet(0, |w, p| w.start(p, PlayFight, true));
    run_all(&mut w, 1.0);
    assert_eq!((w.pets[0].activity, w.pets[1].activity), (PlayFight, PlayFight));
    assert_eq!(w.pets[0].bubble.as_deref(), Some(Species::Cat.taunt()));

    let mut bowled_over = false;
    for _ in 0..(8.0 * 30.0) as usize {
        run_all(&mut w, 1.0 / 30.0);
        bowled_over |= w.pets.iter().any(|p| p.step == BOUNCED);
    }
    assert!(emitted(&w, "💥"), "a pounce lands");
    assert!(bowled_over, "and knocks the other one back");

    run_all(&mut w, GAME_TIME - 8.5);
    assert_eq!(count(&w, "💖"), 2, "it ends as friends");
}

#[test]
fn a_game_stops_when_the_friend_is_called_away() {
    let mut w = world(2);
    w.pets[1].x = 300.0;
    w.with_pet(0, |w, p| w.start(p, PlayFight, true));
    run_all(&mut w, 1.0);
    w.with_pet(1, |w, p| w.start(p, Dragged, true));
    run_all(&mut w, 0.2);
    assert!(w.pets[0].buddy.is_none(), "it leaves the game");
}

#[test]
fn nobody_to_play_with_means_no_game() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.start(p, PlayFight, true));
    run_all(&mut w, 0.2);
    assert_ne!(w.pets[0].activity, PlayFight);
}

#[test]
fn a_kitsune_powers_up_then_shapeshifts_into_a_friend_and_back() {
    let mut w = world(2);
    let friend_look = w.pets[1].skin.clone().unwrap();
    w.pets[0].species = Species::Fox;
    w.pets[0].skin = Some(skin(&["walk", "idle", "transform"]));
    w.with_pet(0, |w, p| w.start(p, Transform, true));
    run_all(&mut w, CHARGE_TIME - 0.3);
    assert!(w.pets[0].disguise.is_none(), "still powering up");
    assert!(emitted(&w, "🔥") || emitted(&w, "✨"));

    run_all(&mut w, 0.5);
    let look = w.pets[0].disguise.clone().expect("shapeshifted");
    assert!(Rc::ptr_eq(&look, &friend_look), "it looks like its friend now");
    assert!(emitted(&w, "💨"), "in a puff of smoke");

    w.pets[0].disguise_ttl = 0.1;
    run_all(&mut w, 0.3);
    assert!(w.pets[0].disguise.is_none(), "the spell wears off");
    assert!(w.pets[0].bubble.as_deref().is_some_and(|b| b.contains("🦊")));
}

#[test]
fn petting_a_disguised_kitsune_breaks_the_spell() {
    let mut w = world(2);
    w.pets[0].species = Species::Fox;
    w.pets[0].disguise = w.pets[1].skin.clone();
    w.pets[0].disguise_ttl = 30.0;
    w.with_pet(0, |w, p| w.start(p, Petted, true));
    assert!(w.pets[0].disguise.is_none());
    assert_eq!(w.pets[0].bubble.as_deref(), Some("Hehe, you found me! 🦊"));
}

#[test]
fn only_pets_with_a_transform_clip_can_transform() {
    let w = world(1);
    assert!(!w.pets[0].supports(Transform));
    let mut p = Pet::new(9, "Kyubi".into(), Species::Fox, Some(skin(&["walk", "transform"])));
    assert!(p.supports(Transform));
    p.skin = Some(skin(&["walk"]));
    assert!(!p.supports(Transform));
}

#[test]
fn a_dragon_breathes_in_then_breathes_fire_and_startles_pets_in_front() {
    let mut w = world(3);
    w.pets[0].species = Species::Dragon;
    w.pets[0].skin = Some(skin(&["walk", "idle", "firebreath"]));
    (w.pets[0].x, w.pets[0].facing) = (200.0, 1.0);
    (w.pets[1].x, w.pets[2].x) = (300.0, 100.0);
    w.with_pet(0, |w, p| w.start(p, BreatheFire, true));
    run_all(&mut w, INHALE_TIME - 0.2);
    assert!(!emitted(&w, "🔥"), "first a deep breath");
    run_all(&mut w, 0.5);
    assert!(count(&w, "🔥") >= 3, "then flames");
    assert!(w.pets[1].bubble.is_some() && w.pets[1].y > 0.0, "the pet in front jumps");
    assert!(w.pets[2].bubble.is_none(), "the one behind is fine");
}

#[test]
fn only_pets_with_a_fire_breath_clip_can_breathe_fire() {
    let w = world(1);
    assert!(!w.pets[0].supports(BreatheFire));
    let p = Pet::new(9, "Naga".into(), Species::Dragon, Some(skin(&["walk", "firebreath"])));
    assert!(p.supports(BreatheFire));
}

#[test]
fn a_dragon_takes_off_cruises_without_falling_and_lands_on_its_floor() {
    let mut w = world(1);
    w.pets[0].species = Species::Dragon;
    w.pets[0].skin = Some(skin(&["walk", "idle", "fly"]));
    w.with_pet(0, |w, p| w.start(p, Fly, true));
    let cruise = w.pets[0].cruise;
    assert!(cruise >= 40.0, "it aims well above the Dock");

    run_all(&mut w, 2.0);
    assert_eq!(w.pets[0].step, CRUISE, "up at cruising height");
    let (y0, x0) = (w.pets[0].y, w.pets[0].x);
    assert!(y0 > 30.0, "up in the air: {y0} (aiming for {cruise})");
    let mut moved = 0.0f64;
    for _ in 0..45 {
        run_all(&mut w, 1.0 / 30.0);
        moved = moved.max((w.pets[0].x - x0).abs());
    }
    // (it may glide to a new cruising height, but it doesn't drop: gravity would have it down by now)
    assert!(w.pets[0].y > 30.0, "no gravity on the wing");
    assert!(moved > 20.0, "it flies about");

    w.pets[0].timer = LAND_TIME - 0.1;
    let mut landed = false;
    for _ in 0..(6.0 * 30.0) as usize {
        run_all(&mut w, 1.0 / 30.0);
        // (A dragon may well choose to take off again straight away, so watch for touchdown.)
        if w.pets[0].y == w.pets[0].floor { landed = true; break }
        assert!(w.pets[0].activity == Fly, "the flight lasts until its feet are down");
    }
    assert!(landed, "it comes back down to its floor");
}

#[test]
fn only_pets_with_a_fly_clip_can_fly() {
    let w = world(1);
    assert!(!w.pets[0].supports(Fly));
    let p = Pet::new(9, "Naga".into(), Species::Dragon, Some(skin(&["walk", "fly"])));
    assert!(p.supports(Fly));
}

fn special(w: &mut World, i: usize, species: Species, clip: &str) {
    w.pets[i].species = species;
    w.pets[i].skin = Some(skin(&["walk", "run", "idle", "sit", clip]));
}

#[test]
fn a_snake_grows_into_a_giant_cobra_scares_its_neighbours_and_shrinks_back() {
    let mut w = world(3);
    special(&mut w, 0, Species::Snake, "grow");
    (w.pets[0].x, w.pets[1].x, w.pets[2].x) = (300.0, 450.0, 900.0);
    w.with_pet(0, |w, p| w.start(p, Grow, true));
    run_all(&mut w, 1.5);
    assert_eq!(w.pets[0].bigness, 1.0, "fully grown");
    let scared = |p: &Pet| p.bubble.as_deref().is_some_and(|b| ["A c-cobra!! 😱", "Eeek! 🙀", "So big!!"].contains(&b));
    assert!(scared(&w.pets[1]), "the pet nearby gets a fright");
    assert!(!scared(&w.pets[2]), "the far one doesn't notice");
    // Checked just before the skill ends: free will may pick it again straight after.
    run_all(&mut w, GROW_TIME - 1.5 - 0.1);
    assert_eq!(w.pets[0].activity, Grow);
    assert!(w.pets[0].bigness < 0.2, "shrunk back to (almost) normal size: {}", w.pets[0].bigness);
}

#[test]
fn a_corgi_herds_its_friends_together() {
    let mut w = world(3);
    special(&mut w, 0, Species::Dog, "herd");
    (w.pets[0].x, w.pets[1].x, w.pets[2].x) = (300.0, 650.0, 900.0);
    let before: f64 = (1..3).map(|i| (w.pets[i].x - 300.0).abs()).sum();
    w.with_pet(0, |w, p| w.start(p, Herd, true));
    run_all(&mut w, 1.0 / 30.0);
    assert!(w.pets[1..].iter().all(|f| f.activity == Herd && f.step == FLOCK && f.target_x.is_some()), "the flock heads for the corgi");
    // (Once there, free will may send them off again, so look for the moment they're all in.)
    let mut closest = f64::INFINITY;
    for _ in 0..(8.0 * 30.0) as usize {
        run_all(&mut w, 1.0 / 30.0);
        closest = closest.min((1..3).map(|i| (w.pets[i].x - w.pets[0].rally).abs()).sum());
    }
    assert!(closest < before / 3.0, "they gathered: {before} → {closest}");
}

#[test]
fn a_lone_corgi_has_no_one_to_herd() {
    let mut w = world(1);
    special(&mut w, 0, Species::Dog, "herd");
    w.with_pet(0, |w, p| w.start(p, Herd, true));
    run_all(&mut w, 0.1);
    assert_ne!(w.pets[0].activity, Herd);
}

#[test]
fn a_cat_vanishes_and_reappears_somewhere_else() {
    let mut w = world(1);
    special(&mut w, 0, Species::Cat, "vanish");
    w.pets[0].x = 200.0;
    w.with_pet(0, |w, p| w.start(p, Vanish, true));
    run_all(&mut w, 0.6);
    assert_eq!(w.pets[0].sink, 1.0, "gone");
    assert!(emitted(&w, "✨"));
    run_all(&mut w, 0.8);
    assert!((w.pets[0].x - 200.0).abs() > 150.0, "it moved while out of sight");
    run_all(&mut w, 0.5);
    assert_eq!(w.pets[0].sink, 0.0, "and pops back up");
}

#[test]
fn a_t_rex_stomps_twice_and_shakes_pets_nearby() {
    let mut w = world(3);
    special(&mut w, 0, Species::Trex, "stomp");
    (w.pets[0].x, w.pets[1].x, w.pets[2].x) = (200.0, 500.0, 900.0);
    w.with_pet(0, |w, p| w.start(p, Stomp, true));
    run_all(&mut w, QUAKES[0] + 0.1);
    assert_eq!(count(&w, "💥"), 1, "the first stomp");
    assert!(w.pets[1].y > 0.0, "the ground throws the nearby pet up");
    assert_eq!(w.pets[2].y, 0.0, "too far away to feel it");
    run_all(&mut w, QUAKES[1] - QUAKES[0]);
    assert_eq!(count(&w, "💥"), 2, "and the second");
}

#[test]
fn each_skill_belongs_to_the_pets_with_its_clip() {
    for (clip, a) in [("grow", Grow), ("herd", Herd), ("vanish", Vanish), ("stomp", Stomp)] {
        let w = world(1);
        assert!(!w.pets[0].supports(a), "{clip}");
        let p = Pet::new(9, "X".into(), Species::Cat, Some(skin(&["walk", clip])));
        assert!(p.supports(a), "{clip}");
    }
}

#[test]
fn the_flock_breaks_up_when_the_corgi_is_called_away() {
    let mut w = world(2);
    special(&mut w, 0, Species::Dog, "herd");
    w.pets[1].x = 700.0;
    w.with_pet(0, |w, p| w.start(p, Herd, true));
    run_all(&mut w, 0.5);
    w.with_pet(0, |w, p| w.start(p, Dragged, true));
    run_all(&mut w, 0.2);
    assert_ne!(w.pets[1].step, FLOCK, "no herder, no flock");
}

#[test]
fn a_pet_busy_with_its_skill_is_left_out_of_games() {
    let mut w = world(2);
    special(&mut w, 1, Species::Snake, "grow");
    w.pets[1].x = 300.0;
    w.with_pet(1, |w, p| w.start(p, Grow, true));
    w.with_pet(0, |w, p| w.start(p, PlayFight, true));
    run_all(&mut w, 1.0);
    assert_eq!(w.pets[1].activity, Grow, "the giant cobra isn't dragged into a play fight");
    assert_ne!(w.pets[0].activity, PlayFight);
}

#[test]
fn a_pets_own_skill_plays_for_its_time_with_its_line_burst_and_startle() {
    use crate::testing::{pet_file, skin_with};
    let mut w = world(2);
    w.pets[1].x = w.pets[0].x + 120.0;
    let own = pet_file(r#"{ "skills": [{ "name": "Bubbles", "clip": "bubbles", "seconds": 2, "say": "Blub blub~", "burst": "🫧", "startle": true }] }"#);
    w.pets[0].skin = Some(skin_with(&["walk", "idle", "bubbles"], own));
    w.pets[0].vx = 50.0;
    w.with_pet(0, |w, p| w.start(p, Skill(0), true));
    let p = &w.pets[0];
    assert_eq!((p.activity, p.timer, p.vx, p.bubble.as_deref()), (Skill(0), 2.0, 0.0, Some("Blub blub~")));
    assert_eq!(count(&w, "🫧"), 6);
    assert!(w.pets[1].vy > 0.0 && w.pets[1].bubble.is_some(), "the friend nearby jumps");
    run_all(&mut w, 1.0);
    assert_eq!(w.pets[0].activity, Skill(0));
    run_all(&mut w, 1.5);
    assert_ne!(w.pets[0].activity, Skill(0), "over after its time");
}

#[test]
fn a_skill_that_isnt_there_ends_at_once() {
    let mut w = world(1);
    w.with_pet(0, |w, p| w.begin_skill(p, 3));
    assert_eq!(w.pets[0].timer, 0.0);
    assert!(w.pets[0].bubble.is_none());
}

#[test]
fn a_quiet_skill_just_plays_its_clip() {
    use crate::testing::{pet_file, skin_with};
    let mut w = world(2);
    w.pets[1].x = w.pets[0].x + 60.0;
    w.pets[0].skin = Some(skin_with(&["walk", "bubbles"], pet_file(r#"{ "skills": [{ "name": "B", "clip": "bubbles" }] }"#)));
    w.with_pet(0, |w, p| w.start(p, Skill(0), true));
    assert_eq!((w.pets[0].timer, w.particles.len()), (4.0, 0));
    assert!(w.pets[0].bubble.is_none() && w.pets[1].vy == 0.0);
}
