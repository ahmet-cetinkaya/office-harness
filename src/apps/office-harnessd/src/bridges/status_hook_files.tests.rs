// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_first_version_in_a_text_is_read() {
    assert_eq!(parse_version("2.1.322 (Claude Code)"), Some([2, 1, 322]));
    assert_eq!(
        parse_version("claude version v10.20.30-beta"),
        Some([10, 20, 30])
    );
    assert_eq!(parse_version("no version here"), None);
    assert_eq!(parse_version("1.2"), None);
    assert_eq!(parse_version("1.2.3.4"), None);
    assert_eq!(parse_version(""), None);
}

#[test]
fn a_version_below_the_minimum_does_not_support_hooks() {
    assert!([2, 1, 294] < MIN_HOOK_VERSION);
    assert!([2, 1, 295] >= MIN_HOOK_VERSION);
    assert!([3, 0, 0] >= MIN_HOOK_VERSION);
}

#[test]
fn the_settings_file_holds_only_hooks_and_no_secret() {
    let text = settings_file_text(4321, "session-1");
    let file: Value = serde_json::from_str(&text).expect("json");
    let top: Vec<_> = file.as_object().expect("object").keys().collect();
    assert_eq!(top, vec!["hooks"]);
    let hooks = file["hooks"].as_object().expect("hooks");
    let mut events: Vec<_> = hooks.keys().map(String::as_str).collect();
    events.sort_unstable();
    let mut wanted = HOOK_EVENTS.to_vec();
    wanted.sort_unstable();
    assert_eq!(events, wanted);
    let hook = &hooks["Stop"][0]["hooks"][0];
    assert_eq!(hook["type"], "http");
    assert_eq!(hook["url"], "http://127.0.0.1:4321/v1/hook/session-1");
    assert_eq!(hook["timeout"], HOOK_TIMEOUT_SECONDS);
    assert_eq!(hook["allowedEnvVars"], json!([HOOK_SECRET_VARIABLE]));
    assert_eq!(
        hook["headers"]["Authorization"],
        format!("Bearer ${HOOK_SECRET_VARIABLE}")
    );
}

mod files {
    use std::sync::atomic::{AtomicBool, Ordering};

    use access_infrastructure::services::MemoryHookSecretStore;
    use agent_application::common::errors::PortError;
    use agent_domain::value_objects::Settings;
    use tempfile::TempDir;

    use super::*;

    struct Switch(AtomicBool);

    impl SettingsStore for Switch {
        fn load(&self) -> Result<Settings, PortError> {
            Ok(Settings {
                hook_status: self.0.load(Ordering::SeqCst),
                ..Settings::default()
            })
        }

        fn save(&self, _settings: &Settings) -> Result<(), PortError> {
            Ok(())
        }
    }

    struct Fixture {
        files: StatusHookFiles,
        secrets: Arc<MemoryHookSecretStore>,
        switch: Arc<Switch>,
        folder: TempDir,
    }

    fn agent() -> AgentId {
        AgentId::from_u128(1)
    }

    fn fixture() -> Fixture {
        let folder = tempfile::tempdir().expect("folder");
        let config = crate::config::Config {
            data_folder: folder.path().join("office"),
        };
        let log = crate::bootstrap::open_log(&config).expect("log");
        let secrets = Arc::new(MemoryHookSecretStore::new().expect("store"));
        let switch = Arc::new(Switch(AtomicBool::new(true)));
        let files = StatusHookFiles::new(
            folder.path().join("hooks"),
            secrets.clone(),
            switch.clone(),
            log,
        );
        files.set_port(4321);
        Fixture {
            files,
            secrets,
            switch,
            folder,
        }
    }

    /// A program that answers `--version` with `text`.
    #[cfg(unix)]
    fn program_answering(folder: &TempDir, text: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let path = folder.path().join("fake-cli");
        std::fs::write(&path, format!("#!/bin/sh\necho '{text}'\n")).expect("script");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
        path.display().to_string()
    }

    #[test]
    fn a_cli_without_hooks_and_a_refused_request_get_no_file_and_say_why() {
        let fixture = fixture();
        let unsupported = fixture
            .files
            .plan(&StatusHooksRequest::Unsupported, "s1", "x", &[]);
        assert_eq!(unsupported.arguments, Vec::<String>::new());
        assert_eq!(unsupported.note, NOTE_NO_HOOKS);
        let refused = fixture.files.plan(
            &StatusHooksRequest::Refused("own option".to_owned()),
            "s1",
            "x",
            &[],
        );
        assert_eq!(refused.arguments, Vec::<String>::new());
        assert_eq!(refused.note, "own option");
    }

    #[test]
    fn the_switch_and_a_missing_port_give_no_file() {
        let fixture = fixture();
        fixture.switch.0.store(false, Ordering::SeqCst);
        let off = fixture
            .files
            .plan(&StatusHooksRequest::Wanted, "s1", "x", &[]);
        assert_eq!(
            (off.arguments.len(), off.note.as_str()),
            (0, NOTE_SWITCHED_OFF)
        );
        let folder = tempfile::tempdir().expect("folder");
        let config = crate::config::Config {
            data_folder: folder.path().join("office"),
        };
        let log = crate::bootstrap::open_log(&config).expect("log");
        let unbound = StatusHookFiles::for_tests(folder.path().join("hooks"), log);
        let plan = unbound.plan(&StatusHooksRequest::Wanted, "s1", "x", &[]);
        assert_eq!(
            (plan.arguments.len(), plan.note.as_str()),
            (0, NOTE_NOT_LISTENING)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_new_enough_version_gets_the_option_and_an_old_or_silent_one_does_not() {
        let fixture = fixture();
        let new = program_answering(&fixture.folder, "2.1.322 (Claude Code)");
        let plan = fixture
            .files
            .plan(&StatusHooksRequest::Wanted, "s1", &new, &[]);
        assert_eq!(plan.arguments.len(), 2);
        assert_eq!(plan.arguments[0], SETTINGS_OPTION);
        assert!(plan.arguments[1].ends_with("s1.json"));
        assert_eq!(plan.note, NOTE_FIRST_REPORT);

        let other = tempfile::tempdir().expect("folder");
        let old = program_answering(&other, "2.1.100 (Claude Code)");
        let old_plan = fixture
            .files
            .plan(&StatusHooksRequest::Wanted, "s1", &old, &[]);
        assert_eq!(old_plan.note, NOTE_OLD_VERSION);

        let silent = program_answering(&tempfile::tempdir().expect("folder"), "no version");
        let silent_plan = fixture
            .files
            .plan(&StatusHooksRequest::Wanted, "s1", &silent, &[]);
        assert_eq!(silent_plan.note, NOTE_OLD_VERSION);
        let missing =
            fixture
                .files
                .plan(&StatusHooksRequest::Wanted, "s1", "/no/such/program", &[]);
        assert_eq!(missing.note, NOTE_OLD_VERSION);
    }

    #[test]
    fn a_file_and_its_secret_live_until_the_session_ends() {
        let fixture = fixture();
        let (name, secret) = fixture.files.activate(agent(), "s1").expect("activate");
        assert_eq!(name, HOOK_SECRET_VARIABLE);
        let path = fixture.folder.path().join("hooks/s1.json");
        let text = std::fs::read_to_string(&path).expect("file");
        assert!(!text.contains(&secret), "the file holds no secret");
        assert_eq!(fixture.secrets.authorize("s1", &secret), Some(agent()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "owner only");
        }

        fixture.files.end(agent());
        assert!(!path.exists());
        assert_eq!(fixture.secrets.authorize("s1", &secret), None);
    }

    #[test]
    fn a_new_session_replaces_the_file_and_secret_of_the_earlier_one() {
        let fixture = fixture();
        let (_, first) = fixture.files.activate(agent(), "s1").expect("activate");
        let (_, second) = fixture.files.activate(agent(), "s2").expect("activate");
        assert!(!fixture.folder.path().join("hooks/s1.json").exists());
        assert!(fixture.folder.path().join("hooks/s2.json").exists());
        assert_eq!(fixture.secrets.authorize("s1", &first), None);
        assert_eq!(fixture.secrets.authorize("s2", &second), Some(agent()));
    }

    #[test]
    fn a_file_that_cannot_be_written_leaves_no_secret() {
        let fixture = fixture();
        // The folder's place holds a file, so the folder cannot be made.
        std::fs::write(fixture.folder.path().join("hooks"), b"in the way").expect("block");
        let reason = fixture.files.activate(agent(), "s1").expect_err("no file");
        assert_eq!(reason, NOTE_FILE_FAILED);
        assert_eq!(fixture.secrets.authorize("s1", "anything"), None);
    }

    #[test]
    fn the_sweep_deletes_every_settings_file_and_leaves_other_files() {
        let fixture = fixture();
        fixture.files.activate(agent(), "s1").expect("activate");
        fixture
            .files
            .activate(AgentId::from_u128(2), "s2")
            .expect("activate");
        let other = fixture.folder.path().join("hooks/notes.txt");
        std::fs::write(&other, b"x").expect("other");
        fixture.files.sweep_stale();
        assert!(!fixture.folder.path().join("hooks/s1.json").exists());
        assert!(!fixture.folder.path().join("hooks/s2.json").exists());
        assert!(other.exists());
    }
}
