//! A human's answer to a paused tool call can be "this once", "for this
//! session" or "always". The first is the existing behavior and stores
//! nothing. The other two are recorded as a capability lease, so there is one
//! mechanism for delegated authority, not a second approvals list.
//!
//! What a remembered answer can never do:
//! - cover more than the exact command the person saw (`grant_exact`);
//! - outlive its lease: "always" is a long lease, revocable with
//!   `yana-rt lease revoke`, not a permanent entry;
//! - skip the checks that run before leases (HALT, availability) or the
//!   intent narrowing that runs after them (`authority::capability_decision`);
//! - widen a denial, or apply to any capability but `command.execute`, the
//!   only one whose call has command text a grant can be pinned to.
//!
//! A refusal to remember is never an error for the turn: the approval the
//! person gave still stands for this one call, and the caller reports that
//! it was not remembered.

use super::authority::command_text_for_lease;
use crate::capability::lease::{Lease, LeaseStore};
use crate::capability::manifest;
use crate::model::tool::ToolCall;
use serde::Deserialize;
use std::path::Path;

/// One working day. A session grant is also keyed to the session id, so this
/// is an upper bound, not the usual lifetime.
pub(crate) const SESSION_GRANT_MINUTES: u64 = 8 * 60;

/// Leases always expire by design, so "always" is a long lease the person can
/// revoke at any time rather than an entry that lives forever.
pub(crate) const ALWAYS_GRANT_MINUTES: u64 = 90 * 24 * 60;

/// Subject of every "always" grant. One workspace, one subject.
pub(crate) const WORKSPACE_SUBJECT: &str = "human-grant:workspace";

/// The only capability whose calls carry command text to pin a grant to.
const GRANTABLE_CAPABILITY: &str = "command.execute";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ApprovalScope {
    #[default]
    Once,
    Session,
    Always,
}

pub(crate) fn session_subject(session_id: &str) -> String {
    format!("human-grant:session:{session_id}")
}

#[derive(Debug)]
pub(crate) enum GrantOutcome {
    /// The person asked for "once" (or said nothing): nothing to remember.
    NotRequested,
    Granted(Lease),
    /// A longer scope was asked for and refused; the reason is for the person.
    Declined(String),
}

/// Records the person's answer when it asked for more than "once".
pub(crate) fn grant_for_decision(
    root: &Path,
    session_id: &str,
    call: &ToolCall,
    approved: bool,
    decided_by: &str,
    scope: ApprovalScope,
) -> GrantOutcome {
    let minutes = match scope {
        ApprovalScope::Once => return GrantOutcome::NotRequested,
        ApprovalScope::Session => SESSION_GRANT_MINUTES,
        ApprovalScope::Always => ALWAYS_GRANT_MINUTES,
    };
    if !approved {
        return GrantOutcome::Declined("only an approval can be remembered, not a denial".into());
    }
    if session_id.trim().is_empty() || session_id.chars().any(char::is_control) {
        return GrantOutcome::Declined("this session has no usable id to remember the answer under".into());
    }
    let registry = manifest();
    let Some(descriptor) = registry.get_by_tool_name(&call.name) else {
        return GrantOutcome::Declined(format!("'{}' is not a known tool", call.name));
    };
    if descriptor.name != GRANTABLE_CAPABILITY {
        return GrantOutcome::Declined(format!("'{}' can only be approved one call at a time", descriptor.name));
    }
    let Some(command) = command_text_for_lease(call) else {
        return GrantOutcome::Declined("the call has no command to remember".into());
    };
    let subject = match scope {
        ApprovalScope::Session => session_subject(session_id),
        _ => WORKSPACE_SUBJECT.to_string(),
    };
    match LeaseStore::for_root(root).grant_exact(
        subject,
        GRANTABLE_CAPABILITY.to_string(),
        &command,
        decided_by.to_string(),
        minutes,
    ) {
        Ok(lease) => GrantOutcome::Granted(lease),
        Err(error) => GrantOutcome::Declined(format!("could not remember this answer: {error}")),
    }
}

/// The lease id of a remembered answer covering this exact command, if any.
/// Needs command text on purpose: a lease checked against no command is
/// vacuously in scope, and a human grant must never cover a call with none.
pub(super) fn matching_human_grant(
    root: &Path,
    session_id: &str,
    capability: &str,
    command_text: Option<&str>,
) -> Option<String> {
    let command_text = command_text?;
    if capability != GRANTABLE_CAPABILITY {
        return None;
    }
    let store = LeaseStore::for_root(root);
    [session_subject(session_id), WORKSPACE_SUBJECT.to_string()]
        .iter()
        .find_map(|subject| {
            store
                .try_consume_matching(subject, capability, root, Some(command_text))
                .ok()
                .flatten()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yana-human-grant-test-{}", uuid::Uuid::new_v4()));
        let marker = dir.join(yana_rt::flock_v1::PROTOCOL_FILE);
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        fs::write(&marker, yana_rt::flock_v1::PROTOCOL_VERSION).unwrap();
        dir
    }

    fn run_command(command: &str) -> ToolCall {
        ToolCall {
            id: "call-1".into(),
            name: "run_command".into(),
            arguments_json: serde_json::json!({ "command": command }).to_string(),
        }
    }

    fn grant(root: &Path, session: &str, command: &str, approved: bool, scope: ApprovalScope) -> GrantOutcome {
        grant_for_decision(root, session, &run_command(command), approved, "human:anh", scope)
    }

    fn covered(root: &Path, session: &str, command: &str) -> bool {
        matching_human_grant(root, session, "command.execute", Some(command)).is_some()
    }

    #[test]
    fn a_session_grant_covers_that_command_in_that_session_only() {
        let root = temp_root();
        assert!(matches!(grant(&root, "s1", "cargo test", true, ApprovalScope::Session), GrantOutcome::Granted(_)));

        assert!(covered(&root, "s1", "cargo test"));
        assert!(!covered(&root, "s2", "cargo test"), "another session must not inherit it");
        assert!(!covered(&root, "s1", "cargo test --release"), "extra flags are a different command");
        assert!(!covered(&root, "s1", "cargo build"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_always_grant_covers_the_command_in_every_session() {
        let root = temp_root();
        assert!(matches!(grant(&root, "s1", "cargo fmt", true, ApprovalScope::Always), GrantOutcome::Granted(_)));
        assert!(covered(&root, "s1", "cargo fmt"));
        assert!(covered(&root, "s2", "cargo fmt"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn once_remembers_nothing() {
        let root = temp_root();
        assert!(matches!(grant(&root, "s1", "cargo test", true, ApprovalScope::Once), GrantOutcome::NotRequested));
        assert!(!covered(&root, "s1", "cargo test"));
        assert!(LeaseStore::for_root(&root).list().unwrap().is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_denial_is_never_remembered_even_when_a_scope_is_sent() {
        let root = temp_root();
        for scope in [ApprovalScope::Session, ApprovalScope::Always] {
            assert!(matches!(grant(&root, "s1", "rm -rf build", false, scope), GrantOutcome::Declined(_)));
        }
        assert!(LeaseStore::for_root(&root).list().unwrap().is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn grants_last_the_documented_time() {
        let root = temp_root();
        let GrantOutcome::Granted(session) = grant(&root, "s1", "a", true, ApprovalScope::Session) else {
            panic!("session grant refused");
        };
        let GrantOutcome::Granted(always) = grant(&root, "s1", "b", true, ApprovalScope::Always) else {
            panic!("always grant refused");
        };
        let minutes = |lease: &Lease| (lease.expires_at - lease.issued_at).num_minutes();
        assert_eq!(minutes(&session), SESSION_GRANT_MINUTES as i64);
        assert_eq!(minutes(&always), ALWAYS_GRANT_MINUTES as i64);
        assert_eq!(session.issued_by, "human:anh");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn only_command_execute_can_be_remembered() {
        let root = temp_root();
        let unknown = ToolCall { id: "c".into(), name: "no_such_tool".into(), arguments_json: "{}".into() };
        let no_command = ToolCall { id: "c".into(), name: "run_command".into(), arguments_json: "{}".into() };
        for call in [unknown, no_command] {
            let outcome = grant_for_decision(&root, "s1", &call, true, "human:anh", ApprovalScope::Always);
            assert!(matches!(outcome, GrantOutcome::Declined(_)), "{call:?}");
        }
        assert!(LeaseStore::for_root(&root).list().unwrap().is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_grant_never_covers_a_call_that_has_no_command_text() {
        let root = temp_root();
        grant(&root, "s1", "cargo test", true, ApprovalScope::Always);
        assert!(matching_human_grant(&root, "s1", "command.execute", None).is_none());
        assert!(matching_human_grant(&root, "s1", "patch.apply", Some("cargo test")).is_none());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn hostile_input_is_refused_and_leaves_nothing_behind() {
        let root = temp_root();
        let long = "a".repeat(65_536);
        let refused = [
            ("s1", ""),
            ("s1", "   "),
            ("s1", long.as_str()),
            ("s1", "git status\0"),
            ("s1", "git status\r\nwhoami"),
            ("", "cargo test"),
            ("s\n1", "cargo test"),
        ];
        for (session, command) in refused {
            let outcome = grant(&root, session, command, true, ApprovalScope::Session);
            assert!(matches!(outcome, GrantOutcome::Declined(_)), "{command:?} in {session:?}");
        }
        assert!(LeaseStore::for_root(&root).list().unwrap().is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_odd_but_parseable_command_stays_an_exact_match() {
        let root = temp_root();
        assert!(matches!(
            grant(&root, "s1", "cat ../../etc/passwd", true, ApprovalScope::Session),
            GrantOutcome::Granted(_)
        ));
        assert!(covered(&root, "s1", "cat ../../etc/passwd"));
        assert!(!covered(&root, "s1", "cat ../../etc/passwd /etc/shadow"));
        assert!(!covered(&root, "s1", "cat ../../etc"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_scope_field_reads_the_three_words_and_nothing_else() {
        let parse = |text: &str| serde_json::from_str::<ApprovalScope>(&format!("\"{text}\""));
        assert_eq!(parse("once").unwrap(), ApprovalScope::Once);
        assert_eq!(parse("session").unwrap(), ApprovalScope::Session);
        assert_eq!(parse("always").unwrap(), ApprovalScope::Always);
        assert!(parse("forever").is_err());
        assert!(parse("Session").is_err());
        assert!(parse("").is_err());
        assert_eq!(ApprovalScope::default(), ApprovalScope::Once);
    }
}
