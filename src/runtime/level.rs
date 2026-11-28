//! Control levels: how much the agent may do without asking.
//!
//! A level only ever changes what has to be asked. It never touches the
//! floors (Giám Thị HALT, the capability table, protected configuration, what
//! `level_rules` calls dangerous): those are checked first and win at every
//! level. The verdict is a pure function of the level and the facts of one
//! call, so each level can be tested, and later replayed over old receipts.

use super::level_rules::{dangerous_command, is_opaque_command, is_safe_command, is_sensitive_path};
use crate::capability::file_mutation::canonical_write_target;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// From the strictest to the most autonomous. The order is meaningful: a
/// higher level approves everything a lower one does, and more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ControlLevel {
    /// 1. Everything that writes or reaches out is asked, every time.
    Strict,
    /// 2. As 1, and an answer the person chose to remember is honored.
    #[default]
    Remember,
    /// 3. Also edits ordinary files inside the project.
    ProjectEdits,
    /// 4. Also runs the short list of safe commands (status, build, test), where isolated.
    SafeCommands,
    /// 5. Also runs other commands, where isolated.
    Sandboxed,
}

impl ControlLevel {
    pub(crate) fn number(self) -> u8 {
        self as u8 + 1
    }

    pub(crate) fn from_number(number: u8) -> Option<Self> {
        match number {
            1 => Some(Self::Strict),
            2 => Some(Self::Remember),
            3 => Some(Self::ProjectEdits),
            4 => Some(Self::SafeCommands),
            5 => Some(Self::Sandboxed),
            _ => None,
        }
    }
}

/// What a level needs to know about one call.
pub(crate) struct CallFacts<'a> {
    /// The canonical capability name, such as `file.write` or `command.execute`.
    pub capability: &'a str,
    /// The command text, for `command.execute`.
    pub command: Option<&'a str>,
    /// The requested path, for a file write or patch.
    pub path: Option<&'a str>,
    pub root: &'a Path,
    pub sandboxed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LevelVerdict {
    /// The level does not approve this call: the usual approval applies.
    Ask,
    /// The level approves this call without asking.
    Allow,
}

/// What `level` decides for one call. Never `Allow` for anything dangerous.
pub(crate) fn verdict(level: ControlLevel, facts: &CallFacts) -> LevelVerdict {
    if level < ControlLevel::ProjectEdits {
        return LevelVerdict::Ask;
    }
    let approved = match facts.capability {
        "file.write" | "file.patch" => ordinary_project_file(facts),
        "command.execute" => command_is_approved(level, facts),
        _ => false,
    };
    if approved {
        LevelVerdict::Allow
    } else {
        LevelVerdict::Ask
    }
}

/// A short reason `level` did not approve a call, for the person who finds it
/// waiting. Only meaningful for a call `verdict` answered `Ask` to.
pub(crate) fn why_not(facts: &CallFacts) -> String {
    if let Some(command) = facts.command {
        if let Some(reason) = dangerous_command(command) {
            return reason;
        }
        if is_opaque_command(command) {
            return "a shell or interpreter runs whatever it is given".to_string();
        }
        if !facts.sandboxed {
            return "commands are not isolated, so none runs without the person".to_string();
        }
    }
    match facts.capability {
        "file.write" | "file.patch" => {
            "it writes a file the away mode does not cover (project structure, a secret, or a path that cannot be verified)"
        }
        "command.execute" => "the command is not on the short list the away mode runs on its own",
        _ => "this kind of action always needs the person",
    }
    .to_string()
}

/// What a grant the person gave their own app may cover without asking: a command
/// on the short safe list that stays inside the project, or an ordinary file of
/// the project. Everything else is asked, whatever the grant lists.
pub(super) fn app_grant_may_cover(facts: &CallFacts) -> bool {
    match facts.capability {
        "command.execute" => facts.command.is_some_and(|command| {
            dangerous_command(command).is_none()
                && !is_opaque_command(command)
                && is_safe_command(command)
                && stays_in_project(command, facts.root)
        }),
        "file.write" | "file.patch" => ordinary_project_file(facts),
        _ => false,
    }
}

/// Beginnings of options that write a file, load configuration, or name a program to run.
const REACHING_OPTIONS: &[&str] = &[
    "--ou", "-o", "--config", "-c", "-C", "-Z", "--exec", "-exec", "--toolexec", "-toolexec", "--ext-diff",
    "--textconv", "--pre", "--hostname-bin", "--no-index", "+", "--o=",
    // Follow links out of the folder, or read a file named inside the option.
    "-f", "--follow", "--dereference",
    // Carry flags, a program or an output file for the tool underneath.
    "-ldflags", "--ldflags", "-gccgoflags", "-gcflags", "-asmflags", "-vettool", "--vettool", "--emit", "--codegen",
    "--node-options", "--script-shell", "-coverprofile", "--coverprofile", "-memprofile", "-cpuprofile",
    "-blockprofile", "-mutexprofile", "-trace", "-outputdir",
];

/// The program is found by name (never a path, which the folder could supply), no
/// option reaches elsewhere, and every path given is inside the project.
fn stays_in_project(command: &str, root: &Path) -> bool {
    let Ok(tokens) = crate::capability::command::tokenize_command(command) else { return false };
    let Some((program, arguments)) = tokens.split_first() else { return false };
    let Ok(root) = root.canonicalize() else { return false };
    let inside = |value: &str| {
        let path = Path::new(value);
        if path.is_absolute() || value.starts_with('~') || path.components().any(|part| part == std::path::Component::ParentDir) {
            return false;
        }
        // Judged where it really is: the nearest part that exists must be in the project.
        let full = root.join(path);
        let existing = full.ancestors().find(|part| part.symlink_metadata().is_ok());
        existing.is_some_and(|part| part.canonicalize().is_ok_and(|real| real.starts_with(&root)))
    };
    // A value that could itself hold flags or a second path is not judged at all.
    let plain = |value: &str| !value.starts_with('-') && !value.contains(['=', ' ', '\t', '\n']);
    !program.contains(['/', '\\'])
        && arguments.iter().all(|argument| {
            if REACHING_OPTIONS.iter().any(|option| argument.starts_with(option)) {
                return false;
            }
            let Some(option) = argument.strip_prefix('-') else { return inside(argument) };
            if let Some((_, value)) = option.split_once('=') {
                // An option's value may be written to: structure files are asked.
                return plain(value) && inside(value) && !is_sensitive_path(Path::new(value));
            }
            // Short options bundled together must not hide one that follows links or names a file.
            let bundle = !option.starts_with('-') && option.contains(['R', 'L', 'f', 'o']);
            !argument.contains('/') && !bundle
        })
}

/// An existing directory inside the project, a file that is not sensitive.
/// The path is judged where it really is: a symlink into `.github` is `.github`.
fn ordinary_project_file(facts: &CallFacts) -> bool {
    let Some(requested) = facts.path else { return false };
    let Some(target) = canonical_write_target(facts.root, requested) else { return false };
    let Ok(root) = facts.root.canonicalize() else { return false };
    target.strip_prefix(&root).is_ok_and(|relative| !is_sensitive_path(relative))
}

fn command_is_approved(level: ControlLevel, facts: &CallFacts) -> bool {
    let Some(command) = facts.command else { return false };
    // A "safe" build or test still runs the folder's own code, so every command needs isolation.
    if !facts.sandboxed || level < ControlLevel::SafeCommands || dangerous_command(command).is_some() || is_opaque_command(command) {
        return false;
    }
    is_safe_command(command) || level == ControlLevel::Sandboxed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for dir in ["src", ".github/workflows", "docs"] {
            fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        root
    }

    fn write<'a>(root: &'a Path, path: &'a str) -> CallFacts<'a> {
        CallFacts { capability: "file.write", command: None, path: Some(path), root, sandboxed: false }
    }

    fn run<'a>(root: &'a Path, command: &'a str, sandboxed: bool) -> CallFacts<'a> {
        CallFacts { capability: "command.execute", command: Some(command), path: None, root, sandboxed }
    }

    #[test]
    fn levels_are_numbered_ordered_and_default_to_remember() {
        for number in 1..=5 {
            assert_eq!(ControlLevel::from_number(number).unwrap().number(), number);
        }
        assert!(ControlLevel::from_number(0).is_none() && ControlLevel::from_number(6).is_none());
        assert!(ControlLevel::Strict < ControlLevel::Remember && ControlLevel::SafeCommands < ControlLevel::Sandboxed);
        assert_eq!(ControlLevel::default(), ControlLevel::Remember);
        let json = serde_json::to_string(&ControlLevel::SafeCommands).unwrap();
        assert_eq!(json, "\"safe_commands\"");
        assert!(serde_json::from_str::<ControlLevel>("\"turbo\"").is_err());
    }

    #[test]
    fn the_two_lowest_levels_approve_nothing() {
        let root = project();
        for level in [ControlLevel::Strict, ControlLevel::Remember] {
            assert_eq!(verdict(level, &write(root.path(), "src/a.rs")), LevelVerdict::Ask);
            assert_eq!(verdict(level, &run(root.path(), "cargo test", true)), LevelVerdict::Ask);
        }
    }

    #[test]
    fn project_edits_approve_ordinary_files_only() {
        let root = project();
        let level = ControlLevel::ProjectEdits;
        assert_eq!(verdict(level, &write(root.path(), "src/a.rs")), LevelVerdict::Allow);
        assert_eq!(verdict(level, &write(root.path(), "docs/guide.md")), LevelVerdict::Allow);
        for sensitive in [".github/workflows/ci.yml", "Cargo.toml", ".env", "src/../.github/x.yml", ".yana-ai/leases.json"] {
            assert_eq!(verdict(level, &write(root.path(), sensitive)), LevelVerdict::Ask, "{sensitive}");
        }
        for unreachable in ["../outside.rs", "nowhere/a.rs", "", "/etc/passwd"] {
            assert_eq!(verdict(level, &write(root.path(), unreachable)), LevelVerdict::Ask, "{unreachable:?}");
        }
        let no_path = CallFacts { capability: "file.write", command: None, path: None, root: root.path(), sandboxed: false };
        assert_eq!(verdict(level, &no_path), LevelVerdict::Ask);
        assert_eq!(verdict(level, &run(root.path(), "cargo test", true)), LevelVerdict::Ask, "commands start at level 4");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_into_a_sensitive_directory_is_judged_by_where_it_leads() {
        let root = project();
        std::os::unix::fs::symlink(root.path().join(".github"), root.path().join("innocent")).unwrap();
        let level = ControlLevel::Sandboxed;
        assert_eq!(verdict(level, &write(root.path(), "innocent/x.yml")), LevelVerdict::Ask);
    }

    #[test]
    fn safe_commands_approve_the_short_list_and_nothing_dangerous() {
        let root = project();
        let level = ControlLevel::SafeCommands;
        for command in ["git status", "cargo test", "npm test", "ls -la"] {
            assert_eq!(verdict(level, &run(root.path(), command, true)), LevelVerdict::Allow, "{command}");
        }
        for command in ["cargo run", "make", "git push", "rm -rf x", "bash -c ls", "npm install", "python3 x.py", ""] {
            assert_eq!(verdict(level, &run(root.path(), command, true)), LevelVerdict::Ask, "{command:?}");
        }
        assert_eq!(verdict(level, &write(root.path(), "src/a.rs")), LevelVerdict::Allow, "edits stay approved");
    }

    #[test]
    fn the_top_level_runs_other_commands_only_inside_a_sandbox() {
        let root = project();
        let level = ControlLevel::Sandboxed;
        assert_eq!(verdict(level, &run(root.path(), "cargo run", false)), LevelVerdict::Ask);
        assert_eq!(verdict(level, &run(root.path(), "cargo run", true)), LevelVerdict::Allow);
        for command in ["rm -rf x", "git push", "npm install x", "bash -c ls", "python3 x.py", "curl https://x", "sudo ls"] {
            assert_eq!(verdict(level, &run(root.path(), command, true)), LevelVerdict::Ask, "{command} is never approved");
        }
    }

    #[test]
    fn no_command_runs_on_its_own_without_isolation() {
        let root = project();
        for level in [ControlLevel::SafeCommands, ControlLevel::Sandboxed] {
            for command in ["git status", "cargo test", "ls -la", "cargo run"] {
                assert_eq!(verdict(level, &run(root.path(), command, false)), LevelVerdict::Ask, "{level:?} {command}");
            }
        }
        assert!(why_not(&run(root.path(), "cargo test", false)).contains("isolat"));
    }

    #[test]
    fn other_capabilities_are_asked_at_every_level() {
        let root = project();
        for capability in ["web.search", "browser.fetch", "mcp.call", "config.write", "lsp.query", "unknown.thing"] {
            let facts = CallFacts { capability, command: Some("cargo test"), path: Some("src/a.rs"), root: root.path(), sandboxed: true };
            assert_eq!(verdict(ControlLevel::Sandboxed, &facts), LevelVerdict::Ask, "{capability}");
        }
    }
}
