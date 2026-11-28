//! The schedule forms a person can write, and when each one fires next.
//!
//! Every function here is pure: no clock, no I/O. The caller passes `now` and
//! the time zone, so the same input always gives the same answer. A time zone
//! is a required argument everywhere on purpose: a daily "09:00" means nothing
//! without one, and leaving it to a default is a known source of off-by-hours
//! bugs.
//!
//! The forms are deliberately few. Raw five-field cron is not one of them yet:
//! these cover what people actually write, and each is easy to check.

use anyhow::{bail, Result};
use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc, Weekday,
};
use serde::{Deserialize, Serialize};

/// A schedule text longer than this is pasted text, not a schedule.
pub(crate) const MAX_SCHEDULE_CHARS: usize = 200;

/// Shortest and longest repeat interval, in minutes (one minute to 366 days).
pub(crate) const MIN_INTERVAL_MINUTES: i64 = 1;
pub(crate) const MAX_INTERVAL_MINUTES: i64 = 366 * 24 * 60;

/// Days searched ahead for a matching day. A weekly schedule needs at most 7;
/// one more covers a local date that has no valid time on it.
const LOOKAHEAD_DAYS: i64 = 8;

const SECONDS_PER_MINUTE: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Schedule {
    /// Fires once, at one moment.
    Once { at: DateTime<Utc> },
    /// Fires every `minutes`, counted from `anchor` (the moment it was
    /// created), so a late run never pushes the following ones back.
    Every { minutes: i64, anchor: DateTime<Utc> },
    /// Fires every day at `time`, in the time zone the caller passes.
    Daily { time: NaiveTime },
    /// Fires on the listed weekdays at `time`.
    Weekly { days: Vec<Weekday>, time: NaiveTime },
}

/// Reads a schedule written by a person. `now` fixes the meaning of
/// "in 30m" and the anchor of "every 2h"; `tz` fixes a date and time written
/// without an offset.
pub(crate) fn parse<Tz: TimeZone>(text: &str, now: DateTime<Utc>, tz: &Tz) -> Result<Schedule> {
    let text = text.trim();
    if text.is_empty() {
        bail!("a schedule cannot be empty");
    }
    if text.chars().count() > MAX_SCHEDULE_CHARS {
        bail!("a schedule is limited to {MAX_SCHEDULE_CHARS} characters");
    }
    if text.chars().any(char::is_control) {
        bail!("a schedule cannot hold control characters");
    }
    let lowered = text.to_ascii_lowercase();
    let words: Vec<&str> = lowered.split_whitespace().collect();
    match words.as_slice() {
        ["in", amount] => Ok(Schedule::Once { at: now + Duration::minutes(parse_minutes(amount)?) }),
        ["every", amount] => Ok(Schedule::Every { minutes: parse_minutes(amount)?, anchor: now }),
        ["daily", time] => Ok(Schedule::Daily { time: parse_time(time)? }),
        ["weekdays", time] => Ok(Schedule::Weekly {
            days: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            time: parse_time(time)?,
        }),
        ["weekly", days, time] => Ok(Schedule::Weekly { days: parse_days(days)?, time: parse_time(time)? }),
        ["in" | "every" | "daily" | "weekdays" | "weekly", ..] => {
            bail!("cannot read '{text}': try 'every 30m', 'daily 09:00', 'weekdays 09:00' or 'weekly mon,thu 18:30'")
        }
        _ => parse_moment(text, tz).map(|at| Schedule::Once { at }),
    }
}

/// "30m", "2h" or "1d" as minutes, within the allowed interval.
fn parse_minutes(amount: &str) -> Result<i64> {
    let (digits, unit) = amount.split_at(amount.len().saturating_sub(1));
    let per_unit = match unit {
        "m" => 1,
        "h" => 60,
        "d" => 24 * 60,
        _ => bail!("'{amount}' is not an amount of time: use minutes, hours or days, like 30m, 2h or 1d"),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("'{amount}' is not an amount of time: use whole numbers, like 30m, 2h or 1d");
    }
    let minutes = digits
        .parse::<i64>()
        .ok()
        .and_then(|count| count.checked_mul(per_unit))
        .unwrap_or(i64::MAX);
    if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&minutes) {
        bail!("'{amount}' is out of range: from 1 minute to 366 days");
    }
    Ok(minutes)
}

/// "09:00" or "9:00", 24-hour, minutes always two digits.
fn parse_time(text: &str) -> Result<NaiveTime> {
    let invalid = || anyhow::anyhow!("'{text}' is not a time: use 24-hour HH:MM, like 09:00 or 18:30");
    let (hour, minute) = text.split_once(':').ok_or_else(invalid)?;
    if hour.is_empty() || hour.len() > 2 || minute.len() != 2 {
        return Err(invalid());
    }
    let hour: u32 = hour.parse().map_err(|_| invalid())?;
    let minute: u32 = minute.parse().map_err(|_| invalid())?;
    NaiveTime::from_hms_opt(hour, minute, 0).ok_or_else(invalid)
}

/// "mon,thu" as weekdays, in week order with no repeats.
fn parse_days(text: &str) -> Result<Vec<Weekday>> {
    let mut days = Vec::new();
    for name in text.split(',') {
        let day = match name {
            "mon" => Weekday::Mon,
            "tue" => Weekday::Tue,
            "wed" => Weekday::Wed,
            "thu" => Weekday::Thu,
            "fri" => Weekday::Fri,
            "sat" => Weekday::Sat,
            "sun" => Weekday::Sun,
            _ => bail!("'{name}' is not a weekday: use mon, tue, wed, thu, fri, sat or sun"),
        };
        if !days.contains(&day) {
            days.push(day);
        }
    }
    days.sort_by_key(|day| day.number_from_monday());
    Ok(days)
}

/// One moment: an RFC 3339 time with its offset, or a date and time written
/// in the caller's own time zone ("2026-12-01 09:00").
fn parse_moment<Tz: TimeZone>(text: &str, tz: &Tz) -> Result<DateTime<Utc>> {
    if let Ok(moment) = DateTime::parse_from_rfc3339(text) {
        return Ok(moment.with_timezone(&Utc));
    }
    let naive = ["%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M"]
        .iter()
        .find_map(|format| NaiveDateTime::parse_from_str(text, format).ok());
    let Some(naive) = naive else {
        bail!("cannot read '{text}' as a schedule: try 'in 30m', 'every 2h', 'daily 09:00' or '2026-12-01 09:00'");
    };
    match local_moment(tz, naive.date(), naive.time()) {
        Some(moment) => Ok(moment),
        None => bail!("'{text}' is not a valid local time"),
    }
}

impl Schedule {
    /// The first moment strictly after `after` at which this schedule fires,
    /// or `None` if it never fires again (a one-time moment already past).
    pub(crate) fn next_after<Tz: TimeZone>(&self, after: DateTime<Utc>, tz: &Tz) -> Option<DateTime<Utc>> {
        match self {
            Schedule::Once { at } => (*at > after).then_some(*at),
            Schedule::Every { minutes, anchor } => {
                let step_seconds = minutes * SECONDS_PER_MINUTE;
                let elapsed = (after - *anchor).num_seconds();
                // The first run is one step after the anchor, then every step.
                let steps = if elapsed < step_seconds { 1 } else { elapsed / step_seconds + 1 };
                Some(*anchor + Duration::seconds(steps * step_seconds))
            }
            Schedule::Daily { time } => next_local(after, tz, *time, |_| true),
            Schedule::Weekly { days, time } => next_local(after, tz, *time, |day| days.contains(&day)),
        }
    }
}

/// The next local `time` after `after` on a day `day_ok` accepts.
fn next_local<Tz: TimeZone>(
    after: DateTime<Utc>,
    tz: &Tz,
    time: NaiveTime,
    day_ok: impl Fn(Weekday) -> bool,
) -> Option<DateTime<Utc>> {
    let first_day = after.with_timezone(tz).date_naive();
    (0..=LOOKAHEAD_DAYS)
        .filter_map(|offset| first_day.checked_add_signed(Duration::days(offset)))
        .filter(|date| day_ok(date.weekday()))
        .find_map(|date| local_moment(tz, date, time).filter(|moment| *moment > after))
}

/// A local date and time as a UTC moment. A time that does not exist (the
/// hour skipped when clocks go forward) moves to the same minute one hour
/// later, which does exist.
fn local_moment<Tz: TimeZone>(tz: &Tz, date: NaiveDate, time: NaiveTime) -> Option<DateTime<Utc>> {
    let naive = date.and_time(time);
    resolve_local(tz.from_local_datetime(&naive))
        .or_else(|| resolve_local(tz.from_local_datetime(&(naive + Duration::hours(1)))))
        .map(|moment| moment.with_timezone(&Utc))
}

/// A local time can be unique, ambiguous (clocks go back, the hour happens
/// twice: take the first) or missing (`None`: the caller moves it forward).
fn resolve_local<T>(result: LocalResult<T>) -> Option<T> {
    match result {
        LocalResult::Single(moment) => Some(moment),
        LocalResult::Ambiguous(first, _second) => Some(first),
        LocalResult::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn utc(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text).unwrap().with_timezone(&Utc)
    }

    fn plus7() -> FixedOffset {
        FixedOffset::east_opt(7 * 3600).unwrap()
    }

    // Tuesday 2026-12-01.
    fn now() -> DateTime<Utc> {
        utc("2026-12-01T10:00:00Z")
    }

    fn read(text: &str) -> Result<Schedule> {
        parse(text, now(), &Utc)
    }

    #[test]
    fn reads_every_form() {
        assert_eq!(read("in 30m").unwrap(), Schedule::Once { at: utc("2026-12-01T10:30:00Z") });
        assert_eq!(read("in 2h").unwrap(), Schedule::Once { at: utc("2026-12-01T12:00:00Z") });
        assert_eq!(read("every 90m").unwrap(), Schedule::Every { minutes: 90, anchor: now() });
        assert_eq!(read("every 2h").unwrap(), Schedule::Every { minutes: 120, anchor: now() });
        assert_eq!(read("every 1d").unwrap(), Schedule::Every { minutes: 1440, anchor: now() });
        let nine = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        assert_eq!(read("daily 09:00").unwrap(), Schedule::Daily { time: nine });
        assert_eq!(read("daily 9:00").unwrap(), Schedule::Daily { time: nine });
        assert_eq!(
            read("weekdays 09:00").unwrap(),
            Schedule::Weekly {
                days: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
                time: nine
            }
        );
        assert_eq!(
            read("weekly mon,thu 18:30").unwrap(),
            Schedule::Weekly { days: vec![Weekday::Mon, Weekday::Thu], time: NaiveTime::from_hms_opt(18, 30, 0).unwrap() }
        );
    }

    #[test]
    fn ignores_case_and_extra_spaces_and_orders_days() {
        assert_eq!(read("  EVERY   2H ").unwrap(), read("every 2h").unwrap());
        assert_eq!(
            read("weekly THU,mon,mon 18:30").unwrap(),
            Schedule::Weekly { days: vec![Weekday::Mon, Weekday::Thu], time: NaiveTime::from_hms_opt(18, 30, 0).unwrap() }
        );
    }

    #[test]
    fn reads_one_moment_with_or_without_an_offset() {
        assert_eq!(
            read("2026-12-01T09:00:00+07:00").unwrap(),
            Schedule::Once { at: utc("2026-12-01T02:00:00Z") }
        );
        // Written without an offset, it means the caller's own zone.
        assert_eq!(
            parse("2026-12-01 09:00", now(), &plus7()).unwrap(),
            Schedule::Once { at: utc("2026-12-01T02:00:00Z") }
        );
        assert_eq!(
            parse("2026-12-01T09:00", now(), &plus7()).unwrap(),
            Schedule::Once { at: utc("2026-12-01T02:00:00Z") }
        );
    }

    #[test]
    fn refuses_what_it_cannot_read() {
        let long = "every 5m ".repeat(30);
        let huge = "a".repeat(65_536);
        for bad in [
            "", "   ", "\t", "every", "in", "tomorrow", "every 0m", "every 5s", "every -5m", "every 5", "every m",
            "every 367d", "every 99999d", "every 99999999999999999999d", "in 0m", "daily", "daily 24:00",
            "daily 09:60", "daily 9:5", "daily 09-00", "daily 09:00 extra", "weekdays", "weekly 09:00",
            "weekly funday 09:00", "weekly mon,,thu 09:00", "weekly mon", "2026-13-40 09:00", "2026-12-01 99:99",
            "every 5m\0", "every 5m\r\nrm", long.as_str(), huge.as_str(),
        ] {
            assert!(read(bad).is_err(), "{:?} must be refused", bad.chars().take(30).collect::<String>());
        }
    }

    #[test]
    fn keeps_the_interval_limits_inclusive() {
        assert!(read("every 1m").is_ok());
        assert!(read("every 366d").is_ok());
        assert!(read("every 527040m").is_ok());
        assert!(read("every 527041m").is_err());
    }

    #[test]
    fn a_one_time_moment_fires_once_and_never_after() {
        let schedule = Schedule::Once { at: utc("2026-12-01T12:00:00Z") };
        assert_eq!(schedule.next_after(utc("2026-12-01T11:59:59Z"), &Utc), Some(utc("2026-12-01T12:00:00Z")));
        assert_eq!(schedule.next_after(utc("2026-12-01T12:00:00Z"), &Utc), None, "strictly after");
        assert_eq!(schedule.next_after(utc("2026-12-02T00:00:00Z"), &Utc), None);
    }

    #[test]
    fn an_interval_counts_from_its_anchor_and_does_not_drift() {
        let schedule = Schedule::Every { minutes: 30, anchor: utc("2026-12-01T10:00:00Z") };
        // Before the first step, and exactly at the anchor.
        assert_eq!(schedule.next_after(utc("2026-12-01T09:00:00Z"), &Utc), Some(utc("2026-12-01T10:30:00Z")));
        assert_eq!(schedule.next_after(utc("2026-12-01T10:00:00Z"), &Utc), Some(utc("2026-12-01T10:30:00Z")));
        // Exactly one step after the anchor: that moment has fired, so the next is the step after it.
        assert_eq!(schedule.next_after(utc("2026-12-01T10:30:00Z"), &Utc), Some(utc("2026-12-01T11:00:00Z")));
        // A late check still lands on the grid, not "late + 30m".
        assert_eq!(schedule.next_after(utc("2026-12-01T11:35:00Z"), &Utc), Some(utc("2026-12-01T12:00:00Z")));
        // Exactly on a step: the next one, strictly after.
        assert_eq!(schedule.next_after(utc("2026-12-01T12:00:00Z"), &Utc), Some(utc("2026-12-01T12:30:00Z")));
        // Far in the future.
        assert_eq!(schedule.next_after(utc("2027-12-01T10:10:00Z"), &Utc), Some(utc("2027-12-01T10:30:00Z")));
    }

    #[test]
    fn a_daily_time_follows_the_time_zone_it_is_given() {
        let schedule = Schedule::Daily { time: NaiveTime::from_hms_opt(9, 0, 0).unwrap() };
        // 09:00 at +07:00 is 02:00 UTC.
        assert_eq!(schedule.next_after(utc("2026-12-01T01:59:00Z"), &plus7()), Some(utc("2026-12-01T02:00:00Z")));
        assert_eq!(schedule.next_after(utc("2026-12-01T02:00:00Z"), &plus7()), Some(utc("2026-12-02T02:00:00Z")));
        // 20:00 UTC is already 03:00 the next local day.
        assert_eq!(schedule.next_after(utc("2026-12-01T20:00:00Z"), &plus7()), Some(utc("2026-12-02T02:00:00Z")));
        // The same schedule in UTC fires at a different moment: the zone is not optional.
        assert_eq!(schedule.next_after(utc("2026-12-01T01:59:00Z"), &Utc), Some(utc("2026-12-01T09:00:00Z")));
    }

    #[test]
    fn a_weekly_schedule_picks_the_next_listed_day() {
        let schedule = read("weekly mon,thu 18:30").unwrap();
        // 2026-12-01 is a Tuesday.
        assert_eq!(schedule.next_after(utc("2026-12-01T00:00:00Z"), &Utc), Some(utc("2026-12-03T18:30:00Z")));
        assert_eq!(schedule.next_after(utc("2026-12-03T18:30:00Z"), &Utc), Some(utc("2026-12-07T18:30:00Z")));
        let sunday_only = read("weekly sun 08:00").unwrap();
        assert_eq!(sunday_only.next_after(utc("2026-12-01T00:00:00Z"), &Utc), Some(utc("2026-12-06T08:00:00Z")));
    }

    #[test]
    fn weekdays_skip_the_weekend() {
        let schedule = read("weekdays 09:00").unwrap();
        // 2026-12-04 is a Friday, 2026-12-07 the Monday after.
        assert_eq!(schedule.next_after(utc("2026-12-04T09:00:00Z"), &Utc), Some(utc("2026-12-07T09:00:00Z")));
    }

    #[test]
    fn an_ambiguous_local_time_takes_the_first_and_a_missing_one_is_none() {
        assert_eq!(resolve_local(LocalResult::Single(1)), Some(1));
        assert_eq!(resolve_local(LocalResult::Ambiguous(1, 2)), Some(1));
        assert_eq!(resolve_local::<i32>(LocalResult::None), None);
    }

    #[test]
    fn every_form_survives_a_round_trip_through_json() {
        for text in ["in 30m", "every 2h", "daily 09:00", "weekly mon,thu 18:30", "2026-12-01T09:00:00+07:00"] {
            let schedule = read(text).unwrap();
            let json = serde_json::to_string(&schedule).unwrap();
            assert_eq!(serde_json::from_str::<Schedule>(&json).unwrap(), schedule, "{text}: {json}");
        }
    }
}
