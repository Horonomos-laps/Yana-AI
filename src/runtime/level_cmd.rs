//! `yana-rt level get` and `yana-rt level set`: how the app (and a person at
//! a terminal) reads and changes the control level.
//!
//! The report says plainly whether a level is enforced or only being observed,
//! so a screen never shows a level as protection that is not applied yet.

use super::level::ControlLevel;
use super::level_shadow::MAX_ENFORCED_LEVEL;
use super::level_store::{describe_level, write_level, LevelInfo};
use anyhow::{bail, Context, Result};

/// The report for one level: its number and name, and whether its rules are
/// enforced or run in shadow.
fn report(info: &LevelInfo) -> serde_json::Value {
    let name = serde_json::to_value(info.level).unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "level": info.level.number(),
        "name": name,
        "enforced": info.level <= MAX_ENFORCED_LEVEL,
        "enforced_up_to": MAX_ENFORCED_LEVEL.number(),
        "set_by": info.set_by,
        "set_at": info.set_at,
        "problem": info.problem,
    })
}

fn describe_in_words(info: &LevelInfo) -> String {
    let status = if info.level <= MAX_ENFORCED_LEVEL {
        "enforced".to_string()
    } else {
        format!("observed only: enforced up to level {}, the rest is recorded, not applied", MAX_ENFORCED_LEVEL.number())
    };
    let mut text = format!("control level {} ({})", info.level.number(), status);
    if let Some(problem) = &info.problem {
        text.push_str(&format!("\nwarning: the record could not be read, so the strictest level applies: {problem}"));
    }
    text
}

pub(crate) fn cmd_level_get(json: bool) -> Result<()> {
    let root = std::env::current_dir().context("cannot resolve project root")?;
    let info = describe_level(&root);
    if json {
        println!("{}", serde_json::to_string_pretty(&report(&info))?);
    } else {
        println!("{}", describe_in_words(&info));
    }
    Ok(())
}

pub(crate) fn cmd_level_set(number: u8, decided_by: &str, json: bool) -> Result<()> {
    let Some(level) = ControlLevel::from_number(number) else {
        bail!("the level must be a number from 1 to 5, not {number}");
    };
    let root = std::env::current_dir().context("cannot resolve project root")?;
    write_level(&root, level, decided_by)?;
    let info = describe_level(&root);
    if json {
        println!("{}", serde_json::to_string_pretty(&report(&info))?);
    } else {
        println!("{}", describe_in_words(&info));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn info(level: ControlLevel) -> LevelInfo {
        LevelInfo { level, set_by: Some("human:anh".into()), set_at: Some(Utc::now()), problem: None }
    }

    #[test]
    fn the_report_says_which_levels_are_enforced_and_which_are_only_observed() {
        let enforced: Vec<bool> = (1..=5)
            .map(|number| report(&info(ControlLevel::from_number(number).unwrap()))["enforced"].as_bool().unwrap())
            .collect();
        assert_eq!(enforced, [true, true, false, false, false]);
        let three = report(&info(ControlLevel::ProjectEdits));
        assert_eq!((three["level"].as_u64(), three["name"].as_str()), (Some(3), Some("project_edits")));
        assert_eq!(three["enforced_up_to"], 2);
    }

    #[test]
    fn an_unreadable_record_is_reported_with_the_strictest_level() {
        let broken = LevelInfo { level: ControlLevel::Strict, set_by: None, set_at: None, problem: Some("bad".into()) };
        let value = report(&broken);
        assert_eq!((value["level"].as_u64(), value["problem"].as_str()), (Some(1), Some("bad")));
        assert!(describe_in_words(&broken).contains("warning"));
    }

    #[test]
    fn observed_levels_are_described_as_such_in_words() {
        assert!(describe_in_words(&info(ControlLevel::SafeCommands)).contains("observed only"));
        assert!(!describe_in_words(&info(ControlLevel::Remember)).contains("observed"));
    }

    #[test]
    fn a_level_outside_one_to_five_is_refused() {
        for number in [0u8, 6, 7, 255] {
            assert!(cmd_level_set(number, "human:anh", true).is_err(), "{number}");
        }
    }
}
