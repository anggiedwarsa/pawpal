//! Calendar events from a private iCal link (Google Calendar's "secret address", Outlook's published
//! calendar…), for heads-ups on systems without a calendar PawPal can read, or for calendars not on this Mac.

use std::collections::HashSet;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rrule::{RRule, RRuleSet, Tz, Unvalidated};

use crate::domain::CalendarEvent;
use crate::engine::workday::meeting_link;

/// How often the calendar is fetched again (seconds).
pub const REFRESH_EVERY: f64 = 10.0 * 60.0;
/// How far ahead (seconds) repeating events are worked out at each fetch.
pub const LOOK_AHEAD: f64 = 36.0 * 3600.0;

/// Downloads the calendar. `webcal://` links (as Apple and Outlook give them) are fetched over https.
pub fn fetch(url: &str) -> Option<String> {
    let url = url.trim();
    let url = url.strip_prefix("webcal://").map_or_else(|| url.to_string(), |rest| format!("https://{rest}"));
    let text = super::web::get(&url, &[])?;
    text.contains("BEGIN:VCALENDAR").then_some(text)
}

/// Whether a link the user typed in looks like a calendar address.
pub fn looks_like_link(url: &str) -> bool {
    let u = url.trim();
    (u.starts_with("https://") || u.starts_with("webcal://")) && !u.contains(char::is_whitespace)
}

/// One VEVENT's properties: name, parameters (such as `TZID=Europe/Paris`) and value.
type Props = Vec<(String, String, String)>;

/// Timed events overlapping `from`…`to` (seconds since the epoch), repeats included. All-day and cancelled
/// events are left out, as the Calendar app's are.
pub fn events_between(ics: &str, from: f64, to: f64) -> Vec<CalendarEvent> {
    let blocks = vevents(&unfold(ics));
    // A changed single repeat of a meeting comes as its own event with RECURRENCE-ID, replacing that repeat.
    let moved: HashSet<(String, i64)> = blocks
        .iter()
        .filter_map(|b| Some((prop(b, "UID")?.2.clone(), time(prop(b, "RECURRENCE-ID")?)?.timestamp())))
        .collect();
    let mut out = Vec::new();
    for b in &blocks {
        if prop(b, "STATUS").is_some_and(|p| p.2.eq_ignore_ascii_case("CANCELLED")) { continue }
        let Some(start) = prop(b, "DTSTART").and_then(time) else { continue };
        let length = prop(b, "DTEND").and_then(time).map(|end| (end - start).num_seconds() as f64).or_else(|| prop(b, "DURATION").and_then(|p| duration(&p.2))).unwrap_or(1800.0).max(0.0);
        let uid = prop(b, "UID").map_or_else(String::new, |p| p.2.clone());
        let title = prop(b, "SUMMARY").map_or_else(String::new, |p| unescape(&p.2));
        let texts: Vec<String> = ["URL", "LOCATION", "DESCRIPTION", "X-GOOGLE-CONFERENCE"].iter().filter_map(|n| prop(b, n).map(|p| unescape(&p.2))).collect();
        let link = meeting_link(&texts.iter().map(String::as_str).collect::<Vec<_>>());
        let starts = match (prop(b, "RRULE"), prop(b, "RECURRENCE-ID")) {
            (Some(rule), None) => repeats(start, &rule.2, b, from - length, to).into_iter().filter(|t| !moved.contains(&(uid.clone(), *t as i64))).collect(),
            _ => vec![start.timestamp() as f64],
        };
        for s in starts {
            if s + length <= from || s >= to { continue }
            out.push(CalendarEvent { id: format!("{uid}@{s}"), title: title.clone(), start: s, end: s + length, link: link.clone() });
        }
    }
    out.sort_by(|a, b| a.start.total_cmp(&b.start));
    out
}

/// Joins folded lines back up (a line starting with a space or tab continues the one before).
fn unfold(ics: &str) -> String {
    ics.replace("\r\n", "\n").replace("\n ", "").replace("\n\t", "")
}

/// The properties of each VEVENT, leaving out those of anything nested inside it (reminders' VALARMs).
fn vevents(text: &str) -> Vec<Props> {
    let mut out = Vec::new();
    let (mut current, mut depth): (Option<Props>, u32) = (None, 0);
    for line in text.lines() {
        if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
            (current, depth) = (Some(Vec::new()), 0);
        } else if line.eq_ignore_ascii_case("END:VEVENT") {
            out.extend(current.take());
        } else if let Some(props) = &mut current {
            if line.starts_with("BEGIN:") { depth += 1 } else if line.starts_with("END:") { depth = depth.saturating_sub(1) } else if depth == 0 {
                let Some((head, value)) = line.split_once(':') else { continue };
                let (name, params) = head.split_once(';').unwrap_or((head, ""));
                props.push((name.to_ascii_uppercase(), params.to_string(), value.to_string()));
            }
        }
    }
    out
}

fn prop<'a>(b: &'a Props, name: &str) -> Option<&'a (String, String, String)> { b.iter().find(|p| p.0 == name) }

fn param<'a>(params: &'a str, name: &str) -> Option<&'a str> {
    params.split(';').find_map(|kv| kv.split_once('=').filter(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.trim_matches('"')))
}

/// The time a DTSTART-like property gives, in its own time zone; None for a date without a time (all day).
fn time(p: &(String, String, String)) -> Option<DateTime<Tz>> {
    let (params, value) = (&p.1, p.2.trim());
    if param(params, "VALUE").is_some_and(|v| v.eq_ignore_ascii_case("DATE")) || value.len() == 8 { return None }
    if let Some(utc) = value.strip_suffix('Z') {
        let t = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S").ok()?;
        return Some(Utc.from_utc_datetime(&t).with_timezone(&Tz::UTC));
    }
    let t = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;
    // A zone chrono-tz doesn't know (Outlook's Windows names) is taken to be the computer's own.
    let zone = param(params, "TZID").and_then(|z| z.parse::<chrono_tz::Tz>().ok()).map_or(Tz::LOCAL, Tz::Tz);
    zone.from_local_datetime(&t).earliest()
}

/// Start times (seconds since the epoch) of a repeating event's repeats from `from` to `to`.
fn repeats(start: DateTime<Tz>, rule: &str, b: &Props, from: f64, to: f64) -> Vec<f64> {
    let tz = start.timezone();
    // UNTIL is often given in UTC (or as a date) while DTSTART isn't, which rrule won't accept: it's
    // applied here instead.
    let until = rule.split(';').find_map(|kv| kv.strip_prefix("UNTIL=")).and_then(until_time);
    let rule: String = rule.split(';').filter(|kv| !kv.starts_with("UNTIL=")).collect::<Vec<_>>().join(";");
    let Ok(parsed) = rule.parse::<RRule<Unvalidated>>() else { return vec![] };
    let Ok(valid) = parsed.validate(start) else { return vec![] };
    let mut set = RRuleSet::new(start).rrule(valid);
    for ex in b.iter().filter(|p| p.0 == "EXDATE") {
        for v in ex.2.split(',') {
            if let Some(t) = time(&(ex.0.clone(), ex.1.clone(), v.to_string())) { set = set.exdate(t.with_timezone(&tz)) }
        }
    }
    let end = until.map_or(to, |u| u.min(to));
    let (Some(a), Some(z)) = (DateTime::from_timestamp(from as i64 - 1, 0), DateTime::from_timestamp(end as i64 + 1, 0)) else { return vec![] };
    set.after(a.with_timezone(&tz)).before(z.with_timezone(&tz)).all(500).dates.iter().map(|d| d.timestamp() as f64).collect()
}

/// An RRULE's UNTIL, in seconds since the epoch: a UTC time, a local time, or the end of a date.
fn until_time(v: &str) -> Option<f64> {
    if let Some(utc) = v.strip_suffix('Z') { return NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S").ok().map(|t| t.and_utc().timestamp() as f64) }
    if let Ok(t) = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S") { return Local.from_local_datetime(&t).earliest().map(|t| t.timestamp() as f64) }
    let d = NaiveDate::parse_from_str(v, "%Y%m%d").ok()?;
    Local.from_local_datetime(&d.and_hms_opt(23, 59, 59)?).earliest().map(|t| t.timestamp() as f64)
}

/// A DURATION such as `PT1H30M` or `P1D`, in seconds.
fn duration(v: &str) -> Option<f64> {
    let rest = v.trim().strip_prefix('P')?;
    let (mut total, mut num) = (0.0, String::new());
    for c in rest.chars() {
        match c {
            '0'..='9' => num.push(c),
            'T' => {}
            'W' | 'D' | 'H' | 'M' | 'S' => {
                let n: f64 = num.parse().ok()?;
                total += n * match c { 'W' => 604800.0, 'D' => 86400.0, 'H' => 3600.0, 'M' => 60.0, _ => 1.0 };
                num.clear();
            }
            _ => return None,
        }
    }
    Some(total)
}

/// Text values escape commas, semicolons and newlines.
fn unescape(v: &str) -> String {
    v.replace("\\n", "\n").replace("\\N", "\n").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}

#[cfg(test)]
#[path = "ical_tests.rs"]
mod tests;
