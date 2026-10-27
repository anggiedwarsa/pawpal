//! Reminders the user sets from the tray ("⏰ Remind Me…"): at the time asked, a pet comes out the same
//! way as for a calendar heads-up and says what it was. They're kept in settings, so they survive a restart.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alarm {
    /// When it was set (milliseconds since the epoch), which also tells reminders apart.
    pub id: u64,
    pub text: String,
    /// When it's due (seconds since the epoch).
    pub at: f64,
}

/// Longest reminder text the pet says in full.
const MAX_TEXT: usize = 60;

/// Reads when a reminder is for, from `now` (seconds since the epoch) with the local clock at `hour` (0–24):
/// a delay ("20", "20 min", "1h", "1h 30m", "in 2 hours") or a time of day ("15:30", "3:30 pm", "3pm"),
/// which means tomorrow once it's gone by today. Up to a day ahead.
pub fn parse_when(input: &str, now: f64, hour: f64) -> Option<f64> {
    let s = input.trim().to_lowercase();
    let s = s.strip_prefix("in ").unwrap_or(&s).trim();
    if s.is_empty() { return None }
    let secs = clock_time(s).map(|h| ((h - hour).rem_euclid(24.0) * 3600.0).max(60.0)).or_else(|| delay(s))?;
    (secs > 0.0 && secs <= 86400.0).then(|| now + secs)
}

/// "20", "20 min", "1h 30m", "2 hours" → seconds; a bare number is minutes.
fn delay(s: &str) -> Option<f64> {
    let mut total = 0.0;
    let mut rest = s;
    while !rest.is_empty() {
        let n_end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
        let n: f64 = rest[..n_end].parse().ok()?;
        rest = rest[n_end..].trim_start();
        let u_end = rest.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(rest.len());
        let unit = match &rest[..u_end] {
            "" | "m" | "min" | "mins" | "minute" | "minutes" => 60.0,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3600.0,
            "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
            _ => return None,
        };
        total += n * unit;
        rest = rest[u_end..].trim_start().trim_start_matches(|c| c == ',' || c == '&').trim_start();
        rest = rest.strip_prefix("and ").unwrap_or(rest).trim_start();
    }
    Some(total)
}

/// "15:30", "3:30 pm", "3pm", "12am" → the hour of the day (0–24).
fn clock_time(s: &str) -> Option<f64> {
    let (s, half) = match s.strip_suffix("pm").or_else(|| s.strip_suffix("p.m.")) {
        Some(t) => (t.trim(), Some(12.0)),
        None => match s.strip_suffix("am").or_else(|| s.strip_suffix("a.m.")) {
            Some(t) => (t.trim(), Some(0.0)),
            None => (s, None),
        },
    };
    let (h, m) = match s.split_once([':', '.']) {
        Some((h, m)) => (h.parse::<u32>().ok()?, m.parse::<u32>().ok().filter(|_| m.len() == 2)?),
        // A bare number is a time only with am/pm; otherwise it's minutes.
        None if half.is_some() => (s.parse::<u32>().ok()?, 0),
        None => return None,
    };
    if m >= 60 { return None }
    let h = match half {
        Some(add) if (1..=12).contains(&h) => (h % 12) as f64 + add,
        Some(_) => return None,
        None if h < 24 => h as f64,
        None => return None,
    };
    Some(h + m as f64 / 60.0)
}

/// The local time of `at`, as a menu or bubble shows it ("3:30 PM"), with the clock at `hour` at `now`.
pub fn clock_label(at: f64, now: f64, hour: f64) -> String {
    let minutes = ((hour + (at - now) / 3600.0).rem_euclid(24.0) * 60.0).round() as u32 % (24 * 60);
    let (h, m) = (minutes / 60, minutes % 60);
    let h12 = if h % 12 == 0 { 12 } else { h % 12 };
    let day = if (at - now) / 3600.0 + hour >= 24.0 { " tomorrow" } else { "" };
    format!("{h12}:{m:02} {}{day}", if h < 12 { "AM" } else { "PM" })
}

/// What the pet says when it's due.
pub fn alarm_text(text: &str) -> String {
    let text = text.trim();
    let text = if text.is_empty() { "You asked me to remind you!" } else { text };
    let short: String = if text.chars().count() > MAX_TEXT { text.chars().take(MAX_TEXT - 1).collect::<String>() + "…" } else { text.to_string() };
    format!("⏰ {short}")
}

/// Takes out the first reminder due at `now`, if any (soonest first).
pub fn take_due(alarms: &mut Vec<Alarm>, now: f64) -> Option<Alarm> {
    let i = (0..alarms.len()).filter(|&i| alarms[i].at <= now).min_by(|&a, &b| alarms[a].at.total_cmp(&alarms[b].at))?;
    Some(alarms.remove(i))
}

#[cfg(test)]
#[path = "alarms_tests.rs"]
mod tests;
