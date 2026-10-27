//! Help with the working day, from the calendar: the meeting link to join, the morning's agenda, free time
//! worth a focus session, meetings that run straight into each other or over time, being late to one,
//! walking over to one in person, and a wrap-up at the end of the day and of the week.

use crate::domain::CalendarEvent;
use super::alarms::clock_label;

/// A free stretch at least this long (seconds) before the next meeting is worth offering focus for.
pub const FREE_GAP: f64 = 50.0 * 60.0;
/// Focus offered in free time ends this long (seconds) before the next meeting.
pub const FOCUS_MARGIN: f64 = 5.0 * 60.0;
/// The longest focus session offered (seconds).
pub const LONGEST_FOCUS: f64 = 90.0 * 60.0;
/// Meetings this close together (seconds) leave no time for a break in between.
pub const BACK_TO_BACK: f64 = 5.0 * 60.0;
/// The wrap-up times in the tray menu (local hours).
pub const WRAP_UP_AT: [u32; 3] = [17, 18, 19];

/// A meeting this long under way (seconds) without the user in its call counts as being late to it…
pub const LATE_AFTER: f64 = 2.0 * 60.0;
/// …until this long in, by when they've likely skipped it.
pub const LATE_UNTIL: f64 = 10.0 * 60.0;
/// A call still going this long (seconds) after its meeting's end counts as running over…
pub const OVER_AFTER: f64 = 3.0 * 60.0;
/// …and is let be after this long.
pub const OVER_UNTIL: f64 = 30.0 * 60.0;

/// Locations that stand for a call rather than a place to walk to.
const NOT_PLACES: [&str; 8] = ["zoom", "google meet", "microsoft teams", "teams meeting", "webex", "online", "virtual", "phone"];

/// Video call services whose links are recognised in an event.
const CALLS: [&str; 7] = ["zoom.us/", "meet.google.com/", "teams.microsoft.com/", "teams.live.com/", "webex.com/", "whereby.com/", "chime.aws/"];

/// The first video call link in an event's URL, location or notes.
pub fn meeting_link(texts: &[&str]) -> Option<String> {
    texts.iter().flat_map(|t| t.split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '(' | ')' | '[' | ']'))).find_map(|word| {
        let start = word.find("https://")?;
        let url = word[start..].trim_end_matches(['.', ',', ';', ':', '!', '?']);
        let host_path = &url["https://".len()..];
        let host_end = host_path.find('/').map_or(host_path.len(), |i| i + 1);
        let host = &host_path[..host_end];
        CALLS.iter().any(|c| host == *c || host.ends_with(&format!(".{c}"))).then(|| url.to_string())
    })
}

/// Where an event's location says it is, if that's a place to go to rather than a call.
pub fn meeting_place(location: &str) -> Option<String> {
    let l = location.trim();
    let lower = l.to_lowercase();
    let online = lower.contains("://") || NOT_PLACES.iter().any(|n| lower.contains(n));
    (!l.is_empty() && !online).then(|| if l.chars().count() > 30 { l.chars().take(29).collect::<String>() + "…" } else { l.to_string() })
}

/// The heads-up for a meeting in person: where it is, and at the 5-minute mark, that it's time to go.
pub fn walk_over_text(heads_up: &str, place: &str, minutes: u32) -> String {
    if (2..=5).contains(&minutes) { format!("{heads_up}, {place}. Time to walk over 🚶") } else { format!("{heads_up} ({place})") }
}

/// A meeting with a call that's been under way a couple of minutes without the user in a call since it began.
/// `last_call` is when a call was last seen (seconds since the epoch).
pub fn running_late(events: &[CalendarEvent], now: f64, last_call: f64) -> Option<&CalendarEvent> {
    events.iter().find(|e| e.link.is_some() && now - e.start >= LATE_AFTER && now - e.start < LATE_UNTIL && last_call < e.start && now < e.end)
}

pub fn late_text(e: &CalendarEvent, now: f64) -> String {
    let m = ((now - e.start) / 60.0).floor().max(1.0) as u32;
    format!("⏰ {} started {m} min ago! Click me to join 🔗", short(&e.title))
}

/// While in a call: the meeting it's for, if that's past its end, and the meeting that's due next, if any.
pub fn running_over(events: &[CalendarEvent], now: f64) -> Option<(&CalendarEvent, Option<&CalendarEvent>)> {
    let over = events.iter().filter(|e| e.end > e.start && now - e.end >= OVER_AFTER && now - e.end < OVER_UNTIL).max_by(|a, b| a.end.total_cmp(&b.end))?;
    let next = events.iter().filter(|o| o.id != over.id && o.start >= over.end - 60.0 && o.start <= now + 120.0).min_by(|a, b| a.start.total_cmp(&b.start));
    Some((over, next))
}

pub fn over_text(over: &CalendarEvent, next: Option<&CalendarEvent>, now: f64) -> String {
    let m = ((now - over.end) / 60.0).round().max(1.0) as u32;
    match next {
        Some(n) if n.link.is_some() => format!("⏰ {} is running {m} min over. {} has started, click me to join 🔗", short(&over.title), short(&n.title)),
        Some(n) => format!("⏰ {} is running {m} min over. {} has started!", short(&over.title), short(&n.title)),
        None => format!("⏰ {} is running {m} min over", short(&over.title)),
    }
}

/// The meeting that starts as soon as `e` ends (within `BACK_TO_BACK`), if there is one.
pub fn back_to_back<'a>(events: &'a [CalendarEvent], e: &CalendarEvent) -> Option<&'a CalendarEvent> {
    if e.end <= e.start { return None }
    events.iter().filter(|o| o.id != e.id && o.start >= e.end - 60.0 && o.start <= e.end + BACK_TO_BACK).min_by(|a, b| a.start.total_cmp(&b.start))
}

/// When the user is free now and the next meeting is a good while off: that meeting.
pub fn free_gap(events: &[CalendarEvent], now: f64) -> Option<&CalendarEvent> {
    if events.iter().any(|e| e.start <= now && now < e.end) { return None }
    let next = events.iter().filter(|e| e.start > now).min_by(|a, b| a.start.total_cmp(&b.start))?;
    (next.start - now >= FREE_GAP).then_some(next)
}

/// When focus offered at `now` should end: a little before the next meeting, and not too long.
pub fn focus_end(next: &CalendarEvent, now: f64) -> f64 { (next.start - FOCUS_MARGIN).min(now + LONGEST_FOCUS) }

/// The heads-up offering focus before `next`.
pub fn focus_offer_text(next: &CalendarEvent, now: f64, hour: f64) -> String {
    let until = clock_label(focus_end(next, now), now, hour);
    format!("🎯 You're free until {} at {}. Click me to focus until {until}", short(&next.title), clock_label(next.start, now, hour))
}

/// The morning's agenda: how many meetings are left today and which one is first, and the to-dos waiting.
pub fn agenda_text(events: &[CalendarEvent], now: f64, hour: f64, todos: usize) -> String {
    let midnight = now + (24.0 - hour) * 3600.0;
    let mut today: Vec<&CalendarEvent> = events.iter().filter(|e| e.start >= now && e.start < midnight).collect();
    today.sort_by(|a, b| a.start.total_cmp(&b.start));
    let meetings = match today.as_slice() {
        [] => "☀️ Good morning! No meetings today, a great day to focus 🎯".into(),
        [one] => format!("☀️ Good morning! One meeting today: {} at {}", short(&one.title), clock_label(one.start, now, hour)),
        [first, ..] => format!("☀️ Good morning! {} meetings today. First up: {} at {}", today.len(), short(&first.title), clock_label(first.start, now, hour)),
    };
    match todos {
        0 => meetings,
        1 => format!("{meetings}. And 1 to-do on your list 📝"),
        n => format!("{meetings}. And {n} to-dos on your list 📝"),
    }
}

/// What got done in a day, for the wrap-ups.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DayStats {
    pub meetings: usize,
    pub focus_sessions: u32,
    pub focus_minutes: f64,
    pub breaks: u32,
    pub todos_done: u32,
}

impl DayStats {
    fn add(mut self, o: &DayStats) -> DayStats {
        self.meetings += o.meetings;
        self.focus_sessions += o.focus_sessions;
        self.focus_minutes += o.focus_minutes;
        self.breaks += o.breaks;
        self.todos_done += o.todos_done;
        self
    }

    /// "3 meetings, 2 focus sessions (1h 15m), …", leaving out what didn't happen; `sessions` names focus sessions.
    fn parts(&self, sessions: bool) -> Vec<String> {
        let count = |n: usize, one: &str, many: &str| match n { 0 => None, 1 => Some(format!("1 {one}")), n => Some(format!("{n} {many}")) };
        let (h, m) = ((self.focus_minutes / 60.0) as u32, (self.focus_minutes % 60.0).round() as u32);
        let time = if h > 0 { format!("{h}h {m}m") } else { format!("{m} min") };
        let focus = match (self.focus_sessions, sessions) {
            (0, _) => None,
            (n, true) => Some(format!("{} ({time})", count(n as usize, "focus session", "focus sessions").unwrap_or_default())),
            (_, false) => Some(format!("{time} of focus")),
        };
        [count(self.meetings, "meeting", "meetings"), focus, count(self.breaks as usize, "break", "breaks"), count(self.todos_done as usize, "to-do done", "to-dos done")]
            .into_iter()
            .flatten()
            .collect()
    }
}

/// The end-of-day wrap-up, with what got done today and the to-dos still open.
pub fn wrap_up_text(stats: &DayStats, todos_left: usize) -> String {
    let done = stats.parts(true);
    let mut text = if done.is_empty() { "🌇 Time to wrap up for today!".to_string() } else { format!("🌇 Time to wrap up! Today: {}.", done.join(", ")) };
    match todos_left {
        0 => text += " See you tomorrow 🐾",
        1 => text += " 1 to-do left for tomorrow 📝",
        n => text += &format!(" {n} to-dos left for tomorrow 📝"),
    }
    text
}

/// The Friday wrap-up: the whole week's tally.
pub fn week_text(days: &[DayStats]) -> String {
    let week = days.iter().fold(DayStats::default(), DayStats::add);
    let done = week.parts(false);
    if done.is_empty() { return "📊 That's the week! Have a great weekend 🐾".into() }
    format!("📊 This week: {}. Have a great weekend 🐾", done.join(", "))
}

/// Whether `hour` is in the few hours after bedtime, when a pet nudges a user still working.
pub fn past_bedtime(hour: f64, bedtime: Option<u32>) -> bool {
    bedtime.is_some_and(|b| (hour - b as f64).rem_euclid(24.0) < 3.0)
}

/// An event title cut down to fit a bubble.
fn short(title: &str) -> String {
    let t = title.trim();
    let t = if t.is_empty() { "your next event" } else { t };
    if t.chars().count() > 30 { t.chars().take(29).collect::<String>() + "…" } else { t.to_string() }
}

#[cfg(test)]
#[path = "workday_tests.rs"]
mod tests;
