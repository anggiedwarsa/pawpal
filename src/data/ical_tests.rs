//! Unit tests for data/ical.rs.

use super::*;

/// Seconds since the epoch of a UTC time.
fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> f64 { Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap().timestamp() as f64 }

const FEED: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\n\
BEGIN:VEVENT\r\nUID:one\r\nDTSTART:20261027T090000Z\r\nDTEND:20261027T093000Z\r\nSUMMARY:Planning\\, Q4\r\n\
LOCATION:https://meet.google.com/abc-defg-hij\r\nBEGIN:VALARM\r\nTRIGGER:-PT10M\r\nDESCRIPTION:Reminder\r\nEND:VALARM\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:paris\r\nDTSTART;TZID=Europe/Paris:20261027T150000\r\nDURATION:PT1H\r\nSUMMARY:Design Review\r\n\
DESCRIPTION:Join Zoom: https://us02web.zoom.us/j/9\r\n 9?pwd=x\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:standup\r\nDTSTART;TZID=Asia/Jakarta:20261019T100000\r\nDTEND;TZID=Asia/Jakarta:20261019T101500\r\n\
RRULE:FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR;UNTIL=20261231T165959Z\r\nEXDATE;TZID=Asia/Jakarta:20261028T100000\r\nSUMMARY:Standup\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:standup\r\nRECURRENCE-ID;TZID=Asia/Jakarta:20261029T100000\r\nDTSTART;TZID=Asia/Jakarta:20261029T110000\r\n\
DTEND;TZID=Asia/Jakarta:20261029T111500\r\nSUMMARY:Standup (moved)\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:holiday\r\nDTSTART;VALUE=DATE:20261027\r\nDTEND;VALUE=DATE:20261028\r\nSUMMARY:Day off\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:gone\r\nDTSTART:20261027T120000Z\r\nDTEND:20261027T130000Z\r\nSTATUS:CANCELLED\r\nSUMMARY:Lunch\r\nEND:VEVENT\r\n\
END:VCALENDAR\r\n";

#[test]
fn reads_timed_events_with_their_zones_and_links() {
    let e = events_between(FEED, at(2026, 10, 27, 0, 0), at(2026, 10, 28, 0, 0));
    let titles: Vec<&str> = e.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Standup", "Planning, Q4", "Design Review"], "all-day and cancelled events are left out");
    let planning = &e[1];
    assert_eq!((planning.start, planning.end), (at(2026, 10, 27, 9, 0), at(2026, 10, 27, 9, 30)));
    assert_eq!(planning.link.as_deref(), Some("https://meet.google.com/abc-defg-hij"));
    let review = &e[2];
    assert_eq!(review.start, at(2026, 10, 27, 14, 0), "15:00 in Paris is 14:00 UTC");
    assert_eq!(review.end - review.start, 3600.0);
    assert_eq!(review.link.as_deref(), Some("https://us02web.zoom.us/j/99?pwd=x"), "folded lines are joined back up");
}

#[test]
fn repeating_meetings_follow_their_rule_skips_and_moves() {
    let e = events_between(FEED, at(2026, 10, 26, 0, 0), at(2026, 11, 2, 0, 0));
    let standups: Vec<(f64, &str)> = e.iter().filter(|e| e.title.starts_with("Standup")).map(|e| (e.start, e.title.as_str())).collect();
    assert_eq!(
        standups,
        [
            (at(2026, 10, 26, 3, 0), "Standup"),
            (at(2026, 10, 27, 3, 0), "Standup"),
            // The 28th is skipped (EXDATE); the 29th's was moved an hour later.
            (at(2026, 10, 29, 4, 0), "Standup (moved)"),
            (at(2026, 10, 30, 3, 0), "Standup"),
        ],
        "weekdays only, 10:00 in Jakarta"
    );
    assert_ne!(e[0].id, e.iter().find(|x| x.start == at(2026, 10, 27, 3, 0)).unwrap().id, "each repeat has its own id");
}

#[test]
fn repeats_stop_at_until() {
    let e = events_between(FEED, at(2027, 1, 4, 0, 0), at(2027, 1, 9, 0, 0));
    assert!(e.is_empty(), "the standup series ended in December");
}

#[test]
fn an_unknown_zone_is_taken_as_local_time() {
    let ics = "BEGIN:VEVENT\nUID:x\nDTSTART;TZID=\"W. Europe Standard Time\":20261027T100000\nDTEND;TZID=\"W. Europe Standard Time\":20261027T110000\nSUMMARY:Outlook\nEND:VEVENT";
    let local = Local.with_ymd_and_hms(2026, 10, 27, 10, 0, 0).unwrap().timestamp() as f64;
    let e = events_between(ics, local - 3600.0, local + 3600.0);
    assert_eq!(e.iter().map(|e| e.start).collect::<Vec<_>>(), [local]);
}

#[test]
fn durations_and_links() {
    assert_eq!(duration("PT1H30M"), Some(5400.0));
    assert_eq!(duration("P1DT2H"), Some(93600.0));
    assert_eq!(duration("PT"), Some(0.0));
    assert_eq!(duration("1H"), None);
    assert!(looks_like_link("webcal://p55-caldav.icloud.com/published/2/abc"));
    assert!(looks_like_link(" https://calendar.google.com/calendar/ical/x/private-y/basic.ics "));
    assert!(!looks_like_link("calendar.google.com"));
    assert!(!looks_like_link("https://a b"));
}

#[test]
fn an_in_person_meeting_keeps_its_room() {
    let ics = "BEGIN:VEVENT\nUID:r\nDTSTART:20261027T090000Z\nDTEND:20261027T100000Z\nSUMMARY:Review\nLOCATION:Room 4B\\, 3rd floor\nEND:VEVENT\n\
BEGIN:VEVENT\nUID:t\nDTSTART:20261027T110000Z\nDTEND:20261027T120000Z\nSUMMARY:Sync\nLOCATION:Microsoft Teams Meeting\nDESCRIPTION:https://teams.microsoft.com/l/meetup-join/1\nEND:VEVENT";
    let e = events_between(ics, at(2026, 10, 27, 0, 0), at(2026, 10, 28, 0, 0));
    assert_eq!((e[0].place.as_deref(), e[0].link.as_deref()), (Some("Room 4B, 3rd floor"), None));
    assert_eq!(e[1].place, None, "a call's placeholder location isn't a room");
    assert!(e[1].link.is_some());
}
