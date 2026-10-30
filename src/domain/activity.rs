/// Everything a pet can be doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Activity {
    #[default]
    Idle,
    Walk,
    Sniff,
    Run,
    Sit,
    Lounge,
    Knead,
    Sleep,
    Groom,
    Stretch,
    Jump,
    Pounce,
    Eat,
    PlayBall,
    ChaseButterfly,
    ChaseCursor,
    VisitCursor,
    Dance,
    Wave,
    Meow,
    Peekaboo,
    PlayFriend,
    /// Rough-and-tumble play with a friend: pouncing, tussling, then making up.
    PlayFight,
    /// A kitsune's trick: powering up, then shapeshifting into another pet for a while.
    Transform,
    /// A dragon's party trick: a big breath in, then a gout of flame.
    BreatheFire,
    /// A dragon taking to the air: up from its floor, cruising about the screen, then landing again.
    Fly,
    /// A snake rearing up into a giant cobra, hood spread, then shrinking back.
    Grow,
    /// A corgi rounding up the other pets, running back and forth and barking.
    Herd,
    /// A cat sinking out of sight in a sparkle and popping up somewhere else.
    Vanish,
    /// A T-rex stomping hard enough to bounce every pet on its level.
    Stomp,
    Petted,
    Hop,
    /// Peeking from the bottom-right corner while something plays full screen.
    Hide,
    /// Out in the middle of the screen telling the user an event is about to start.
    Remind,
    Dragged,
    Falling,
}

use Activity::*;

impl Activity {
    pub fn title(self) -> &'static str {
        match self {
            Idle => "😐 Idle",
            Walk => "🚶 Walk",
            Sniff => "👃 Sniff Around",
            Run => "💨 Zoomies",
            Sit => "🪑 Sit",
            Lounge => "🍞 Loaf / Lounge",
            Knead => "🍪 Make Biscuits",
            Sleep => "💤 Sleep",
            Groom => "👅 Groom",
            Stretch => "🧘 Stretch",
            Jump => "⬆️ Jump",
            Pounce => "🐾 Pounce",
            Eat => "🍗 Eat",
            PlayBall => "⚽ Play Ball",
            ChaseButterfly => "🦋 Chase Butterfly",
            ChaseCursor => "🖱️ Chase Cursor",
            VisitCursor => "🧗 Visit My Cursor",
            Dance => "💃 Dance",
            Wave => "👋 Wave",
            Meow => "💬 Talk",
            Peekaboo => "🙈 Peekaboo",
            PlayFriend => "💞 Play Tag With Friend",
            PlayFight => "🤼 Play Fight",
            Transform => "🦊 Transform",
            BreatheFire => "🔥 Breathe Fire",
            Fly => "🐉 Fly",
            Grow => "🐍 Grow Giant",
            Herd => "🐕 Herd Friends",
            Vanish => "✨ Vanish",
            Stomp => "🦖 Earthquake Stomp",
            Petted => "🥰 Get Petted",
            Hop => "🪜 Hop Up / Down",
            Hide => "🙈 Hide in the Corner",
            Remind => "📅 Calendar Heads-Up",
            Dragged => "Dragged",
            Falling => "Falling",
        }
    }

    /// Activities the user can pick from a menu.
    pub const MENU: [Activity; 31] = [
        Walk, Sniff, Run, Sit, Lounge, Knead, Sleep, Groom, Stretch, Jump, Pounce, Eat, PlayBall, ChaseButterfly, ChaseCursor,
        VisitCursor, Dance, Wave, Meow, Peekaboo, PlayFriend, PlayFight, Transform, BreatheFire, Fly, Grow, Herd, Vanish, Stomp,
        Petted, Hop,
    ];

    /// Special powers only some kinds of pet have; menus leave them out for the others rather than
    /// suggesting a clip to import.
    pub fn is_special(self) -> bool { matches!(self, Transform | BreatheFire | Fly | Grow | Herd | Vanish | Stomp) }

    /// Quiet activities that can be animated at a lower frame rate.
    pub fn is_calm(self) -> bool {
        matches!(self, Idle | Sit | Lounge | Sleep | Groom | Knead | Hide)
    }
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;
