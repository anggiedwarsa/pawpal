use serde::{Deserialize, Serialize};

/// What kind of animal a pet is; decides what it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Species {
    #[default]
    Cat,
    Dog,
}

impl Species {
    pub fn sounds(self) -> &'static [&'static str] {
        match self {
            Species::Cat => &["Meow!", "Mrrp?", "Nya~", "Mew mew", "Prrrt!"],
            Species::Dog => &["Woof!", "Arf arf!", "Bork!", "Ruff!", "*wags*"],
        }
    }
    pub fn hungry(self) -> &'static [&'static str] {
        match self {
            Species::Cat => &["Feed me, human 🐟", "My bowl is empty…", "Hungry! 😿"],
            Species::Dog => &["Food? FOOD? 🍖", "I'm sooo hungry", "Snack time? 🦴"],
        }
    }
    pub fn petted(self) -> &'static [&'static str] {
        match self {
            Species::Cat => &["Purrrr~", "Prrr ♥", "More pets pls"],
            Species::Dog => &["*happy wiggle*", "Good boy? Me?", "♥ ♥ ♥"],
        }
    }
    pub fn knead(self) -> &'static str { if self == Species::Cat { "*purr purr purr*" } else { "*happy paws*" } }
    pub fn hello(self) -> &'static str { if self == Species::Cat { "Hi there! 👋" } else { "Hiii! 👋" } }

    /// Picks the cat line or the dog line.
    pub fn line(self, cat: &'static str, dog: &'static str) -> &'static str { if self == Species::Cat { cat } else { dog } }
}

#[cfg(test)]
#[path = "species_tests.rs"]
mod tests;
