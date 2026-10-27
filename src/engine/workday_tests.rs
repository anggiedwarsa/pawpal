//! Unit tests for engine/workday.rs.

use super::*;

fn ev(id: &str, title: &str, start: f64, end: f64) -> CalendarEvent {
    CalendarEvent { id: id.into(), title: title.into(), start, end, link: None }
}

#[test]
fn finds_the_video_call_link_wherever_it_is() {
    assert_eq!(meeting_link(&["", "Room 4", "Join: https://us02web.zoom.us/j/123?pwd=abc."]).as_deref(), Some("https://us02web.zoom.us/j/123?pwd=abc"));
    assert_eq!(meeting_link(&["https://meet.google.com/abc-defg-hij"]).as_deref(), Some("https://meet.google.com/abc-defg-hij"));
    assert_eq!(
        meeting_link(&["<https://teams.microsoft.com/l/meetup-join/19%3a>"]).as_deref(),
        Some("https://teams.microsoft.com/l/meetup-join/19%3a")
    );
    assert_eq!(meeting_link(&["Agenda: https://docs.google.com/doc then https://meet.google.com/x"]).as_deref(), Some("https://meet.google.com/x"), "skips other links");
    assert_eq!(meeting_link(&["https://notzoom.us.evil.com/j/1", "https://evilzoom.us/j/1"]), None, "only the real services");
    assert_eq!(meeting_link(&["Lunch at the café"]), None);
}

#[test]
fn spots_a_meeting_that_starts_as_this_one_ends() {
    let events = [ev("a", "Standup", 0.0, 1800.0), ev("b", "Design", 1800.0, 3600.0), ev("c", "Lunch", 7200.0, 9000.0)];
    assert_eq!(back_to_back(&events, &events[0]).map(|e| e.id.as_str()), Some("b"));
    assert_eq!(back_to_back(&events, &events[1]), None, "an hour's break before lunch");
    let gap = [ev("a", "A", 0.0, 1800.0), ev("b", "B", 1800.0 + 240.0, 3600.0)];
    assert!(back_to_back(&gap, &gap[0]).is_some(), "four minutes isn't a break");
}

#[test]
fn offers_focus_only_when_free_with_time_to_spare() {
    let now = 1000.0;
    let soon = [ev("a", "Sync", now + 1800.0, now + 3600.0)];
    assert_eq!(free_gap(&soon, now), None, "half an hour isn't enough");
    let later = [ev("a", "Sync", now + 3.0 * 3600.0, now + 4.0 * 3600.0)];
    assert_eq!(free_gap(&later, now).map(|e| e.id.as_str()), Some("a"));
    assert_eq!(focus_end(&later[0], now), now + LONGEST_FOCUS, "a session at most 90 minutes long");
    let close = [ev("a", "Sync", now + 3600.0, now + 7200.0)];
    assert_eq!(focus_end(&close[0], now), now + 3600.0 - FOCUS_MARGIN, "ends 5 minutes before the meeting");
    let busy = [ev("now", "Call", now - 60.0, now + 600.0), ev("a", "Sync", now + 3.0 * 3600.0, now + 4.0 * 3600.0)];
    assert_eq!(free_gap(&busy, now), None, "not while in a meeting");
    assert_eq!(free_gap(&[], now), None, "nothing to fit it before");
}

#[test]
fn the_focus_offer_says_until_when() {
    let next = ev("a", "Design Review", 3.0 * 3600.0, 4.0 * 3600.0);
    assert_eq!(focus_offer_text(&next, 0.0, 9.0), "🎯 You're free until Design Review at 12:00 PM. Click me to focus until 10:30 AM");
}

#[test]
fn the_agenda_counts_todays_meetings_and_names_the_first() {
    let (now, hour) = (0.0, 8.0);
    let today = [ev("b", "Design", 5.0 * 3600.0, 6.0 * 3600.0), ev("a", "Standup", 2.0 * 3600.0, 2.25 * 3600.0), ev("t", "Tomorrow", 20.0 * 3600.0, 21.0 * 3600.0)];
    assert_eq!(agenda_text(&today, now, hour), "☀️ Good morning! 2 meetings today. First up: Standup at 10:00 AM");
    assert_eq!(agenda_text(&today[..1], now, hour), "☀️ Good morning! One meeting today: Design at 1:00 PM");
    assert_eq!(agenda_text(&today[2..], now, hour), "☀️ Good morning! No meetings today, a great day to focus 🎯", "tomorrow's don't count");
}

#[test]
fn the_wrap_up_sums_up_the_day() {
    assert_eq!(wrap_up_text(&DayStats::default()), "🌇 Time to wrap up for today! See you tomorrow 🐾");
    let day = DayStats { meetings: 3, focus_sessions: 2, focus_minutes: 75.0 };
    assert_eq!(wrap_up_text(&day), "🌇 Time to wrap up! Today: 3 meetings, 2 focus sessions (1h 15m). Great work 🐾");
    let day = DayStats { meetings: 1, focus_sessions: 1, focus_minutes: 25.0 };
    assert_eq!(wrap_up_text(&day), "🌇 Time to wrap up! Today: 1 meeting, 1 focus session (25 min). Great work 🐾");
}

#[test]
fn late_night_is_the_few_hours_after_bedtime() {
    assert!(past_bedtime(22.5, Some(22)));
    assert!(past_bedtime(0.5, Some(23)), "across midnight");
    assert!(!past_bedtime(21.0, Some(22)));
    assert!(!past_bedtime(3.0, Some(22)), "by then it's early morning");
    assert!(!past_bedtime(23.0, None));
}

#[test]
fn long_titles_are_shortened() {
    assert_eq!(short(&"x".repeat(40)).chars().count(), 30);
    assert_eq!(short("  "), "your next event");
}
