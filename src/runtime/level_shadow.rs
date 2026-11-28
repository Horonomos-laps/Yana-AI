//! Shadow mode for the levels that are not enforced yet.
//!
//! A level above `MAX_ENFORCED_LEVEL` does not approve anything. It only
//! writes down what it would have approved, so that the lines can be compared
//! with what the person actually answered. A call a level would approve and
//! the person then denied is a blind spot in that level's rules; a level is
//! raised into enforcement only after it has run without one.

use super::level::{verdict, CallFacts, ControlLevel, LevelVerdict};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

/// The highest level whose approvals are real. Raising it is the decision to
/// start enforcing the levels above, and should follow the shadow record.
pub(crate) const MAX_ENFORCED_LEVEL: ControlLevel = ControlLevel::Remember;

const SHADOW_LOG: &str = "level-shadow.jsonl";

/// Bytes of the digest kept: enough to tell calls apart, not to recover them.
const DIGEST_BYTES: usize = 8;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Judged {
    /// What the enforced part of the chosen level decides.
    pub enforced: LevelVerdict,
    /// The chosen level would approve this call, but its rules are not
    /// enforced yet: worth writing down.
    pub shadow_only: bool,
}

/// Judges one call at `level`, enforcing only up to `cap`.
pub(crate) fn judge(level: ControlLevel, cap: ControlLevel, facts: &CallFacts) -> Judged {
    let enforced = verdict(level.min(cap), facts);
    let wanted = verdict(level, facts);
    Judged { enforced, shadow_only: enforced == LevelVerdict::Ask && wanted == LevelVerdict::Allow }
}

/// A short digest of what the call acts on. The text itself is never written:
/// a command or a path can hold a secret.
fn subject_digest(facts: &CallFacts) -> String {
    let subject = facts.command.or(facts.path).unwrap_or("");
    Sha256::digest(subject.as_bytes()).iter().take(DIGEST_BYTES).map(|byte| format!("{byte:02x}")).collect()
}

/// Appends one line to `.yana-ai/level-shadow.jsonl`. Best effort: a log that
/// cannot be written must never change what happens to the call.
pub(crate) fn record_shadow(root: &Path, level: ControlLevel, facts: &CallFacts, decision_id: &str) {
    let line = serde_json::json!({
        "ts": Utc::now().to_rfc3339(),
        "level": level.number(),
        "capability": facts.capability,
        "would": "allow",
        "decision_id": decision_id,
        "subject": subject_digest(facts),
    });
    let directory = root.join(".yana-ai");
    let path = directory.join(SHADOW_LOG);
    let written = fs::create_dir_all(&directory)
        .and_then(|()| match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Err(std::io::Error::other("the shadow log is a symlink"))
            }
            _ => Ok(()),
        })
        .and_then(|()| fs::OpenOptions::new().create(true).append(true).open(&path))
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = written {
        eprintln!("[level] could not write the shadow record: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("src")).unwrap();
        root
    }

    fn run<'a>(root: &'a Path, command: &'a str, sandboxed: bool) -> CallFacts<'a> {
        CallFacts { capability: "command.execute", command: Some(command), path: None, root, sandboxed }
    }

    #[test]
    fn the_default_cap_enforces_nothing_above_remember() {
        let root = project();
        for level in [ControlLevel::SafeCommands, ControlLevel::Sandboxed] {
            let judged = judge(level, MAX_ENFORCED_LEVEL, &run(root.path(), "cargo test", true));
            assert_eq!(judged.enforced, LevelVerdict::Ask, "{level:?} must not approve yet");
            assert!(judged.shadow_only, "{level:?} must be recorded as what it would have done");
        }
        // Level 3 does not approve commands at all, so a command is nothing to record there.
        let command_at_three = judge(ControlLevel::ProjectEdits, MAX_ENFORCED_LEVEL, &run(root.path(), "cargo test", true));
        assert!(!command_at_three.shadow_only);
        let edit = CallFacts { capability: "file.write", command: None, path: Some("src/a.rs"), root: root.path(), sandboxed: false };
        let edit_at_three = judge(ControlLevel::ProjectEdits, MAX_ENFORCED_LEVEL, &edit);
        assert_eq!(edit_at_three.enforced, LevelVerdict::Ask);
        assert!(edit_at_three.shadow_only);
    }

    #[test]
    fn a_raised_cap_enforces_up_to_it_and_shadows_the_rest() {
        let root = project();
        let safe = run(root.path(), "cargo test", true);
        let sandbox_only = run(root.path(), "cargo run", true);
        // Enforced through level 4: the safe command is approved, the sandbox-only one is shadowed.
        let at_four = judge(ControlLevel::Sandboxed, ControlLevel::SafeCommands, &safe);
        assert_eq!((at_four.enforced, at_four.shadow_only), (LevelVerdict::Allow, false));
        let shadowed = judge(ControlLevel::Sandboxed, ControlLevel::SafeCommands, &sandbox_only);
        assert_eq!((shadowed.enforced, shadowed.shadow_only), (LevelVerdict::Ask, true));
    }

    #[test]
    fn a_call_no_level_would_approve_is_not_recorded() {
        let root = project();
        let judged = judge(ControlLevel::Sandboxed, ControlLevel::Remember, &run(root.path(), "rm -rf x", true));
        assert_eq!(judged, Judged { enforced: LevelVerdict::Ask, shadow_only: false });
    }

    #[test]
    fn the_shadow_line_names_the_call_without_holding_its_text() {
        let root = project();
        let secret = "cargo test --token hunter2-secret";
        record_shadow(root.path(), ControlLevel::SafeCommands, &run(root.path(), secret, false), "decision-1");
        record_shadow(root.path(), ControlLevel::SafeCommands, &run(root.path(), "cargo test", false), "decision-2");

        let log = std::fs::read_to_string(root.path().join(".yana-ai").join(SHADOW_LOG)).unwrap();
        assert_eq!(log.lines().count(), 2);
        assert!(!log.contains("hunter2") && !log.contains("--token"), "the command text must not be logged: {log}");
        let first: serde_json::Value = serde_json::from_str(log.lines().next().unwrap()).unwrap();
        assert_eq!(first["level"], 4);
        assert_eq!(first["capability"], "command.execute");
        assert_eq!(first["would"], "allow");
        assert_eq!(first["decision_id"], "decision-1");
        let second: serde_json::Value = serde_json::from_str(log.lines().nth(1).unwrap()).unwrap();
        assert_ne!(first["subject"], second["subject"], "different calls must have different digests");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_shadow_log_is_not_followed() {
        let root = project();
        let target = root.path().join("elsewhere.log");
        std::fs::create_dir_all(root.path().join(".yana-ai")).unwrap();
        std::os::unix::fs::symlink(&target, root.path().join(".yana-ai").join(SHADOW_LOG)).unwrap();
        record_shadow(root.path(), ControlLevel::SafeCommands, &run(root.path(), "cargo test", false), "d");
        assert!(!target.exists(), "nothing may be written through the symlink");
    }
}
