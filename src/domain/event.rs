/// An upcoming event from the user's calendar.
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarEvent {
    /// The same on every read: the calendar's id for the event plus its start, so each repeat of a meeting counts.
    pub id: String,
    pub title: String,
    /// Seconds since the Unix epoch.
    pub start: f64,
}
