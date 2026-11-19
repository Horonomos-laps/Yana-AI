//! What the away mode could not do on its own: the queue the person finds on
//! return. An item is a description for a human to read, not a way to run
//! anything: it records what was asked, never the means to approve it.

use super::level::CallFacts;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// The queue keeps the newest items; an older one is dropped past this.
pub(crate) const MAX_PARKED_ITEMS: usize = 200;

/// Longest description kept for one item.
const MAX_SUMMARY_CHARS: usize = 200;

/// A record file is read only up to this size; anything larger is not one.
const MAX_STORE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParkedItem {
    pub id: String,
    pub parked_at: DateTime<Utc>,
    pub session_id: String,
    pub capability: String,
    /// The command or path, trimmed, with control characters turned into spaces.
    pub summary: String,
    /// Why the away mode did not approve it.
    pub reason: String,
}

impl ParkedItem {
    pub(crate) fn from_call(session_id: &str, facts: &CallFacts, reason: &str) -> Self {
        let subject = facts.command.or(facts.path).unwrap_or("");
        let summary: String = subject
            .chars()
            .map(|character| if character.is_control() { ' ' } else { character })
            .take(MAX_SUMMARY_CHARS)
            .collect();
        Self {
            id: Uuid::new_v4().simple().to_string()[..8].to_string(),
            parked_at: Utc::now(),
            session_id: session_id.chars().filter(|character| !character.is_control()).collect(),
            capability: facts.capability.to_string(),
            summary: summary.trim().to_string(),
            reason: reason.to_string(),
        }
    }
}

/// A JSON file in the person's store: missing is `None`, a symlink, a
/// non-file or an oversized file is an error.
pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("cannot inspect {}", path.display())),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        anyhow::bail!("{} must be a regular file", path.display());
    }
    if metadata.len() > MAX_STORE_BYTES {
        anyhow::bail!("{} is larger than {MAX_STORE_BYTES} bytes", path.display());
    }
    let text = fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&text).map(Some).with_context(|| format!("invalid record {}", path.display()))
}

/// Replaces the file atomically (temporary file in the same directory, then rename).
pub(super) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path.parent().context("the record has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("cannot create {}", parent.display()))?;
    let temporary = path.with_extension(format!("json.tmp.{}", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("cannot write {}", temporary.display()))?;
    fs::rename(&temporary, path).with_context(|| format!("cannot replace {}", path.display()))
}

/// Adds an item, dropping the oldest past `MAX_PARKED_ITEMS`. The caller holds the away lock.
pub(super) fn park(path: &Path, item: ParkedItem) -> Result<()> {
    let mut items: Vec<ParkedItem> = read_json(path)?.unwrap_or_default();
    items.push(item);
    if items.len() > MAX_PARKED_ITEMS {
        let excess = items.len() - MAX_PARKED_ITEMS;
        items.drain(..excess);
    }
    write_json(path, &items)
}

pub(super) fn list(path: &Path) -> Result<Vec<ParkedItem>> {
    Ok(read_json(path)?.unwrap_or_default())
}

pub(super) fn clear(path: &Path) -> Result<usize> {
    let count = list(path)?.len();
    write_json(path, &Vec::<ParkedItem>::new())?;
    Ok(count)
}
