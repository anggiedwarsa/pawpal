use serde::{Deserialize, Serialize};

/// What kind of animal a pet is; decides what it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Species {
    #[default]
    Cat,
    Dog,
    Snake,
    Fox,
    Trex,
    Dragon,
}

impl Species {
    /// Reads a species name such as "snake" or "T-Rex" (any case; spaces and dashes ignored).
    pub fn parse(name: &str) -> Option<Species> {
        let name: String = name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
        serde_json::from_value(serde_json::Value::String(name)).ok()
    }

    pub fn sounds(self) -> &'static [&'static str] {
        match self {
            Species::Cat => &["Meow!", "Mrrp?", "Nya~", "Mew mew", "Prrrt!"],
            Species::Dog => &["Woof!", "Arf arf!", "Bork!", "Ruff!", "*wags*"],
            Species::Snake => &["Hsss~", "Sss?", "*blep*", "Hiss hiss!", "*tongue flick*"],
            Species::Fox => &["Yip yip!", "Kon kon~", "*fluffs nine tails*", "Ack-ack!", "Heh heh~"],
            Species::Trex => &["RAWR! 🦖", "Rawr~", "*stomp stomp*", "*tiny arm wave*", "Grrr!"],
            Species::Dragon => &["Rawr~ 🐉", "*puff of smoke*", "Grrr~", "*flap flap*", "Hehe~ 🔥"],
        }
    }
    pub fn hungry(self) -> &'static [&'static str] {
        match self {
            Species::Cat => &["Feed me, human 🐟", "My bowl is empty…", "Hungry! 😿"],
            Species::Dog => &["Food? FOOD? 🍖", "I'm sooo hungry", "Snack time? 🦴"],
            Species::Snake => &["Got any mice? 🐭", "Sss… so hungry…", "Snack? 🥚"],
            Species::Fox => &["Fried tofu, please! 🍱", "I'm starving~", "Feed me, mortal 🦊"],
            Species::Trex => &["Need… MEAT 🍖", "*tummy rumbles*", "Snack? Big snack? 🍗"],
            Species::Dragon => &["Got any gold? …or cookies? 🍪", "Hungry dragon! 🍖", "Feed me or I toast~ 🔥"],
        }
    }
    pub fn petted(self) -> &'static [&'static str] {
        match self {
            Species::Cat => &["Purrrr~", "Prrr ♥", "More pets pls"],
            Species::Dog => &["*happy wiggle*", "Good boy? Me?", "♥ ♥ ♥"],
            Species::Snake => &["Sss~ ♥", "*happy coil*", "More scritches"],
            Species::Fox => &["*all nine tails wag*", "Kon~ ♥", "Hehe, again!"],
            Species::Trex => &["*happy stomp*", "Rawr ♥", "I can't hug back… 🥲"],
            Species::Dragon => &["*happy smoke puffs*", "Purr-rawr ♥", "*wings flutter*"],
        }
    }
    pub fn knead(self) -> &'static str {
        match self {
            Species::Cat => "*purr purr purr*",
            Species::Dog => "*happy paws*",
            Species::Snake => "*wiggle wiggle*",
            Species::Fox => "*paw paw paw*",
            Species::Trex => "*stomp stomp stomp*",
            Species::Dragon => "*kneads the gold pile*",
        }
    }
    pub fn hello(self) -> &'static str {
        match self {
            Species::Cat => "Hi there! 👋",
            Species::Dog => "Hiii! 👋",
            Species::Snake => "Hiss-lo! 👋",
            Species::Fox => "Kon-nichiwa! 👋",
            Species::Trex => "Hi! *tiny wave* 👋",
            Species::Dragon => "Hewwo, human! 👋",
        }
    }

    /// What it says squaring up for a play fight.
    pub fn taunt(self) -> &'static str {
        match self {
            Species::Cat => "Hiss! 😼",
            Species::Dog => "Grrr… *wags* 😆",
            Species::Snake => "Hsss! Come here~ 🐍",
            Species::Fox => "Kon! Come at me~ 🔥",
            Species::Trex => "RAWR! Fight me! 🦖",
            Species::Dragon => "Prepare to be toasted~ 🐉",
        }
    }

    /// Picks the cat line or the dog line: snakes and dragons keep a cat's cool, foxes and
    /// T-rexes share a dog's bounce.
    pub fn line<'a>(self, cat: &'a str, dog: &'a str) -> &'a str {
        match self {
            Species::Cat | Species::Snake | Species::Dragon => cat,
            Species::Dog | Species::Fox | Species::Trex => dog,
        }
    }
}

#[cfg(test)]
#[path = "species_tests.rs"]
mod tests;
