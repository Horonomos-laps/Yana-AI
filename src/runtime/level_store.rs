//! The control level a person chose, kept outside the repository.
//!
//! It lives in the person's own store, next to their remembered answers, for
//! the same reason: a file inside the project could be rewritten by a command
//! the person approved, and an agent that could raise its own level would not
//! be limited by it. Anything unreadable fails closed, to the strictest level.

use super::level::ControlLevel;
use crate::capability::lease::user_store_file;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const LEVEL_STORE: &str = "levels";

/// The record is a few dozen bytes; anything larger is not one.
const MAX_RECORD_BYTES: u64 = 4096;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LevelRecord {
    level: ControlLevel,
    set_by: String,
    set_at: DateTime<Utc>,
}

fn read_record(root: &Path) -> Result<Option<LevelRecord>> {
    let path = user_store_file(root, LEVEL_STORE)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("cannot inspect {}", path.display())),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("the level record must be a regular file: {}", path.display());
    }
    if metadata.len() > MAX_RECORD_BYTES {
        bail!("the level record is larger than {MAX_RECORD_BYTES} bytes: {}", path.display());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&text).map(Some).with_context(|| format!("invalid level record {}", path.display()))
}

/// The level in force for a repository, who chose it and when, and why the
/// record could not be trusted if it could not.
#[derive(Debug)]
pub(crate) struct LevelInfo {
    pub level: ControlLevel,
    pub set_by: Option<String>,
    pub set_at: Option<DateTime<Utc>>,
    pub problem: Option<String>,
}

/// Nothing chosen yet is the default level; a record that cannot be read or
/// trusted is the strictest, with the reason.
pub(crate) fn describe_level(root: &Path) -> LevelInfo {
    match read_record(root) {
        Ok(Some(record)) => {
            LevelInfo { level: record.level, set_by: Some(record.set_by), set_at: Some(record.set_at), problem: None }
        }
        Ok(None) => LevelInfo { level: ControlLevel::default(), set_by: None, set_at: None, problem: None },
        Err(error) => LevelInfo {
            level: ControlLevel::Strict,
            set_by: None,
            set_at: None,
            problem: Some(format!("{error:#}")),
        },
    }
}

/// The level for this repository, saying why on stderr if it fell back to the strictest.
pub(crate) fn read_level(root: &Path) -> ControlLevel {
    let info = describe_level(root);
    if let Some(problem) = &info.problem {
        eprintln!("[level] could not read the control level, using the strictest: {problem}");
    }
    info.level
}

/// Records the level the person chose, replacing any earlier one.
pub(crate) fn write_level(root: &Path, level: ControlLevel, set_by: &str) -> Result<()> {
    let set_by = set_by.trim();
    if set_by.is_empty() || set_by.chars().any(char::is_control) {
        bail!("who set the level must be a plain, non-empty name");
    }
    let path = user_store_file(root, LEVEL_STORE)?;
    let parent = path.parent().context("the level record has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("cannot create {}", parent.display()))?;
    let record = LevelRecord { level, set_by: set_by.to_string(), set_at: Utc::now() };
    let temporary = path.with_extension(format!("json.tmp.{}", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(&record)?)
        .with_context(|| format!("cannot write {}", temporary.display()))?;
    fs::rename(&temporary, &path).with_context(|| format!("cannot replace {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn record_path(root: &Path) -> std::path::PathBuf {
        user_store_file(root, LEVEL_STORE).unwrap()
    }

    #[test]
    fn nothing_chosen_is_the_default_level() {
        assert_eq!(read_level(repo().path()), ControlLevel::default());
    }

    #[test]
    fn a_chosen_level_is_read_back_and_replaced_by_the_next() {
        let root = repo();
        write_level(root.path(), ControlLevel::ProjectEdits, "human:anh").unwrap();
        assert_eq!(read_level(root.path()), ControlLevel::ProjectEdits);
        write_level(root.path(), ControlLevel::Strict, "human:anh").unwrap();
        assert_eq!(read_level(root.path()), ControlLevel::Strict);
    }

    #[test]
    fn each_repository_has_its_own_level() {
        let (first, second) = (repo(), repo());
        write_level(first.path(), ControlLevel::SafeCommands, "human:anh").unwrap();
        assert_eq!(read_level(second.path()), ControlLevel::default());
    }

    #[test]
    fn the_record_is_outside_the_repository() {
        let root = repo();
        write_level(root.path(), ControlLevel::Strict, "human:anh").unwrap();
        assert!(!record_path(root.path()).starts_with(root.path().canonicalize().unwrap()));
    }

    #[test]
    fn a_record_that_cannot_be_trusted_means_the_strictest_level() {
        let root = repo();
        write_level(root.path(), ControlLevel::Sandboxed, "human:anh").unwrap();
        let path = record_path(root.path());
        let valid = fs::read_to_string(&path).unwrap();
        let bad: [(&str, String); 5] = [
            ("not json", "garbage".into()),
            ("unknown level", valid.replace("sandboxed", "turbo")),
            ("unknown field", valid.replace("\"level\"", "\"extra\": 1, \"level\"")),
            ("empty", String::new()),
            ("oversized", "x".repeat(MAX_RECORD_BYTES as usize + 1)),
        ];
        for (name, contents) in bad {
            fs::write(&path, contents).unwrap();
            assert_eq!(read_level(root.path()), ControlLevel::Strict, "{name}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_record_is_not_followed() {
        let root = repo();
        write_level(root.path(), ControlLevel::Sandboxed, "human:anh").unwrap();
        let path = record_path(root.path());
        let target = root.path().join("elsewhere.json");
        fs::copy(&path, &target).unwrap();
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert_eq!(read_level(root.path()), ControlLevel::Strict);
    }

    #[test]
    fn who_set_the_level_must_be_a_plain_name() {
        let root = repo();
        for bad in ["", "   ", "human\nanh", "a\0b"] {
            assert!(write_level(root.path(), ControlLevel::Strict, bad).is_err(), "{bad:?}");
        }
        assert_eq!(read_level(root.path()), ControlLevel::default(), "a refused write leaves nothing");
    }
}
