//! Unit tests for engine/alarms.rs.

use super::*;

const NOW: f64 = 1_000_000.0;
/// 2:15 PM.
const HOUR: f64 = 14.25;

fn when(s: &str) -> Option<f64> { parse_when(s, NOW, HOUR).map(|t| t - NOW) }

#[test]
fn delays_in_minutes_and_hours() {
    assert_eq!(when("20"), Some(1200.0), "a bare number is minutes");
    assert_eq!(when("20 min"), Some(1200.0));
    assert_eq!(when("in 20 minutes"), Some(1200.0));
    assert_eq!(when("1h"), Some(3600.0));
    assert_eq!(when("1h 30m"), Some(5400.0));
    assert_eq!(when("1 hour and 15 minutes"), Some(4500.0));
    assert_eq!(when("1.5h"), Some(5400.0));
    assert_eq!(when("90s"), Some(90.0));
    assert_eq!(when("  2 Hours "), Some(7200.0));
}

#[test]
fn times_of_day() {
    assert_eq!(when("15:30"), Some(1.25 * 3600.0));
    assert_eq!(when("3:30 pm"), Some(1.25 * 3600.0));
    assert_eq!(when("3:30PM"), Some(1.25 * 3600.0));
    assert_eq!(when("3pm"), Some(0.75 * 3600.0));
    assert_eq!(when("14:30"), Some(0.25 * 3600.0));
}

#[test]
fn a_time_gone_by_today_is_tomorrow() {
    assert_eq!(when("9:00"), Some((24.0 - 5.25) * 3600.0));
    assert_eq!(when("12am"), Some((24.0 - 14.25) * 3600.0), "midnight");
    assert_eq!(when("12pm"), Some((24.0 - 2.25) * 3600.0), "noon");
}

#[test]
fn nonsense_and_too_far_are_refused() {
    for s in ["", "soon", "tomorrow", "25:00", "3:75", "13pm", "0 min", "2 days", "48h", "20 apples", "3:5"] {
        assert_eq!(when(s), None, "{s:?}");
    }
}

#[test]
fn labels_read_like_a_clock() {
    assert_eq!(clock_label(NOW + 1.25 * 3600.0, NOW, HOUR), "3:30 PM");
    assert_eq!(clock_label(NOW + 0.0, NOW, 0.5), "12:30 AM");
    assert_eq!(clock_label(NOW + 3600.0, NOW, 11.0), "12:00 PM");
    assert_eq!(clock_label(NOW + 19.0 * 3600.0, NOW, HOUR), "9:15 AM tomorrow");
}

#[test]
fn the_pet_says_what_it_was() {
    assert_eq!(alarm_text(" Take out the laundry "), "⏰ Take out the laundry");
    assert_eq!(alarm_text(""), "⏰ You asked me to remind you!");
    let long = alarm_text(&"x".repeat(100));
    assert!(long.ends_with('…') && long.chars().count() == MAX_TEXT + 2);
}

#[test]
fn due_reminders_come_soonest_first_and_once() {
    let a = |id, at| Alarm { id, text: format!("#{id}"), at };
    let mut alarms = vec![a(1, 300.0), a(2, 100.0), a(3, 900.0)];
    assert_eq!(take_due(&mut alarms, 50.0), None);
    assert_eq!(take_due(&mut alarms, 400.0).map(|x| x.id), Some(2));
    assert_eq!(take_due(&mut alarms, 400.0).map(|x| x.id), Some(1));
    assert_eq!(take_due(&mut alarms, 400.0), None);
    assert_eq!(alarms.len(), 1);
}

#[test]
fn reminders_round_trip_as_json() {
    let x = Alarm { id: 7, text: "Call mum".into(), at: 123.5 };
    assert_eq!(serde_json::from_str::<Alarm>(&serde_json::to_string(&x).unwrap()).unwrap(), x);
}
