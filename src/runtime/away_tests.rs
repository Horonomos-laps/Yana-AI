use super::*;
use crate::runtime::away_parked::MAX_PARKED_ITEMS;
use std::sync::Arc;

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join(yana_rt::flock_v1::PROTOCOL_FILE);
    std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
    std::fs::write(&marker, yana_rt::flock_v1::PROTOCOL_VERSION).unwrap();
    std::fs::create_dir_all(root.path().join("src")).unwrap();
    root
}

fn run<'a>(root: &'a Path, command: &'a str) -> CallFacts<'a> {
    CallFacts { capability: "command.execute", command: Some(command), path: None, root, sandboxed: false }
}

fn edit<'a>(root: &'a Path, path: &'a str) -> CallFacts<'a> {
    CallFacts { capability: "file.write", command: None, path: Some(path), root, sandboxed: false }
}

fn begin(root: &Path, budget: u32, level: ControlLevel) -> AwayRecord {
    start(root, 60, budget, level, "human:anh").unwrap()
}

fn approved(outcome: &AwayOutcome) -> bool {
    matches!(outcome, AwayOutcome::Approved { .. })
}

#[test]
fn the_box_is_validated_before_it_starts() {
    let root = project();
    let level = ControlLevel::SafeCommands;
    for (minutes, budget, level, who) in [
        (4, 10, level, "human:anh"),
        (0, 10, level, "human:anh"),
        (-5, 10, level, "human:anh"),
        (24 * 60 + 1, 10, level, "human:anh"),
        (60, 0, level, "human:anh"),
        (60, MAX_AWAY_BUDGET + 1, level, "human:anh"),
        (60, 10, ControlLevel::Strict, "human:anh"),
        (60, 10, ControlLevel::Remember, "human:anh"),
        (60, 10, level, ""),
        (60, 10, level, "a\nb"),
    ] {
        assert!(start(root.path(), minutes, budget, level, who).is_err(), "{minutes} {budget} {level:?} {who:?}");
    }
    assert!(read_record(root.path()).unwrap().is_none(), "a refused start leaves nothing");
    for edge in [(MIN_AWAY_MINUTES, 1), (MAX_AWAY_MINUTES, MAX_AWAY_BUDGET)] {
        stop(root.path()).unwrap();
        assert!(start(root.path(), edge.0, edge.1, level, "human:anh").is_ok(), "{edge:?}");
    }
}

#[test]
fn a_second_box_cannot_start_over_an_active_one_but_can_after_a_stop() {
    let root = project();
    begin(root.path(), 5, ControlLevel::SafeCommands);
    assert!(start(root.path(), 60, 5, ControlLevel::SafeCommands, "human:anh").is_err());
    assert!(stop(root.path()).unwrap().is_some());
    assert!(stop(root.path()).unwrap().is_none(), "stopping twice is harmless");
    assert!(start(root.path(), 60, 5, ControlLevel::SafeCommands, "human:anh").is_ok());
}

#[test]
fn without_a_box_nothing_is_decided() {
    let root = project();
    assert_eq!(judge(root.path(), &run(root.path(), "cargo test"), "s1"), AwayOutcome::NotAway);
    assert!(parked_items(root.path()).unwrap().is_empty());
}

#[test]
fn inside_the_box_it_approves_what_the_level_covers_and_counts_it() {
    let root = project();
    begin(root.path(), 3, ControlLevel::SafeCommands);
    assert!(approved(&judge(root.path(), &run(root.path(), "cargo test"), "s1")));
    assert!(approved(&judge(root.path(), &edit(root.path(), "src/a.rs"), "s1")));
    assert_eq!(read_record(root.path()).unwrap().unwrap().used, 2);
}

#[test]
fn what_is_dangerous_is_parked_never_approved_and_never_refused() {
    let root = project();
    begin(root.path(), 100, ControlLevel::Sandboxed);
    for command in ["rm -rf build", "git push origin main", "npm install x", "bash -c ls", "yana-rt level set 5"] {
        let outcome = judge(root.path(), &run(root.path(), command), "s1");
        assert!(matches!(outcome, AwayOutcome::Parked { .. }), "{command}: {outcome:?}");
    }
    assert!(matches!(judge(root.path(), &edit(root.path(), ".github/workflows/ci.yml"), "s1"), AwayOutcome::Parked { .. }));
    assert_eq!(read_record(root.path()).unwrap().unwrap().used, 0, "parking spends nothing");
    let items = parked_items(root.path()).unwrap();
    assert_eq!(items.len(), 6);
    assert_eq!((items[0].capability.as_str(), items[0].summary.as_str()), ("command.execute", "rm -rf build"));
    assert!(items[0].reason.contains("deletes") || items[0].reason.contains("rm"), "{}", items[0].reason);
    assert!(items[0].session_id == "s1" && !items[0].id.is_empty());
}

#[test]
fn a_level_covers_only_its_own_actions() {
    let root = project();
    begin(root.path(), 100, ControlLevel::ProjectEdits);
    assert!(approved(&judge(root.path(), &edit(root.path(), "src/a.rs"), "s1")));
    let command = judge(root.path(), &run(root.path(), "cargo test"), "s1");
    assert!(matches!(command, AwayOutcome::Parked { .. }), "level 3 does not run commands: {command:?}");
}

#[test]
fn the_budget_is_a_hard_stop() {
    let root = project();
    begin(root.path(), 2, ControlLevel::SafeCommands);
    assert!(approved(&judge(root.path(), &run(root.path(), "cargo test"), "s1")));
    assert!(approved(&judge(root.path(), &run(root.path(), "cargo build"), "s1")));
    let third = judge(root.path(), &run(root.path(), "cargo check"), "s1");
    assert!(matches!(&third, AwayOutcome::Parked { reason } if reason.contains("budget")), "{third:?}");
    assert_eq!(read_record(root.path()).unwrap().unwrap().used, 2);
}

#[test]
fn an_expired_or_stopped_box_decides_nothing() {
    let root = project();
    let mut record = begin(root.path(), 5, ControlLevel::SafeCommands);
    record.expires_at = Utc::now() - Duration::minutes(1);
    write_json(&away_file(root.path()).unwrap(), &record).unwrap();
    assert_eq!(judge(root.path(), &run(root.path(), "cargo test"), "s1"), AwayOutcome::NotAway);
    assert!(parked_items(root.path()).unwrap().is_empty());

    let root = project();
    begin(root.path(), 5, ControlLevel::SafeCommands);
    stop(root.path()).unwrap();
    assert_eq!(judge(root.path(), &run(root.path(), "cargo test"), "s1"), AwayOutcome::NotAway);
}

#[test]
fn a_record_that_cannot_be_read_means_ask_as_usual() {
    let root = project();
    begin(root.path(), 5, ControlLevel::SafeCommands);
    let path = away_file(root.path()).unwrap();
    for contents in ["garbage", "", "{\"level\":\"turbo\"}", "x".repeat(10_000_000).as_str()] {
        std::fs::write(&path, contents).unwrap();
        assert_eq!(judge(root.path(), &run(root.path(), "cargo test"), "s1"), AwayOutcome::NotAway);
    }
}

#[test]
fn the_budget_holds_when_many_calls_race_for_it() {
    let root = project();
    begin(root.path(), 3, ControlLevel::SafeCommands);
    let files = Arc::new(AwayFiles::for_root(root.path()).unwrap());
    let path = Arc::new(root.path().to_path_buf());
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let (files, path) = (Arc::clone(&files), Arc::clone(&path));
            std::thread::spawn(move || decide(&path, &files, &run(&path, "cargo test"), "s1").unwrap())
        })
        .collect();
    let outcomes: Vec<AwayOutcome> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|outcome| approved(outcome)).count(), 3, "{outcomes:?}");
    assert_eq!(read_record(root.path()).unwrap().unwrap().used, 3);
}

#[test]
fn the_queue_keeps_the_newest_items_and_describes_them_safely() {
    let root = project();
    begin(root.path(), 1, ControlLevel::SafeCommands);
    for index in 0..(MAX_PARKED_ITEMS + 5) {
        let command = format!("rm file-{index}");
        judge(root.path(), &run(root.path(), &command), "s1");
    }
    let items = parked_items(root.path()).unwrap();
    assert_eq!(items.len(), MAX_PARKED_ITEMS);
    assert_eq!(items.last().unwrap().summary, format!("rm file-{}", MAX_PARKED_ITEMS + 4));
    assert_eq!(items[0].summary, "rm file-5", "the oldest were dropped");

    clear_parked(root.path()).unwrap();
    let long = format!("rm {}\n\u{1b}[31mred", "a".repeat(500));
    judge(root.path(), &run(root.path(), &long), "s\n1");
    let item = &parked_items(root.path()).unwrap()[0];
    assert!(item.summary.chars().count() <= 200 && !item.summary.chars().any(char::is_control), "{:?}", item.summary);
    assert_eq!(item.session_id, "s1");
}

#[test]
fn clearing_the_queue_reports_how_many_were_waiting() {
    let root = project();
    begin(root.path(), 1, ControlLevel::SafeCommands);
    judge(root.path(), &run(root.path(), "rm a"), "s1");
    judge(root.path(), &run(root.path(), "rm b"), "s1");
    assert_eq!(clear_parked(root.path()).unwrap(), 2);
    assert_eq!(clear_parked(root.path()).unwrap(), 0);
}
