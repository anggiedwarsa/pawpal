//! Help with the working day, from the calendar: the meeting link to join, the morning's agenda, free time
//! worth a focus session, meetings that run straight into each other, and a wrap-up at the end of the day.

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

/// The morning's agenda: how many meetings are left today and which one is first.
pub fn agenda_text(events: &[CalendarEvent], now: f64, hour: f64) -> String {
    let midnight = now + (24.0 - hour) * 3600.0;
    let mut today: Vec<&CalendarEvent> = events.iter().filter(|e| e.start >= now && e.start < midnight).collect();
    today.sort_by(|a, b| a.start.total_cmp(&b.start));
    match today.as_slice() {
        [] => "☀️ Good morning! No meetings today, a great day to focus 🎯".into(),
        [one] => format!("☀️ Good morning! One meeting today: {} at {}", short(&one.title), clock_label(one.start, now, hour)),
        [first, ..] => format!("☀️ Good morning! {} meetings today. First up: {} at {}", today.len(), short(&first.title), clock_label(first.start, now, hour)),
    }
}

/// What got done today, for the wrap-up.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DayStats {
    pub meetings: usize,
    pub focus_sessions: u32,
    pub focus_minutes: f64,
}

/// The end-of-day wrap-up, with what got done today.
pub fn wrap_up_text(stats: &DayStats) -> String {
    let mut done = Vec::new();
    match stats.meetings {
        0 => {}
        1 => done.push("1 meeting".to_string()),
        n => done.push(format!("{n} meetings")),
    }
    if stats.focus_sessions > 0 {
        let (h, m) = ((stats.focus_minutes / 60.0) as u32, (stats.focus_minutes % 60.0).round() as u32);
        let time = if h > 0 { format!("{h}h {m}m") } else { format!("{m} min") };
        let sessions = if stats.focus_sessions == 1 { "1 focus session".to_string() } else { format!("{} focus sessions", stats.focus_sessions) };
        done.push(format!("{sessions} ({time})"));
    }
    if done.is_empty() { return "🌇 Time to wrap up for today! See you tomorrow 🐾".into() }
    format!("🌇 Time to wrap up! Today: {}. Great work 🐾", done.join(", "))
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
