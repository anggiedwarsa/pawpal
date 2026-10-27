//! Unit tests for engine/workday.rs.

use super::*;

fn ev(id: &str, title: &str, start: f64, end: f64) -> CalendarEvent {
    CalendarEvent { id: id.into(), title: title.into(), start, end, link: None, place: None }
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
    assert_eq!(agenda_text(&today, now, hour, 0), "☀️ Good morning! 2 meetings today. First up: Standup at 10:00 AM");
    assert_eq!(agenda_text(&today[..1], now, hour, 0), "☀️ Good morning! One meeting today: Design at 1:00 PM");
    assert_eq!(agenda_text(&today[2..], now, hour, 0), "☀️ Good morning! No meetings today, a great day to focus 🎯", "tomorrow's don't count");
}

#[test]
fn the_wrap_up_sums_up_the_day() {
    assert_eq!(wrap_up_text(&DayStats::default(), 0), "🌇 Time to wrap up for today! See you tomorrow 🐾");
    let day = DayStats { meetings: 3, focus_sessions: 2, focus_minutes: 75.0, breaks: 4, todos_done: 1 };
    assert_eq!(wrap_up_text(&day, 2), "🌇 Time to wrap up! Today: 3 meetings, 2 focus sessions (1h 15m), 4 breaks, 1 to-do done. 2 to-dos left for tomorrow 📝");
    let day = DayStats { meetings: 1, focus_sessions: 1, focus_minutes: 25.0, ..Default::default() };
    assert_eq!(wrap_up_text(&day, 1), "🌇 Time to wrap up! Today: 1 meeting, 1 focus session (25 min). 1 to-do left for tomorrow 📝");
}

#[test]
fn the_week_adds_up_every_day() {
    let mon = DayStats { meetings: 5, focus_sessions: 2, focus_minutes: 100.0, breaks: 6, todos_done: 3 };
    let tue = DayStats { meetings: 4, focus_sessions: 1, focus_minutes: 50.0, breaks: 5, todos_done: 0 };
    assert_eq!(week_text(&[mon, tue]), "📊 This week: 9 meetings, 2h 30m of focus, 11 breaks, 3 to-dos done. Have a great weekend 🐾");
    assert_eq!(week_text(&[]), "📊 That's the week! Have a great weekend 🐾");
}

#[test]
fn the_agenda_mentions_waiting_to_dos() {
    assert_eq!(agenda_text(&[], 0.0, 8.0, 3), "☀️ Good morning! No meetings today, a great day to focus 🎯. And 3 to-dos on your list 📝");
    assert!(agenda_text(&[], 0.0, 8.0, 1).ends_with("And 1 to-do on your list 📝"));
}

#[test]
fn in_person_meetings_say_where_and_when_to_go() {
    assert_eq!(meeting_place(" Room 4B "), Some("Room 4B".into()));
    assert_eq!(meeting_place("Microsoft Teams Meeting"), None);
    assert_eq!(meeting_place("https://zoom.us/j/1"), None);
    assert_eq!(meeting_place("Online"), None);
    assert_eq!(meeting_place(""), None);
    assert_eq!(walk_over_text("📅 Sync in 5 minutes", "Room 4B", 5), "📅 Sync in 5 minutes, Room 4B. Time to walk over 🚶");
    assert_eq!(walk_over_text("📅 Sync in 10 minutes", "Room 4B", 10), "📅 Sync in 10 minutes (Room 4B)");
    assert_eq!(walk_over_text("📅 Sync starts in 1 minute!", "Room 4B", 1), "📅 Sync starts in 1 minute! (Room 4B)");
}

#[test]
fn late_means_a_call_meeting_under_way_without_joining() {
    let mut standup = ev("s", "Standup", 0.0, 900.0);
    standup.link = Some("https://meet.google.com/x".into());
    let events = [standup.clone()];
    assert_eq!(running_late(&events, 60.0, -1.0), None, "a minute in is still on time");
    assert_eq!(running_late(&events, 180.0, -1.0).map(|e| e.id.as_str()), Some("s"));
    assert_eq!(running_late(&events, 180.0, 30.0), None, "already in the call");
    assert_eq!(running_late(&events, 700.0, -1.0), None, "too late to bother");
    let in_person = [ev("r", "Review", 0.0, 900.0)];
    assert_eq!(running_late(&in_person, 180.0, -1.0), None, "no call to tell from");
    assert_eq!(late_text(&standup, 190.0), "⏰ Standup started 3 min ago! Click me to join 🔗");
}

#[test]
fn over_time_names_the_meeting_and_the_one_waiting() {
    let mut next = ev("n", "Design", 1800.0, 3600.0);
    let events = [ev("a", "Standup", 0.0, 1800.0), next.clone()];
    assert_eq!(running_over(&events, 1800.0 + 60.0), None, "a minute over is nothing");
    let (over, waiting) = running_over(&events, 1800.0 + 300.0).unwrap();
    assert_eq!((over.id.as_str(), waiting.map(|e| e.id.as_str())), ("a", Some("n")));
    assert_eq!(over_text(over, waiting, 2100.0), "⏰ Standup is running 5 min over. Design has started!");
    next.link = Some("https://zoom.us/j/1".into());
    assert_eq!(over_text(over, Some(&next), 2100.0), "⏰ Standup is running 5 min over. Design has started, click me to join 🔗");
    assert_eq!(over_text(over, None, 2100.0), "⏰ Standup is running 5 min over");
    assert_eq!(running_over(&events[..1], 1800.0 + 3600.0), None, "long past it");
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
