//! The calendar side of the working day: reading events (from the system calendar and/or an iCal link),
//! heads-ups with the meeting link or the room to walk to, being late or running over, the morning agenda,
//! focus offered in free time, to-dos, and the wrap-ups at the end of the day and the week.

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver};

use super::{now_secs, App};
use chrono::Datelike;

use crate::data::settings::{self, DayRecord, Todo};
use crate::data::ical;
use crate::domain::{CalendarEvent, ClickAction};
use crate::domain::Activity::Remind;
use crate::engine::alarms::clock_label;
use crate::engine::calendar::{heads_up_text, Reminders};
use crate::engine::workday::{self, DayStats};
use crate::platform;

/// How far ahead (seconds) the calendar is read: enough for today's agenda and the next meeting.
const READ_AHEAD: f64 = 16.0 * 3600.0;
/// The user counts as at the computer when they've touched the mouse or keyboard this recently (seconds).
const ACTIVE: f64 = 120.0;
/// A call seen this recently (seconds) is still going.
const IN_CALL: f64 = 30.0;
/// Longest to-do kept (characters).
const MAX_TODO: usize = 80;
/// When the weekly summary comes on Fridays if there's no wrap-up time set (local hour).
const WEEKLY_AT: u32 = 16;

#[derive(Default)]
pub struct Workday {
    reminders: Reminders,
    /// Events from the iCal link, as of its last download.
    feed: Vec<CalendarEvent>,
    feed_rx: Option<Receiver<Option<Vec<CalendarEvent>>>>,
    /// When to download the iCal link again (seconds since the epoch).
    next_fetch: f64,
    /// The link was just set: say whether it worked once it's downloaded.
    announce: bool,
    /// The day (month, day) the morning agenda was given, the meeting focus was last offered before,
    /// and the day the wrap-up was given.
    agenda_day: Option<(u32, u32)>,
    offered_before: Option<String>,
    wrap_day: Option<(u32, u32)>,
    /// When a pet last said it's getting late (seconds since the epoch).
    late_nudge: f64,
    /// Today's tally, saved as it changes so the weekly summary survives restarts.
    today: DayRecord,
    /// When a call was last seen (seconds since the epoch), whether or not focus during calls is on.
    pub last_call: f64,
    /// Meetings already nudged about being late to or running over.
    told_late: HashSet<String>,
    told_over: HashSet<String>,
    /// When the focus session that's on began, if it was started by the user (not a call).
    pub focus_started: Option<f64>,
}

impl App {
    /// Events overlapping the next `within` seconds from every calendar there is, soonest first,
    /// with an event in both the system calendar and the iCal link counted once.
    fn calendar_events(&self, within: f64) -> Vec<CalendarEvent> {
        let now = now_secs();
        // Meetings that ended a little while ago are included too, to tell when one runs over.
        let mut events = platform::calendar_events(workday::OVER_UNTIL, within);
        let fed = self.workday.feed.iter().filter(|e| e.end > now - workday::OVER_UNTIL && e.start < now + within);
        for e in fed {
            if !events.iter().any(|o| o.title.trim() == e.title.trim() && (o.start - e.start).abs() < 60.0) { events.push(e.clone()) }
        }
        events.sort_by(|a, b| a.start.total_cmp(&b.start));
        events
    }

    /// Whether there's any calendar to go by.
    fn has_calendar(&self) -> bool { platform::has_calendar() || settings::calendar_url().is_some() }

    /// Every few seconds: downloads the iCal link now and then (in the background) and picks up the result.
    pub(super) fn poll_calendar_feed(&mut self) {
        if let Some(rx) = &self.workday.feed_rx {
            let Ok(result) = rx.try_recv() else { return };
            self.workday.feed_rx = None;
            let announce = std::mem::take(&mut self.workday.announce);
            match result {
                Some(events) => {
                    if announce {
                        let n = events.iter().filter(|e| e.start < now_secs() + 24.0 * 3600.0).count();
                        self.world.tell(&format!("Got your calendar! 📅 {n} event{} in the next day", if n == 1 { "" } else { "s" }));
                    }
                    self.workday.feed = events;
                }
                None if announce => {
                    let text = "PawPal couldn't download it. Check the link and your internet connection, then try again.";
                    platform::alert("Couldn't read that calendar", text, &["OK"], true);
                }
                None => {}
            }
        }
        let Some(url) = settings::calendar_url() else { return };
        let now = now_secs();
        if now < self.workday.next_fetch { return }
        self.workday.next_fetch = now + ical::REFRESH_EVERY;
        let (tx, rx) = mpsc::channel();
        self.workday.feed_rx = Some(rx);
        std::thread::spawn(move || {
            let events = ical::fetch(&url).map(|text| ical::events_between(&text, now - 3600.0, now + ical::LOOK_AHEAD));
            let _ = tx.send(events);
        });
    }

    /// "🔗 Calendar Link…": asks for (or changes, or removes) the private iCal link.
    pub(super) fn set_calendar_link(&mut self) {
        let current = settings::calendar_url().unwrap_or_default();
        let message = "Paste your calendar's private iCal address.\n\nGoogle Calendar: Settings → your calendar → Secret address in iCal format.\nOutlook: Settings → Calendar → Shared calendars → Publish a calendar → ICS link.\n\nLeave it empty to remove it.";
        let Some(url) = platform::prompt_text("Calendar Link", message, &current) else { return };
        let url = url.trim().to_string();
        if url.is_empty() {
            settings::set_calendar_url(None);
            self.workday.feed.clear();
            self.world.tell("Okay, calendar link removed 📅");
            return;
        }
        if !ical::looks_like_link(&url) {
            platform::alert("That doesn't look like a calendar link", "It should start with https:// or webcal://", &["OK"], true);
            return;
        }
        settings::set_calendar_url(Some(url));
        (self.workday.next_fetch, self.workday.announce, self.workday.feed_rx) = (0.0, true, None);
        self.poll_calendar_feed();
    }

    /// Every few seconds: the next calendar heads-up if one is due, otherwise the morning agenda, focus offered
    /// in free time, or the wrap-up, one at a time and only while the user is at the computer.
    pub(super) fn check_calendar(&mut self) {
        if self.world.pets.is_empty() { return }
        let (now, hour, today) = (now_secs(), platform::local_hour(), platform::local_date());
        let events = self.calendar_events(READ_AHEAD);
        self.tally(&events, now);

        if settings::calendar_heads_ups() {
            if let Some((event, minutes)) = self.workday.reminders.due(&events, now) {
                let mut text = heads_up_text(&event.title, minutes);
                // The last heads-up is the time to join; clicking an earlier one only sends the pet home.
                let join = event.link.clone().filter(|_| minutes <= 1).map(ClickAction::Join);
                if let Some(place) = &event.place {
                    text = workday::walk_over_text(&text, place, minutes);
                } else if join.is_some() {
                    text += " Click me to join 🔗";
                }
                if join.is_none() {
                    if let Some(next) = workday::back_to_back(&events, &event).filter(|_| settings::back_to_back()) {
                        text += &format!(" Then straight into {}, grab water now 💧", next.title.trim());
                    }
                }
                return self.world.remind_with(&text, join);
            }
        }

        let w = &self.world;
        let away = w.mouse_still > ACTIVE && w.since_key > ACTIVE;
        let out = w.pets.iter().any(|p| p.activity == Remind);
        if settings::late_nudges() && !out {
            let in_call = now - self.workday.last_call < IN_CALL;
            if let Some(e) = workday::running_late(&events, now, self.workday.last_call).filter(|e| !away && !self.workday.told_late.contains(&e.id)) {
                self.workday.told_late.insert(e.id.clone());
                return self.world.remind_with(&workday::late_text(e, now), e.link.clone().map(ClickAction::Join));
            }
            if let Some((over, next)) = workday::running_over(&events, now).filter(|(o, _)| in_call && !self.workday.told_over.contains(&o.id)) {
                self.workday.told_over.insert(over.id.clone());
                return self.world.remind_with(&workday::over_text(over, next, now), next.and_then(|n| n.link.clone()).map(ClickAction::Join));
            }
        }
        if away || out || w.fullscreen || w.focusing || self.calls.on { return }

        if self.has_calendar() && settings::morning_agenda() && (5.0..12.0).contains(&hour) && self.workday.agenda_day != Some(today) {
            self.workday.agenda_day = Some(today);
            return self.world.remind(&workday::agenda_text(&events, now, hour, settings::todos().len()));
        }
        if self.has_calendar() && settings::focus_offers() && (7.0..20.0).contains(&hour) {
            if let Some(next) = workday::free_gap(&events, now).filter(|n| self.workday.offered_before.as_ref() != Some(&n.id)) {
                self.workday.offered_before = Some(next.id.clone());
                return self.world.remind_with(&workday::focus_offer_text(next, now, hour), Some(ClickAction::FocusUntil(workday::focus_end(next, now))));
            }
        }
        // On Fridays the week's summary stands in for the day's wrap-up.
        let date = chrono::Local::now().date_naive();
        let weekly_at = settings::wrap_up().unwrap_or(WEEKLY_AT) as f64;
        let friday = date.weekday() == chrono::Weekday::Fri && settings::weekly_summary();
        let date_text = date.format("%Y-%m-%d").to_string();
        if friday && (weekly_at..weekly_at + 3.0).contains(&hour) && settings::last_weekly().as_deref() != Some(date_text.as_str()) {
            settings::set_last_weekly(date_text);
            self.workday.wrap_day = Some(today);
            let week = self.this_week(date);
            return self.world.remind(&workday::week_text(&week));
        }
        let Some(wrap) = settings::wrap_up() else { return };
        if (wrap as f64..wrap as f64 + 2.0).contains(&hour) && self.workday.wrap_day != Some(today) {
            self.workday.wrap_day = Some(today);
            return self.world.remind(&workday::wrap_up_text(&stats(&self.workday.today), settings::todos().len()));
        }
        if workday::past_bedtime(hour, self.world.bedtime) && self.world.since_key < 60.0 && now - self.workday.late_nudge > 12.0 * 3600.0 {
            self.workday.late_nudge = now;
            self.world.remind("🌙 It's getting late… time to call it a night? Your work will still be here tomorrow 🐾");
        }
    }

    /// Counts today's meetings for the wrap-ups.
    fn tally(&mut self, events: &[CalendarEvent], now: f64) {
        let new: Vec<String> = events
            .iter()
            .filter(|e| e.start <= now && now < e.end && !self.workday.today.meetings.contains(&e.id))
            .map(|e| e.id.clone())
            .collect();
        if !new.is_empty() { self.update_today(|d| d.meetings.extend(new)) }
    }

    /// Changes today's tally and saves it, starting a fresh one when the day has changed (or at launch,
    /// picking up today's from before a restart).
    fn update_today(&mut self, f: impl FnOnce(&mut DayRecord)) {
        let date = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
        if self.workday.today.date != date {
            self.workday.today = settings::days().into_iter().find(|d| d.date == date).unwrap_or(DayRecord { date, ..Default::default() });
        }
        f(&mut self.workday.today);
        settings::save_day(self.workday.today.clone());
    }

    /// This week's tallies, Monday to today.
    fn this_week(&mut self, date: chrono::NaiveDate) -> Vec<DayStats> {
        self.update_today(|_| {});
        let monday = (date - chrono::Duration::days(date.weekday().num_days_from_monday() as i64)).format("%Y-%m-%d").to_string();
        settings::days().iter().filter(|d| d.date >= monday).map(stats).collect()
    }

    /// A focus session the user started is over: it counts towards today's tally if it lasted a while.
    pub(super) fn tally_focus(&mut self) {
        let Some(began) = self.workday.focus_started.take() else { return };
        let minutes = (now_secs() - began) / 60.0;
        if minutes >= 5.0 {
            self.update_today(|d| {
                d.focus_sessions += 1;
                d.focus_minutes += minutes;
            });
        }
    }

    /// The user came back after a while away: that was a break.
    pub(super) fn tally_break(&mut self) { self.update_today(|d| d.breaks += 1) }

    /// "📝 Jot It Down…": adds a to-do.
    pub(super) fn jot_down(&mut self) {
        let Some(text) = platform::prompt_text("PawPal", "What do you need to do?", "") else { return };
        let text: String = text.trim().chars().take(MAX_TODO).collect();
        if text.is_empty() { return }
        let mut todos = settings::todos();
        todos.push(Todo { id: (now_secs() * 1000.0) as u64, text });
        settings::set_todos(todos);
        self.world.tell("Got it! I'll keep it on your list 📝");
        self.build_tray();
    }

    /// A to-do ticked off in the tray.
    pub(super) fn done_todo(&mut self, id: u64) {
        let mut todos = settings::todos();
        let before = todos.len();
        todos.retain(|t| t.id != id);
        if todos.len() == before { return }
        let left = todos.len();
        settings::set_todos(todos);
        self.update_today(|d| d.todos_done += 1);
        self.world.tell(&match left {
            0 => "All done! ✅ Nothing left on your list 🎉".to_string(),
            1 => "Nice! ✅ 1 to-do left".to_string(),
            n => format!("Nice! ✅ {n} to-dos left"),
        });
        self.build_tray();
    }

    /// The pet out with a heads-up was clicked, and it offered something.
    pub(super) fn clicked_action(&mut self, action: ClickAction) {
        match action {
            ClickAction::Join(url) => platform::open(url),
            ClickAction::FocusUntil(until) => {
                let now = now_secs();
                if until < now + 60.0 { return }
                (self.focus_until, self.call_focus, self.workday.focus_started) = (Some(until), false, Some(now));
                let label = clock_label(until, now, platform::local_hour());
                self.world.start_focus(&format!("Shh… focus time until {label} 🎯"));
                self.build_tray();
            }
        }
    }
}

/// A saved day's tally, as the wrap-ups count it.
fn stats(d: &DayRecord) -> DayStats {
    DayStats { meetings: d.meetings.len(), focus_sessions: d.focus_sessions, focus_minutes: d.focus_minutes, breaks: d.breaks, todos_done: d.todos_done }
}
