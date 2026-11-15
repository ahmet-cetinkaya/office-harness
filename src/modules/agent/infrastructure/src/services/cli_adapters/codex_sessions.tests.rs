// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use tempfile::TempDir;

use super::*;

const QUICK: AttributionTiming = AttributionTiming {
    poll_interval: Duration::from_millis(5),
    timeout: Duration::from_millis(150),
    settle: Duration::from_millis(60),
};

/// A session id of the CLI's shape, told apart by `number`.
fn id(number: u8) -> String {
    format!("00000000-0000-0000-0000-{number:012x}")
}

struct Fixture {
    _root: TempDir,
    sessions: PathBuf,
    folder: PathBuf,
    pending: Arc<PendingStarts>,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().expect("root");
    let sessions = root.path().join("home").join(".codex").join("sessions");
    let folder = root.path().join("work");
    fs::create_dir_all(&sessions).expect("sessions");
    fs::create_dir_all(&folder).expect("folder");
    Fixture {
        _root: root,
        sessions,
        folder,
        pending: Arc::new(PendingStarts::default()),
    }
}

fn write_session(fixture: &Fixture, id: &str, folder: &Path) {
    let day = fixture.sessions.join("2026").join("11").join("15");
    fs::create_dir_all(&day).expect("day");
    let meta = format!(
        r#"{{"type":"session_meta","payload":{{"id":"{id}","cwd":{}}}}}"#,
        serde_json::to_string(&folder.to_string_lossy()).expect("json")
    );
    fs::write(day.join(format!("rollout-{id}.jsonl")), format!("{meta}\n")).expect("write");
}

fn begin(fixture: &Fixture) -> CodexTracking {
    CodexTracking::begin(
        fixture.sessions.clone(),
        &fixture.folder,
        QUICK,
        &fixture.pending,
    )
}

fn environment(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn the_sessions_folder_follows_codex_home_then_the_home_folder() {
    let both = environment(&[("HOME", "/h"), ("CODEX_HOME", "/c")]);
    let home_only = environment(&[("HOME", "/h")]);

    assert_eq!(sessions_folder(&both), Some(PathBuf::from("/c/sessions")));
    assert_eq!(
        sessions_folder(&home_only),
        Some(PathBuf::from("/h/.codex/sessions"))
    );
    assert_eq!(sessions_folder(&[]), None);
}

#[test]
fn the_one_new_session_of_the_folder_is_found() {
    let fixture = fixture();
    write_session(&fixture, &id(1), &fixture.folder);
    let mut tracking = begin(&fixture);
    write_session(&fixture, &id(2), &fixture.folder);
    write_session(&fixture, &id(3), Path::new("/some/other/folder"));

    assert_eq!(tracking.resolve(), TrackedSession::Found(id(2)));
}

#[test]
fn no_new_session_in_time_is_unmatched_with_a_reason() {
    let fixture = fixture();
    write_session(&fixture, &id(1), &fixture.folder);

    let result = begin(&fixture).resolve();

    assert!(
        matches!(result, TrackedSession::Unmatched(reason) if reason.contains("no new session"))
    );
}

#[test]
fn two_new_sessions_in_the_folder_are_never_guessed() {
    let fixture = fixture();
    let mut tracking = begin(&fixture);
    write_session(&fixture, &id(4), &fixture.folder);
    write_session(&fixture, &id(5), &fixture.folder);

    let result = tracking.resolve();

    assert!(
        matches!(result, TrackedSession::Unmatched(reason) if reason.contains("more than one"))
    );
}

#[test]
fn two_starts_that_overlap_in_one_folder_both_give_up_and_never_take_each_others_id() {
    let fixture = fixture();
    let mut first = begin(&fixture);
    let mut second = begin(&fixture);
    write_session(&fixture, &id(6), &fixture.folder);

    let results = [first.resolve(), second.resolve()];

    for result in results {
        assert!(
            matches!(&result, TrackedSession::Unmatched(reason) if reason.contains("same time")),
            "{result:?}"
        );
    }
}

#[test]
fn a_start_after_the_other_has_finished_gets_its_own_id_not_the_earlier_one() {
    let fixture = fixture();
    let mut first = begin(&fixture);
    write_session(&fixture, &id(6), &fixture.folder);
    assert_eq!(first.resolve(), TrackedSession::Found(id(6)));
    drop(first);

    let mut second = begin(&fixture);
    write_session(&fixture, &id(7), &fixture.folder);

    assert_eq!(second.resolve(), TrackedSession::Found(id(7)));
}

#[test]
fn a_start_in_another_folder_does_not_disturb_this_one() {
    let fixture = fixture();
    let other_folder = fixture.folder.with_file_name("other");
    fs::create_dir_all(&other_folder).expect("other folder");
    let mut first = begin(&fixture);
    let mut second = CodexTracking::begin(
        fixture.sessions.clone(),
        &other_folder,
        QUICK,
        &fixture.pending,
    );
    write_session(&fixture, &id(8), &fixture.folder);
    write_session(&fixture, &id(9), &other_folder);

    assert_eq!(first.resolve(), TrackedSession::Found(id(8)));
    assert_eq!(second.resolve(), TrackedSession::Found(id(9)));
}

#[test]
fn a_session_file_that_is_not_complete_yet_is_looked_at_again() {
    let fixture = fixture();
    let mut tracking = begin(&fixture);
    let day = fixture.sessions.join("2026").join("11").join("15");
    fs::create_dir_all(&day).expect("day");
    fs::write(day.join("rollout-late.jsonl"), "{\"type\":").expect("partial");

    let finish = thread::spawn({
        let file = day.join("rollout-late.jsonl");
        let folder = fixture.folder.clone();
        move || {
            thread::sleep(Duration::from_millis(30));
            let meta = format!(
                "{{\"payload\":{{\"id\":\"{}\",\"cwd\":{}}}}}\n",
                id(10),
                serde_json::to_string(&folder.to_string_lossy()).expect("json")
            );
            fs::write(file, meta).expect("complete");
        }
    });

    assert_eq!(tracking.resolve(), TrackedSession::Found(id(10)));
    finish.join().expect("writer");
}

#[test]
fn looking_for_a_session_changes_nothing_in_the_cli_folder() {
    let fixture = fixture();
    write_session(&fixture, &id(1), &fixture.folder);
    let before = listing(&fixture.sessions);

    let mut tracking = begin(&fixture);
    let _ = tracking.resolve();

    assert_eq!(listing(&fixture.sessions), before);
}

#[test]
fn a_second_new_session_inside_the_settle_time_makes_the_match_ambiguous() {
    let fixture = fixture();
    let mut tracking = begin(&fixture);
    write_session(&fixture, &id(1), &fixture.folder);

    let writer = thread::scope(|scope| {
        scope.spawn(|| {
            thread::sleep(Duration::from_millis(20));
            write_session(&fixture, &id(2), &fixture.folder);
        });
        tracking.resolve()
    });

    assert!(
        matches!(writer, TrackedSession::Unmatched(reason) if reason.contains("more than one"))
    );
}

#[test]
fn a_start_that_found_nothing_keeps_its_registration_so_a_late_file_is_not_taken() {
    let fixture = fixture();
    let first = begin(&fixture);
    let (sender, receiver) = std::sync::mpsc::channel();
    Box::new(first).follow(Box::new(move |tracked| {
        let _ = sender.send(tracked);
    }));
    let reported = receiver.recv().expect("report");
    assert!(matches!(reported, TrackedSession::Unmatched(_)));

    let mut second = begin(&fixture);
    write_session(&fixture, &id(1), &fixture.folder);

    assert!(matches!(
        second.resolve(),
        TrackedSession::Unmatched(reason) if reason.contains("same time")
    ));
}

#[test]
fn only_a_uuid_shaped_id_is_taken() {
    assert!(is_uuid(&id(1)));
    for text in [
        "",
        "mine",
        "00000000-0000-0000-0000-00000000000",
        "0000000g-0000-0000-0000-000000000000",
        "00000000_0000_0000_0000_000000000000",
    ] {
        assert!(!is_uuid(text), "{text}");
    }
    let fixture = fixture();
    let mut tracking = begin(&fixture);
    write_session(&fixture, "not-an-id", &fixture.folder);

    assert!(
        matches!(tracking.resolve(), TrackedSession::Unmatched(reason) if reason.contains("no new session"))
    );
}

#[test]
fn a_first_line_beyond_the_cap_is_rejected_for_good_without_reading_it_all() {
    let fixture = fixture();
    let mut tracking = begin(&fixture);
    let day = fixture.sessions.join("2026").join("11").join("15");
    fs::create_dir_all(&day).expect("day");
    let huge = day.join("rollout-huge.jsonl");
    fs::write(
        &huge,
        vec![b'x'; usize::try_from(MAX_FIRST_LINE_BYTES).expect("cap") * 2],
    )
    .expect("write");

    assert!(matches!(read_session_file(&huge), SessionFile::Rejected));
    let _ = tracking.resolve();

    assert!(tracking.known_files.contains(&huge));
}

#[test]
fn a_file_of_another_folder_is_not_read_again() {
    let fixture = fixture();
    let mut tracking = begin(&fixture);
    write_session(&fixture, &id(1), Path::new("/some/other/folder"));

    let _ = tracking.resolve();

    assert_eq!(tracking.known_files.len(), 1);
}

fn listing(folder: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files: Vec<_> = session_files(folder)
        .into_iter()
        .map(|file| {
            let bytes = fs::read(&file).expect("read");
            (file, bytes)
        })
        .collect();
    files.sort();
    files
}

fn overlap_flag(guard: &PendingGuard) -> bool {
    guard.ambiguous.load(Ordering::SeqCst)
}

#[test]
fn two_registrations_of_the_same_folder_through_a_dot_dot_overlap() {
    let fixture = fixture();
    let first = fixture.pending.register(&fixture.folder);
    let detour = fixture.folder.join("..").join("work");

    let second = fixture.pending.register(&detour);

    assert!(overlap_flag(&first));
    assert!(overlap_flag(&second));
}

#[test]
fn registrations_of_different_folders_do_not_overlap() {
    let fixture = fixture();
    let other = fixture.folder.parent().expect("root").join("elsewhere");
    fs::create_dir_all(&other).expect("other");
    let first = fixture.pending.register(&fixture.folder);

    let second = fixture.pending.register(&other);

    assert!(!overlap_flag(&first));
    assert!(!overlap_flag(&second));
}

#[cfg(unix)]
#[test]
fn two_registrations_of_the_same_folder_through_a_symlink_overlap() {
    let fixture = fixture();
    let link = fixture.folder.parent().expect("root").join("link");
    std::os::unix::fs::symlink(&fixture.folder, &link).expect("symlink");
    let first = fixture.pending.register(&fixture.folder);

    let second = fixture.pending.register(&link);

    assert!(overlap_flag(&first));
    assert!(overlap_flag(&second));
}

#[cfg(unix)]
#[test]
fn a_symlinked_folder_or_file_in_the_sessions_folder_is_not_followed() {
    let fixture = fixture();
    let outside = fixture.folder.parent().expect("root").join("outside");
    fs::create_dir_all(&outside).expect("outside");
    let real_file = outside.join(format!("rollout-{}.jsonl", id(7)));
    let meta = format!(
        "{{\"payload\":{{\"id\":\"{}\",\"cwd\":{}}}}}\n",
        id(7),
        serde_json::to_string(&fixture.folder.to_string_lossy()).expect("json")
    );
    fs::write(&real_file, meta).expect("write");
    let mut tracking = begin(&fixture);
    std::os::unix::fs::symlink(&outside, fixture.sessions.join("linked-folder")).expect("link");
    std::os::unix::fs::symlink(
        &real_file,
        fixture.sessions.join(real_file.file_name().expect("name")),
    )
    .expect("link");

    let result = tracking.resolve();

    assert!(
        matches!(&result, TrackedSession::Unmatched(reason) if reason.contains("no new session")),
        "{result:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_fifo_named_like_a_session_file_is_skipped_and_never_opened() {
    let fixture = fixture();
    let fifo = fixture.sessions.join(format!("rollout-{}.jsonl", id(8)));
    let status = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo");
    assert!(status.success());

    assert_eq!(session_files(&fixture.sessions), Vec::<PathBuf>::new());
}
