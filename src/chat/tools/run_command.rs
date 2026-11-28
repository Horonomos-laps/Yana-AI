//! Chat adapter for canonical command validation and execution.

use std::path::Path;

pub use crate::capability::{CommandOutcome as ExecOutcome, ValidatedCommand as Validated};

/// Pure, synchronous: parses the command into argv and asks
/// `crate::guard::check_command()` — the single source of judgment for
/// "is this destructive" (identical logic to
/// `core/hooks/guard-destructive.sh`) — never a second pattern list of
/// its own.
pub fn validate(command: &str) -> Result<Validated, String> {
    crate::capability::validate_command(command).map_err(Into::into)
}

/// Actually runs the command. Only ever called after (a) `validate()`
/// found `guard_verdict == None`, and (b) a human approved it — this function
/// does not re-check either condition itself. Direct argv exec, no `sh -c`
/// string-building (per `shell-sanitize-law.md`, same style as
/// `plugin.rs`/`guard/lock.rs`), and no isolation.
pub fn execute(repo_root: &Path, argv: &[String]) -> Result<ExecOutcome, String> {
    crate::capability::execute_command(repo_root, argv).map_err(Into::into)
}
