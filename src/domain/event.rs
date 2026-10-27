/// An upcoming event from the user's calendar.
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarEvent {
    /// The same on every read: the calendar's id for the event plus its start, so each repeat of a meeting counts.
    pub id: String,
    pub title: String,
    /// Seconds since the Unix epoch.
    pub start: f64,
    pub end: f64,
    /// The video call to join (Zoom, Meet, Teams…), when the event has one.
    pub link: Option<String>,
}

/// What clicking a pet that's out with a heads-up does, besides sending it home.
#[derive(Clone, Debug, PartialEq)]
pub enum ClickAction {
    /// Opens a meeting's video call.
    Join(String),
    /// Starts focus mode until then (seconds since the epoch).
    FocusUntil(f64),
}
