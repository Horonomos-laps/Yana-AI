//! `yana-rt away start|status|stop|report`: how the person (or the app) sets
//! the box before leaving and reads what happened on return.

use super::away::{self, AwayRecord};
use super::level::ControlLevel;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::path::Path;

pub(crate) const DEFAULT_AWAY_MINUTES: i64 = 120;
pub(crate) const DEFAULT_AWAY_BUDGET: u32 = 50;

/// The box as the app shows it. `None` means no box is active.
fn status_json(record: Option<&AwayRecord>, now: DateTime<Utc>, parked: usize) -> Value {
    match record.filter(|record| record.is_active(now)) {
        Some(record) => json!({
            "active": true,
            "id": record.id,
            "level": record.level.number(),
            "budget": record.budget,
            "used": record.used,
            "remaining": record.budget.saturating_sub(record.used),
            "started_at": record.started_at,
            "expires_at": record.expires_at,
            "started_by": record.started_by,
            "parked": parked,
        }),
        None => json!({ "active": false, "parked": parked }),
    }
}

fn describe_in_words(status: &Value) -> String {
    if status["active"] != true {
        return format!("away mode is off; {} item(s) waiting for you", status["parked"]);
    }
    format!(
        "away mode is on at level {} until {}: {} of {} actions used, {} item(s) waiting for you",
        status["level"],
        status["expires_at"].as_str().unwrap_or("?"),
        status["used"],
        status["budget"],
        status["parked"]
    )
}

fn current_status(root: &Path) -> Result<Value> {
    let record = away::read_record(root)?;
    let parked = away::parked_items(root)?.len();
    Ok(status_json(record.as_ref(), Utc::now(), parked))
}

fn print_status(status: &Value, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(status)?);
    } else {
        println!("{}", describe_in_words(status));
    }
    Ok(())
}

pub(crate) fn cmd_away_start(minutes: i64, budget: u32, level: u8, decided_by: &str, json: bool) -> Result<()> {
    let Some(level) = ControlLevel::from_number(level) else {
        bail!("the level must be a number from 1 to 5, not {level}");
    };
    let root = std::env::current_dir().context("cannot resolve project root")?;
    away::start(&root, minutes, budget, level, decided_by)?;
    print_status(&current_status(&root)?, json)
}

pub(crate) fn cmd_away_status(json: bool) -> Result<()> {
    let root = std::env::current_dir().context("cannot resolve project root")?;
    print_status(&current_status(&root)?, json)
}

pub(crate) fn cmd_away_stop(json: bool) -> Result<()> {
    let root = std::env::current_dir().context("cannot resolve project root")?;
    let was_on = away::stop(&root)?.is_some();
    if !json {
        println!("{}", if was_on { "away mode stopped" } else { "away mode was not on" });
    }
    print_status(&current_status(&root)?, json)
}

/// What the person finds on return: the box, and the queue of what waited for them.
pub(crate) fn cmd_away_report(clear: bool, json: bool) -> Result<()> {
    let root = std::env::current_dir().context("cannot resolve project root")?;
    let items = away::parked_items(&root)?;
    let mut status = current_status(&root)?;
    if json {
        status["parked_items"] = serde_json::to_value(&items)?;
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("{}", describe_in_words(&status));
        for item in &items {
            println!("  [{}] {}: {} ({})", item.id, item.capability, item.summary, item.reason);
        }
    }
    if clear && !items.is_empty() {
        away::clear_parked(&root)?;
        if !json {
            println!("cleared {} item(s)", items.len());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn record(level: ControlLevel, used: u32, stopped: bool, expires_in_minutes: i64) -> AwayRecord {
        let now = Utc::now();
        AwayRecord {
            id: "ab12cd34".into(),
            started_at: now,
            expires_at: now + Duration::minutes(expires_in_minutes),
            level,
            budget: 10,
            used,
            started_by: "human:anh".into(),
            stopped_at: stopped.then_some(now),
        }
    }

    #[test]
    fn an_active_box_reports_what_is_left_and_what_is_waiting() {
        let status = status_json(Some(&record(ControlLevel::SafeCommands, 4, false, 30)), Utc::now(), 3);
        assert_eq!(status["active"], true);
        assert_eq!((status["level"].as_u64(), status["remaining"].as_u64(), status["parked"].as_u64()), (Some(4), Some(6), Some(3)));
        assert!(describe_in_words(&status).contains("4 of 10"));
    }

    #[test]
    fn a_missing_stopped_or_expired_box_is_off_and_still_reports_the_queue() {
        let now = Utc::now();
        for record in [None, Some(record(ControlLevel::SafeCommands, 0, true, 30)), Some(record(ControlLevel::SafeCommands, 0, false, -1))] {
            let status = status_json(record.as_ref(), now, 2);
            assert_eq!((status["active"].as_bool(), status["parked"].as_u64()), (Some(false), Some(2)));
            assert!(status.get("id").is_none());
        }
    }

    #[test]
    fn a_level_outside_one_to_five_is_refused_before_anything_is_written() {
        for number in [0u8, 6, 255] {
            assert!(cmd_away_start(60, 10, number, "human:anh", true).is_err(), "{number}");
        }
    }
}
