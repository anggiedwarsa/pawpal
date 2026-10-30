use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::support::pick;

/// What a kind of pet says, from the `lines` in its skin's `pet.json`. Each key holds one line or a
/// list to pick from; any key left out is said the way the pet's base species says it.
///
/// Keys: `sounds`, `hungry`, `petted`, `knead`, `hello`, `taunt`, `picked_up`, `yawn`, `yum`,
/// `treat`, `cursor`, `found_you`, `nice_spot`, `startled`, `cuddle`, `too_sleepy`, `not_now`,
/// `trick_done`, `tag`, `tag_over`, `fight_over`, and one per special skill: `transform`,
/// `breathe_fire`, `fly`, `grow`, `herd`, `vanish`, `stomp`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Voice(BTreeMap<String, Lines>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Lines {
    One(String),
    Many(Vec<String>),
}

impl Voice {
    pub fn is_empty(&self) -> bool { self.0.is_empty() }

    /// One of the pet's own lines for `key`, or None if it has none (blank lines don't count).
    pub fn get(&self, key: &str) -> Option<&str> { self.0.get(key)?.pick() }
}

impl Default for Lines {
    fn default() -> Lines { Lines::Many(Vec::new()) }
}

impl Lines {
    /// One of the lines at random, or None if there are none (blank lines don't count).
    pub fn pick(&self) -> Option<&str> {
        let lines: Vec<&str> = match self {
            Lines::One(s) => vec![s.as_str()],
            Lines::Many(v) => v.iter().map(String::as_str).collect(),
        };
        let lines: Vec<&str> = lines.into_iter().filter(|s| !s.trim().is_empty()).collect();
        (!lines.is_empty()).then(|| pick(&lines))
    }
}

#[cfg(test)]
#[path = "voice_tests.rs"]
mod tests;
