//! The away mode: work that continues while the person is gone, inside a box
//! the person drew before leaving.
//!
//! The box has a time, a number of actions it may approve on its own, and a
//! control level (3 to 5) that says which actions those are. Inside it, a call
//! that would have asked runs and is recorded. Outside it, a call is not
//! refused and not approved: it is parked for the person, and the agent is
//! told to carry on with other work. Nothing dangerous is ever approved, here
//! or at any level; it all waits.
//!
//! Only the person starts it. The record lives in the person's own store, and
//! an agent cannot run `yana-rt away` for itself (`level_rules`).

use super::away_parked::{park, read_json, write_json, ParkedItem};
use super::level::{verdict, why_not, CallFacts, ControlLevel, LevelVerdict};
use crate::capability::lease::user_store_file;
use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(crate) const MIN_AWAY_MINUTES: i64 = 5;
/// A day. A longer absence should start a new box the person has looked at again.
pub(crate) const MAX_AWAY_MINUTES: i64 = 24 * 60;
pub(crate) const MAX_AWAY_BUDGET: u32 = 1000;

const AWAY_STORE: &str = "away";
const PARKED_STORE: &str = "parked";
const LOCK_KEY: &str = "key:away-store";
const LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AwayRecord {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub level: ControlLevel,
    /// How many actions this box may approve on its own, and how many it has.
    pub budget: u32,
    pub used: u32,
    pub started_by: String,
    pub stopped_at: Option<DateTime<Utc>>,
}

impl AwayRecord {
    pub(crate) fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.stopped_at.is_none() && self.expires_at > now
    }
}

/// What the away mode decides for one call.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AwayOutcome {
    /// No active box: the usual approval applies.
    NotAway,
    Approved { away_id: String },
    /// Not approved and not refused: waiting for the person, with the reason.
    Parked { reason: String },
}

fn away_file(root: &Path) -> Result<PathBuf> {
    user_store_file(root, AWAY_STORE)
}

pub(super) fn parked_file(root: &Path) -> Result<PathBuf> {
    user_store_file(root, PARKED_STORE)
}

/// One critical section for the record and the queue, so two calls cannot both spend the last unit.
fn locked<T>(root: &Path, action: impl FnOnce() -> Result<T>) -> Result<T> {
    match yana_rt::flock_v1::with_lock(LOCK_KEY, root, LOCK_TIMEOUT, action) {
        Ok(inner) => inner,
        Err(error) => Err(error.context("could not acquire the away store lock")),
    }
}

pub(crate) fn read_record(root: &Path) -> Result<Option<AwayRecord>> {
    read_json(&away_file(root)?)
}

/// Starts the box. Refused while another is active: extending one in silence
/// is exactly what the person should decide again.
pub(crate) fn start(root: &Path, minutes: i64, budget: u32, level: ControlLevel, started_by: &str) -> Result<AwayRecord> {
    if !(MIN_AWAY_MINUTES..=MAX_AWAY_MINUTES).contains(&minutes) {
        bail!("away time must be from {MIN_AWAY_MINUTES} minutes to {MAX_AWAY_MINUTES} minutes (24 hours)");
    }
    if budget == 0 || budget > MAX_AWAY_BUDGET {
        bail!("the away budget must be from 1 to {MAX_AWAY_BUDGET} actions");
    }
    if !(ControlLevel::ProjectEdits..=ControlLevel::Sandboxed).contains(&level) {
        bail!("the away mode needs a level from 3 to 5; at 1 or 2 nothing would run without asking");
    }
    let started_by = started_by.trim();
    if started_by.is_empty() || started_by.chars().any(char::is_control) {
        bail!("who started the away mode must be a plain, non-empty name");
    }
    let path = away_file(root)?;
    locked(root, || {
        let now = Utc::now();
        if let Some(existing) = read_json::<AwayRecord>(&path)?.filter(|record| record.is_active(now)) {
            bail!("the away mode is already on until {}; stop it first", existing.expires_at);
        }
        let record = AwayRecord {
            id: Uuid::new_v4().simple().to_string()[..8].to_string(),
            started_at: now,
            expires_at: now + Duration::minutes(minutes),
            level,
            budget,
            used: 0,
            started_by: started_by.to_string(),
            stopped_at: None,
        };
        write_json(&path, &record)?;
        Ok(record)
    })
}

/// Ends the box now. `None` when none was active.
pub(crate) fn stop(root: &Path) -> Result<Option<AwayRecord>> {
    let path = away_file(root)?;
    locked(root, || {
        let now = Utc::now();
        let Some(mut record) = read_json::<AwayRecord>(&path)?.filter(|record| record.is_active(now)) else {
            return Ok(None);
        };
        record.stopped_at = Some(now);
        write_json(&path, &record)?;
        Ok(Some(record))
    })
}

/// Decides one call. Anything that goes wrong reading the box means "not
/// away": the person is asked as usual, never approved by accident.
pub(crate) fn judge(root: &Path, facts: &CallFacts, session_id: &str) -> AwayOutcome {
    match try_judge(root, facts, session_id) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("[away] could not check the away mode, asking as usual: {error:#}");
            AwayOutcome::NotAway
        }
    }
}

/// The two files of the away mode, resolved once.
pub(super) struct AwayFiles {
    record: PathBuf,
    parked: PathBuf,
}

impl AwayFiles {
    pub(super) fn for_root(root: &Path) -> Result<Self> {
        Ok(Self { record: away_file(root)?, parked: parked_file(root)? })
    }
}

fn try_judge(root: &Path, facts: &CallFacts, session_id: &str) -> Result<AwayOutcome> {
    decide(root, &AwayFiles::for_root(root)?, facts, session_id)
}

/// `judge` with the files given, so a test can race several callers on one box.
pub(super) fn decide(root: &Path, files: &AwayFiles, facts: &CallFacts, session_id: &str) -> Result<AwayOutcome> {
    locked(root, || {
        let now = Utc::now();
        let Some(mut record) = read_json::<AwayRecord>(&files.record)?.filter(|record| record.is_active(now)) else {
            return Ok(AwayOutcome::NotAway);
        };
        let reason = if verdict(record.level, facts) != LevelVerdict::Allow {
            why_not(facts)
        } else if record.used >= record.budget {
            "the away budget is used up".to_string()
        } else {
            record.used += 1;
            write_json(&files.record, &record)?;
            return Ok(AwayOutcome::Approved { away_id: record.id });
        };
        park(&files.parked, ParkedItem::from_call(session_id, facts, &reason))?;
        Ok(AwayOutcome::Parked { reason })
    })
}

/// The items waiting for the person, oldest first. Read only, so no lock: a
/// writer replaces the file atomically and a reader sees the old or the new one.
pub(crate) fn parked_items(root: &Path) -> Result<Vec<ParkedItem>> {
    super::away_parked::list(&parked_file(root)?)
}

/// Empties the queue once the person has read it; how many were waiting.
pub(crate) fn clear_parked(root: &Path) -> Result<usize> {
    let path = parked_file(root)?;
    locked(root, || super::away_parked::clear(&path))
}

#[cfg(test)]
#[path = "away_tests.rs"]
mod tests;
