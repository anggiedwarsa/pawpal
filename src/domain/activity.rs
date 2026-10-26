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
            PlayFriend => "💞 Play With Friend",
            Petted => "🥰 Get Petted",
            Hop => "🪜 Hop Up / Down",
            Hide => "🙈 Hide in the Corner",
            Remind => "📅 Calendar Heads-Up",
            Dragged => "Dragged",
            Falling => "Falling",
        }
    }

    /// Activities the user can pick from a menu.
    pub const MENU: [Activity; 23] = [
        Walk, Sniff, Run, Sit, Lounge, Knead, Sleep, Groom, Stretch, Jump, Pounce, Eat, PlayBall, ChaseButterfly, ChaseCursor,
        VisitCursor, Dance, Wave, Meow, Peekaboo, PlayFriend, Petted, Hop,
    ];

    /// Quiet activities that can be animated at a lower frame rate.
    pub fn is_calm(self) -> bool {
        matches!(self, Idle | Sit | Lounge | Sleep | Groom | Knead | Hide)
    }
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;
