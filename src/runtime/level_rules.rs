//! What counts as dangerous, and what counts as safe, for the control levels.
//!
//! Plain data and small pure functions. The lists are deliberately short and
//! named: a longer list is not a safer one, it is a harder one to review.
//!
//! "Dangerous" is the structure of the project and of the machine: deleting,
//! rewriting history, publishing, installing, changing who may do what. These
//! are never approved by a level, and never by the away mode either.

use crate::capability::command::tokenize_command;
use std::path::{Component, Path};

/// Programs that delete, move, change permissions or reach outside the
/// project. Matched on the program's file name, so `/bin/rm` counts too.
const DANGEROUS_PROGRAMS: &[&str] = &[
    "rm", "rmdir", "mv", "dd", "mkfs", "chmod", "chown", "sudo", "doas", "kill", "pkill", "launchctl",
    "systemctl", "gh", "curl", "wget", "ssh", "scp", "rsync", "docker", "kubectl", "terraform",
    // Reads and changes the Keychain, where provider keys are stored.
    "security",
];

/// Shells and interpreters run whatever they are handed, so the command text
/// says nothing about what will happen. Never approved by a level.
const OPAQUE_PROGRAMS: &[&str] = &[
    "sh", "bash", "zsh", "fish", "dash", "ksh", "env", "xargs", "eval", "exec", "nohup", "time", "timeout",
    "watch", "nice", "arch", "caffeinate", "stdbuf", "node", "deno", "bun", "ruby", "perl", "php", "lua", "osascript", "open",
];

/// Package managers and the subcommands that change what is installed or published.
const PACKAGE_CHANGES: &[(&str, &[&str])] = &[
    ("npm", &["install", "i", "add", "uninstall", "publish", "update", "ci"]),
    ("pnpm", &["install", "i", "add", "remove", "publish", "update"]),
    ("yarn", &["install", "add", "remove", "publish", "upgrade"]),
    ("pip", &["install", "uninstall"]),
    ("pip3", &["install", "uninstall"]),
    ("cargo", &["install", "publish", "add", "remove", "yank", "update"]),
    ("brew", &["install", "uninstall", "upgrade", "tap"]),
    ("go", &["install", "get"]),
];

/// Our own commands that change who may do what: the control level, leases,
/// the away mode and the authority records, plus `os` (which holds the stored
/// provider keys) and `chat` (which spends them). An agent never runs these for
/// itself at any level, or a level could be raised by the agent it limits.
const CONTROL_COMMANDS: &[(&str, &[&str])] = &[
    ("yana-rt", &["level", "lease", "away", "authority", "os", "chat"]),
    ("yana-ai", &["level", "lease", "away", "authority", "os", "chat"]),
    ("yana-ai-rt", &["level", "lease", "away", "authority", "os", "chat"]),
];

/// Git subcommands that rewrite history, discard work or reach a remote.
const DANGEROUS_GIT: &[&str] = &[
    "push", "reset", "clean", "rebase", "filter-branch", "checkout", "restore", "gc", "prune", "reflog",
    "update-ref", "remote", "config", "fetch", "pull", "clone", "submodule",
];

/// Commands a level may approve: a program and the one subcommand that is safe.
const SAFE_SUBCOMMANDS: &[(&str, &[&str])] = &[
    ("git", &["status", "diff", "log", "show", "rev-parse", "ls-files"]),
    ("cargo", &["build", "check", "test", "clippy", "fmt", "doc"]),
    ("swift", &["build", "test"]),
    ("npm", &["test"]),
    ("go", &["build", "test", "vet"]),
];

/// Read-only programs a level may approve with any arguments.
const SAFE_ANY_ARGS: &[&str] = &["ls", "pwd", "cat", "head", "tail", "wc", "grep", "rg", "echo", "which", "date"];

/// Directory names that are never auto-written wherever they appear.
const SENSITIVE_SEGMENTS: &[&str] = &[
    ".git", ".github", ".gitlab", ".claude", ".codex", ".cursor", ".yana-ai", ".ssh", ".aws", "migrations", "db",
];

/// Two directories in a row, for the places that decide what runs or who may do what.
const SENSITIVE_PAIRS: &[(&str, &str)] =
    &[("core", "hooks"), ("core", "gates"), ("core", "rules"), ("core", "scripts"), ("core", "config"), ("src", "guard")];

/// File names that change dependencies or the build, compared in lower case.
const SENSITIVE_FILES: &[&str] = &[
    "cargo.toml", "cargo.lock", "package.json", "package-lock.json", "pnpm-lock.yaml", "yarn.lock",
    "package.swift", "package.resolved", "dockerfile", "docker-compose.yml", "pyproject.toml",
    "requirements.txt", "go.mod", "go.sum", "gemfile", "gemfile.lock",
];

/// Extensions of files that hold keys or certificates.
const SECRET_EXTENSIONS: &[&str] = &["pem", "key", "p12", "pfx", "crt"];

fn program_name(program: &str) -> String {
    Path::new(program).file_name().map_or_else(|| program.to_string(), |name| name.to_string_lossy().into_owned())
}

/// The program, its first argument, and the rest, or `None` if the text does not parse.
fn split_command(command: &str) -> Option<(String, Option<String>, Vec<String>)> {
    let tokens = tokenize_command(command).ok()?;
    let (program, rest) = tokens.split_first()?;
    Some((program_name(program), rest.first().cloned(), rest.iter().skip(1).cloned().collect()))
}

/// Why this command is dangerous, or `None`. A command that does not parse is
/// dangerous: nothing can be said about what it does.
pub(crate) fn dangerous_command(command: &str) -> Option<String> {
    if let Some(reason) = crate::guard::check_command(command) {
        return Some(reason.to_string());
    }
    let Some((program, first, _)) = split_command(command) else {
        return Some("the command could not be parsed".into());
    };
    // A case-insensitive volume runs `SECURITY` too.
    let lower = program.to_ascii_lowercase();
    if DANGEROUS_PROGRAMS.contains(&lower.as_str()) {
        return Some(format!("'{program}' deletes, moves, changes permissions or reaches outside the project"));
    }
    if program == "git" && first.as_deref().is_some_and(|sub| DANGEROUS_GIT.contains(&sub)) {
        return Some("this git command rewrites history, discards work or reaches a remote".into());
    }
    let changes_packages = PACKAGE_CHANGES
        .iter()
        .any(|(tool, subs)| *tool == program && first.as_deref().is_some_and(|sub| subs.contains(&sub)));
    if changes_packages {
        return Some(format!("'{program}' would change what is installed or published"));
    }
    let changes_control = CONTROL_COMMANDS
        .iter()
        .any(|(tool, subs)| *tool == lower && first.as_deref().is_some_and(|sub| subs.contains(&sub)));
    changes_control.then(|| "this changes what the agent is allowed to do, which only the person decides".to_string())
}

/// `rg --pre` and `--hostname-bin` name a program for ripgrep to run.
fn rg_runs_a_program<'a>(program: &str, mut args: impl Iterator<Item = &'a String>) -> bool {
    program == "rg" && args.any(|arg| ["--pre", "--hostname-bin"].iter().any(|flag| arg == flag || arg.starts_with(&format!("{flag}="))))
}

/// True for a shell or interpreter, which no level approves.
pub(crate) fn is_opaque_command(command: &str) -> bool {
    split_command(command).is_some_and(|(program, first, rest)| {
        // `find -exec` runs whatever follows, like a shell does.
        let runs_others = program == "find"
            && first.iter().chain(rest.iter()).any(|arg| matches!(arg.as_str(), "-exec" | "-execdir" | "-ok" | "-okdir"));
        let rg_runs = rg_runs_a_program(&program, first.iter().chain(rest.iter()));
        runs_others || rg_runs || OPAQUE_PROGRAMS.contains(&program.as_str()) || program.starts_with("python")
    })
}

/// True for a command on the short list a level may approve.
pub(crate) fn is_safe_command(command: &str) -> bool {
    let Some((program, first, rest)) = split_command(command) else { return false };
    if program == "git" && rest.iter().chain(first.iter()).any(|arg| arg.starts_with("--output")) {
        return false;
    }
    if rg_runs_a_program(&program, first.iter().chain(rest.iter())) {
        return false;
    }
    if SAFE_ANY_ARGS.contains(&program.as_str()) {
        return true;
    }
    SAFE_SUBCOMMANDS
        .iter()
        .any(|(tool, subs)| *tool == program && first.as_deref().is_some_and(|sub| subs.contains(&sub)))
}

/// True when `relative` (a path inside the repository, already resolved to
/// where it really is) is one a level must never auto-write.
pub(crate) fn is_sensitive_path(relative: &Path) -> bool {
    let parts: Vec<String> = relative
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect();
    let Some(file) = parts.last() else { return true };
    let in_sensitive_dir = parts.iter().any(|part| SENSITIVE_SEGMENTS.contains(&part.as_str()));
    let in_sensitive_pair = parts
        .windows(2)
        .any(|pair| SENSITIVE_PAIRS.iter().any(|(first, second)| pair[0] == *first && pair[1] == *second));
    let sensitive_name = SENSITIVE_FILES.contains(&file.as_str()) || file.starts_with(".env");
    let secret_extension = Path::new(file)
        .extension()
        .is_some_and(|extension| SECRET_EXTENSIONS.contains(&extension.to_string_lossy().as_ref()));
    in_sensitive_dir || in_sensitive_pair || sensitive_name || secret_extension
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_commands_are_named() {
        for command in [
            "rm -rf build", "rm file", "/bin/rm file", "mv a b", "sudo ls", "chmod 600 key", "curl https://x",
            "gh pr merge 1", "docker run x", "git push origin main", "git reset --hard", "git clean -fd",
            "git checkout -- .", "git rebase main", "npm install left-pad", "npm publish", "cargo install x",
            "cargo publish", "pip install x", "brew install x", "dd if=/dev/zero of=/dev/disk2",
            "yana-rt level set 5", "yana-rt lease grant --subject x", "/usr/local/bin/yana-rt away start", "yana-ai authority receipts",
            "bad \"quote", "",
        ] {
            assert!(dangerous_command(command).is_some(), "{command:?} must be dangerous");
        }
    }

    #[test]
    fn ordinary_development_commands_are_not_dangerous() {
        for command in [
            "git status", "git diff --stat", "git log -5", "cargo test", "cargo build --release", "npm test", "ls -la",
            "rg foo src", "yana-rt --version", "yana-rt doctor",
        ] {
            assert!(dangerous_command(command).is_none(), "{command:?} must not be dangerous");
        }
    }

    #[test]
    fn commands_that_reach_stored_keys_are_never_approved_by_a_level() {
        for command in [
            "security find-generic-password -s yana-ai -a OPENAI_API_KEY -w", "/usr/bin/security dump-keychain",
            "yana-rt os credential remove openai", "yana-rt os credential set openai", "yana-ai os credential status",
            "yana-rt chat --headless --provider openai", "SECURITY dump-keychain", "YANA-RT os credential remove openai",
            "yana-ai-rt chat --headless",
        ] {
            assert!(dangerous_command(command).is_some(), "{command:?} must be dangerous");
        }
        for command in ["find . -exec yana-rt os credential remove openai ;", "find . -execdir sh -c x ;", "find . -ok rm x ;", "nice security dump-keychain", "caffeinate yana-rt os status"] {
            assert!(is_opaque_command(command) || dangerous_command(command).is_some(), "{command:?} must not be approved");
        }
        assert!(!is_opaque_command("find . -name main.rs"));
    }

    #[test]
    fn ripgrep_options_that_run_a_program_are_not_approved() {
        for command in [
            "rg --pre cat x", "rg --pre=cat x", "rg foo --pre ./run.sh", "/usr/local/bin/rg --pre sh .",
            "rg --hostname-bin ./x y", "rg --hostname-bin=./x y",
        ] {
            assert!(!is_safe_command(command), "{command:?} must not be safe");
            assert!(is_opaque_command(command), "{command:?} must be opaque");
        }
        for command in ["rg foo src", "rg --pretty foo", "rg --pre-glob '*.pdf' foo", "rg -- --pre-like"] {
            assert!(is_safe_command(command), "{command:?} must stay safe");
        }
    }

    #[test]
    fn the_safe_list_is_short_and_exact() {
        for command in ["git status", "git diff", "git log --oneline", "cargo test", "cargo fmt", "npm test", "ls -la", "rg todo", "swift build"] {
            assert!(is_safe_command(command), "{command:?} must be safe");
        }
        for command in [
            "git push", "git commit -m x", "git branch -D x", "git -c core.pager=x status", "git diff --output=/tmp/x",
            "cargo run", "cargo install x", "npm run build", "npm install", "make", "find . -delete", "rm x", "bash -c ls",
            "python3 x.py", "", "git",
        ] {
            assert!(!is_safe_command(command), "{command:?} must not be safe");
        }
    }

    #[test]
    fn shells_and_interpreters_are_opaque() {
        for command in ["bash -c ls", "/bin/sh x.sh", "python3 x.py", "python3.12 x.py", "env FOO=1 cargo test", "node x.js", "xargs rm"] {
            assert!(is_opaque_command(command), "{command:?} must be opaque");
        }
        assert!(!is_opaque_command("cargo test"));
        assert!(!is_opaque_command("ls"));
    }

    #[test]
    fn sensitive_paths_are_found_wherever_they_sit() {
        for path in [
            ".git/hooks/pre-commit", ".github/workflows/ci.yml", ".claude/settings.json", ".yana-ai/leases.json",
            "core/hooks/guard.sh", "src/guard/mod.rs", "Cargo.toml", "app/Cargo.lock", "package.json", "web/package-lock.json",
            ".env", ".env.production", "certs/server.pem", "keys/id.key", "db/schema.sql", "x/migrations/001.sql",
            "Dockerfile", "CARGO.TOML", ".GIT/config", "", ".",
        ] {
            assert!(is_sensitive_path(Path::new(path)), "{path:?} must be sensitive");
        }
    }

    #[test]
    fn ordinary_source_files_are_not_sensitive() {
        for path in ["src/main.rs", "src/runtime/level.rs", "README.md", "docs/guide.md", "app/Sources/Chat.swift", "core/skills/x/SKILL.md", "src/guardian.rs"] {
            assert!(!is_sensitive_path(Path::new(path)), "{path:?} must not be sensitive");
        }
    }
}
