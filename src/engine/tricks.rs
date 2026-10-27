//! Tricks on command, from the pet's menu: the pet shows the trick and is pleased with itself afterwards.
//! A grumpy or sleepy pet may not feel like it.

use super::World;
use crate::domain::Activity::{self, *};
use crate::domain::Pet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trick {
    Sit,
    HighFive,
    Speak,
    Bow,
    Jump,
    Dance,
}

/// Happiness at or below this and the pet won't do tricks.
pub const GRUMPY: f64 = 25.0;
/// Energy at or below this and the pet is too sleepy for tricks.
pub const SLEEPY: f64 = 15.0;
/// How much a finished trick cheers the pet up.
pub const PROUD: f64 = 6.0;

impl Trick {
    pub const ALL: [Trick; 6] = [Trick::Sit, Trick::HighFive, Trick::Speak, Trick::Bow, Trick::Jump, Trick::Dance];

    pub fn title(self) -> &'static str {
        match self {
            Trick::Sit => "🪑 Sit",
            Trick::HighFive => "🖐️ High Five",
            Trick::Speak => "💬 Speak",
            Trick::Bow => "🙇 Bow",
            Trick::Jump => "⬆️ Jump",
            Trick::Dance => "💃 Dance",
        }
    }

    /// The activity that shows it.
    pub fn activity(self) -> Activity {
        match self {
            Trick::Sit => Sit,
            Trick::HighFive => Wave,
            Trick::Speak => Meow,
            // A play bow: front down, tail up, which is what a stretch clip shows.
            Trick::Bow => Stretch,
            Trick::Jump => Jump,
            Trick::Dance => Dance,
        }
    }

    /// What the pet says as it does it; Speak uses the pet's own sounds.
    fn cue(self) -> Option<&'static str> {
        match self {
            Trick::Sit => Some("*sits nicely* 🐾"),
            Trick::HighFive => Some("🖐️ High five!"),
            Trick::Speak => None,
            Trick::Bow => Some("*takes a bow* 🙇"),
            Trick::Jump => Some("Hup! ⬆️"),
            Trick::Dance => Some("💃 Look at me!"),
        }
    }

    /// How long it lasts (seconds).
    fn time(self) -> f64 {
        match self {
            Trick::Sit => 3.5,
            Trick::Dance => 5.0,
            _ => 3.0,
        }
    }
}

impl World {
    /// Asks a pet for a trick. Returns whether it does it.
    pub fn trick(&mut self, p: &mut Pet, t: Trick) -> bool {
        if !p.supports(t.activity()) { return false }
        if p.energy <= SLEEPY {
            p.say(p.species.line("*too sleepy…* 😴", "*yawns* Too tired… 😴"), 2.6);
            return false;
        }
        if p.happiness <= GRUMPY {
            p.say(p.species.line("*ignores you* 😼", "*sighs* Not now… 😔"), 2.6);
            return false;
        }
        self.start(p, t.activity(), true);
        // Down in the corner it hops back up first; that's not the trick.
        if p.activity != t.activity() { return false }
        p.trick = true;
        p.timer = t.time();
        if let Some(cue) = t.cue() { p.say(cue, t.time()) }
        true
    }

    /// A trick done to the end: the pet is proud of itself.
    pub(super) fn trick_done(&mut self, p: &mut Pet) {
        p.trick = false;
        p.happiness = (p.happiness + PROUD).min(100.0);
        self.emit("⭐", self.head_point(p), 14.0);
        p.say(p.species.line("Ta-da! ✨", "Did I do good? ✨"), 2.6);
    }
}

#[cfg(test)]
#[path = "tricks_tests.rs"]
mod tests;
