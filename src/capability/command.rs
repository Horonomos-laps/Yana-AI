//! Command validation and execution. Moved as-is from the original
//! single-file `capability/mod.rs` — bodies unchanged, error type changed
//! from `String` to `CapabilityError`.

use super::error::CapabilityError;
use super::MAX_COMMAND_OUTPUT_BYTES;
#[cfg(unix)]
use std::ffi::OsStr;
use std::{
    ffi::OsString,
    path::Path,
    process::{Command, Output},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCommand {
    pub argv: Vec<String>,
    pub guard_verdict: Option<&'static str>,
}

/// Most zero-width characters (combining marks) shown on one base character. More would
/// stack up and can spill over neighbouring rows in some terminals.
const MAX_COMBINING_MARKS: usize = 2;

/// `text` as a person is shown it in an approval: control and invisible format characters
/// become `?`, and a zero-width character is kept only right after a visible one and at
/// most `MAX_COMBINING_MARKS` times (so decomposed Vietnamese letters stay readable, while
/// a stack of marks, or marks with nothing to attach to, cannot hide or blur anything).
pub(crate) fn visible_line(text: &str) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut out = String::with_capacity(text.len());
    let mut marks = 0usize;
    for c in text.chars() {
        let bad = c.is_control() || crate::capability::untrusted::is_invisible_format_char(c);
        if bad {
            out.push('?');
            marks = 0;
        } else if c.width() == Some(0) {
            if out.is_empty() || marks >= MAX_COMBINING_MARKS || out.ends_with('?') {
                out.push('?');
            } else {
                out.push(c);
                marks += 1;
            }
        } else {
            out.push(c);
            marks = 0;
        }
    }
    out
}

/// The canonical command tokenizer — the one place `shell_words::split` is
/// called for a command about to be validated or matched against a lease
/// scope. `validate_command` uses it for real execution;
/// `capability::lease::command_matches` uses the exact same function so a
/// lease's `allow`/`deny` entries are compared against the same token
/// boundaries the command will actually be split on, not a second,
/// independently-written parser that could disagree with this one.
pub fn tokenize_command(command: &str) -> Result<Vec<String>, CapabilityError> {
    let argv = shell_words::split(command).map_err(|e| CapabilityError::CommandParseError {
        detail: e.to_string(),
    })?;
    if argv.is_empty() {
        return Err(CapabilityError::EmptyCommand);
    }
    Ok(argv)
}

pub fn validate_command(command: &str) -> Result<ValidatedCommand, CapabilityError> {
    let argv = tokenize_command(command)?;
    Ok(ValidatedCommand {
        argv,
        guard_verdict: crate::guard::check_command(command),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub truncated: bool,
}

/// Spawns `argv` and returns
/// the FULL, uncapped process output. Extracted out of `execute_command` so
/// a caller that must compute an exact statistic from output (a commit
/// count, a pass/fail count) before any byte truncation — see
/// `crate::compact` — doesn't inherit `execute_command`'s own 32KB cap.
/// Reusing `execute_command` for that purpose would reproduce the exact
/// class of bug `compact` exists to prevent: a count computed from output
/// that was already silently cut.
/// Set in the environment of every command the agent runs. `yana-rt trust allow`
/// refuses to run with it set: confirming a configuration is for a person. This is
/// defence in depth, not a barrier (a command can remove the variable); the barrier
/// is that a command needs a person's approval to run at all.
pub const AGENT_CHILD_ENV: &str = "YANA_AGENT_CHILD";

/// Nothing here isolates a command: it runs in the working folder with the rights
/// of the person's account. Levels that act without asking read this, so no
/// command runs unattended until real isolation exists. The folder being worked
/// in never supplies a wrapper script of its own.
pub const COMMANDS_ARE_ISOLATED: bool = false;

/// The program and PATH an approved command gets: a bare name is looked up, and
/// the child's PATH is limited, to directories outside the working folder, so the
/// folder cannot stand in for the program the approver read. A name written with a
/// path runs as written.
#[cfg(unix)]
fn outside_the_folder(program: &str, root: &Path, path: &OsStr) -> Result<(String, OsString), CapabilityError> {
    use super::program_path::is_bare_name;
    let home = std::env::var_os("HOME").and_then(|home| Path::new(&home).canonicalize().ok());
    // Without a PATH, the system's own directories.
    let path = if path.is_empty() { OsStr::new("/usr/bin:/bin") } else { path };
    let child = folder_safe_path(root, path, home.as_deref()).ok_or_else(|| CapabilityError::SpawnFailed {
        detail: "PATH has no directory outside the working folder".to_string(),
    })?;
    let program = if is_bare_name(program) {
        std::env::split_paths(&child)
            .map(|dir| dir.join(program))
            .find(|candidate| is_program_outside(candidate, root, home.as_deref()))
            .and_then(|found| found.to_str().map(str::to_string))
            .ok_or_else(|| CapabilityError::SpawnFailed {
                detail: format!("'{program}' was not found in PATH outside the working folder; give its path"),
            })?
    } else {
        program.to_string()
    };
    Ok((program, child))
}

/// A person's home or the whole disk is not a downloaded project: treating it as
/// one would hide every installed tool.
#[cfg(unix)]
fn is_not_a_project(root: &Path, home: Option<&Path>) -> bool {
    root.parent().is_none() || Some(root) == home
}

#[cfg(unix)]
fn folder_safe_path(root: &Path, path: &OsStr, home: Option<&Path>) -> Option<OsString> {
    let root = root.canonicalize().ok()?;
    if !is_not_a_project(&root, home.and_then(|home| home.canonicalize().ok()).as_deref()) {
        return super::program_path::safe_path_var(path, &root);
    }
    let kept: Vec<_> = std::env::split_paths(path).filter(|dir| dir.is_absolute() && dir.is_dir()).collect();
    std::env::join_paths(kept).ok().filter(|joined| !joined.is_empty())
}

/// An executable that, through any link, does not lead back into the project.
#[cfg(unix)]
fn is_program_outside(candidate: &Path, root: &Path, home: Option<&Path>) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(candidate) else { return false };
    if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
        return false;
    }
    if is_not_a_project(root, home) {
        return true;
    }
    let Some(project) = super::program_path::repository_bounds(root) else { return false };
    candidate.canonicalize().is_ok_and(|real| !real.starts_with(project))
}

pub fn spawn_command(root: &Path, argv: &[String]) -> Result<Output, CapabilityError> {
    run(root, argv, false)
}

fn run(root: &Path, argv: &[String], approved: bool) -> Result<Output, CapabilityError> {
    if argv.is_empty() {
        return Err(CapabilityError::EmptyCommand);
    }
    let root = root.canonicalize().map_err(|error| CapabilityError::Io {
        detail: format!("resolve command root: {error}"),
    })?;
    if !root.is_dir() {
        return Err(CapabilityError::NotADirectory {
            requested: root.display().to_string(),
        });
    }
    #[allow(unused_mut)]
    let (mut program, mut path) = (argv[0].clone(), None::<OsString>);
    #[cfg(unix)]
    if approved {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let (found, child) = outside_the_folder(&argv[0], &root, &inherited)?;
        (program, path) = (found, Some(child));
    }
    #[cfg(not(unix))]
    let _ = approved;
    let mut command = Command::new(program);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command
        .args(&argv[1..])
        .current_dir(root)
        .env(AGENT_CHILD_ENV, "1")
        .output()
        .map_err(|error| CapabilityError::SpawnFailed {
            detail: error.to_string(),
        })
}

pub fn execute_command(root: &Path, argv: &[String]) -> Result<CommandOutcome, CapabilityError> {
    Ok(cap_command_output(run(root, argv, true)?))
}

fn cap_command_output(output: Output) -> CommandOutcome {
    let (stdout, stdout_truncated) = cap_command_bytes(&output.stdout);
    let (stderr, stderr_truncated) = cap_command_bytes(&output.stderr);
    CommandOutcome {
        stdout,
        stderr,
        exit_code: output.status.code(),
        truncated: stdout_truncated || stderr_truncated,
    }
}

fn cap_command_bytes(bytes: &[u8]) -> (String, bool) {
    if bytes.len() > MAX_COMMAND_OUTPUT_BYTES {
        (
            String::from_utf8_lossy(&bytes[..MAX_COMMAND_OUTPUT_BYTES]).to_string(),
            true,
        )
    } else {
        (String::from_utf8_lossy(bytes).to_string(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tmp_repo(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("yana-capability-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn command_validation_allows_benign_command() {
        let benign = validate_command("git status --short").unwrap();
        assert_eq!(benign.argv, ["git", "status", "--short"]);
        assert!(benign.guard_verdict.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn an_approved_command_runs_in_the_folder_and_never_through_its_scripts() {
        let root = tmp_repo("planted");
        fs::create_dir_all(root.join("core/scripts")).unwrap();
        fs::write(root.join("core/scripts/sandbox-exec.sh"), "#!/bin/bash\ntouch planted\n").unwrap();
        let outcome = execute_command(&root, &["touch".into(), "made".into()]).unwrap();
        assert!(!root.join("planted").exists(), "a script from the folder ran");
        assert_eq!(outcome.exit_code, Some(0), "{}", outcome.stderr);
        assert!(root.join("made").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn the_working_folder_cannot_supply_the_program_an_approval_names() {
        use std::os::unix::fs::PermissionsExt;
        let root = tmp_repo("shadow");
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("bin/git"), "#!/bin/sh\n").unwrap();
        fs::set_permissions(root.join("bin/git"), fs::Permissions::from_mode(0o755)).unwrap();
        let path = std::env::join_paths([PathBuf::from("."), root.join("bin"), "/usr/bin".into(), "/bin".into()]).unwrap();
        let (program, child_path) = outside_the_folder("git", &root, &path).unwrap();
        assert_eq!(program, "/usr/bin/git");
        assert_eq!(std::env::split_paths(&child_path).collect::<Vec<_>>(), [PathBuf::from("/usr/bin"), "/bin".into()]);
        let (relative, _) = outside_the_folder("./bin/git", &root, &path).unwrap();
        assert_eq!(relative, "./bin/git", "a path the approver can read runs as written");
        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn a_chat_opened_at_home_or_the_disk_root_can_still_run_commands() {
        let home = tmp_repo("home");
        fs::create_dir_all(home.join("tools")).unwrap();
        let path = std::env::join_paths([PathBuf::from("."), home.join("tools"), "/usr/bin".into()]).unwrap();
        for (root, own_home) in [(home.clone(), home.as_path()), (PathBuf::from("/"), home.as_path())] {
            let kept = folder_safe_path(&root, &path, Some(own_home)).unwrap();
            assert_eq!(std::env::split_paths(&kept).collect::<Vec<_>>(), [home.join("tools"), "/usr/bin".into()], "{root:?}");
        }
        let (program, _) = outside_the_folder("/usr/bin/true", &home, std::ffi::OsStr::new("")).unwrap();
        assert_eq!(program, "/usr/bin/true", "an absolute program needs no PATH");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn command_validation_denies_rm_rf() {
        assert!(validate_command("rm -rf /tmp/x")
            .unwrap()
            .guard_verdict
            .is_some());
    }

    #[test]
    fn command_validation_denies_force_push() {
        assert!(validate_command("git push --force origin main")
            .unwrap()
            .guard_verdict
            .is_some());
    }

    #[test]
    fn command_validation_denies_inline_python_bypass() {
        assert!(
            validate_command("python3 -c \"import os; os.system('rm -rf /')\"")
                .unwrap()
                .guard_verdict
                .is_some()
        );
    }

    #[test]
    fn command_validation_rejects_empty_command() {
        assert!(matches!(
            validate_command(""),
            Err(CapabilityError::EmptyCommand)
        ));
    }

    #[test]
    fn command_validation_rejects_unbalanced_quotes() {
        assert!(matches!(
            validate_command("echo \"unterminated"),
            Err(CapabilityError::CommandParseError { .. })
        ));
    }

    #[test]
    fn command_execution_runs_direct_argv() {
        let root = tmp_repo("command-direct");
        let argv = ["echo".into(), "hello".into()];
        let outcome = execute_command(&root, &argv).unwrap();
        assert_eq!(outcome.stdout.trim(), "hello");
        assert_eq!(outcome.exit_code, Some(0));
        assert!(!outcome.truncated);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn command_execution_is_rooted_in_the_repository() {
        let root = tmp_repo("command");
        let init = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .unwrap();
        assert!(init.success());

        let argv = ["git".into(), "rev-parse".into(), "--show-toplevel".into()];
        let outcome = execute_command(&root, &argv).unwrap();
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(
            PathBuf::from(outcome.stdout.trim()).canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
        assert!(!outcome.truncated);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn command_execution_captures_nonzero_exit_code() {
        let root = tmp_repo("command-nonzero");
        let init = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .unwrap();
        assert!(init.success());
        let failing = [
            "git".into(),
            "cat-file".into(),
            "-e".into(),
            "refs/heads/definitely-missing".into(),
        ];
        let outcome = execute_command(&root, &failing).unwrap();
        assert_ne!(outcome.exit_code, Some(0));
        fs::remove_dir_all(root).ok();
    }
}
