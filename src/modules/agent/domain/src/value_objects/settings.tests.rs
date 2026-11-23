// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn defaults_follow_the_data_model() {
    let settings = Settings::default();
    assert_eq!(settings.quiet_period_ms, 5_000);
    assert_eq!(settings.output_line_limit, 10_000);
    assert_eq!(settings.stop_grace_ms, 10_000);
    assert_eq!(settings.focus_shortcut, "Ctrl+Shift+F12");
    assert_eq!(settings.last_view, None);
    assert_eq!(settings.idle_sleep_minutes, 0);
    assert_eq!(settings.temp_cap, 8);
    assert_eq!(settings.temp_idle_minutes, 30);
    assert!(settings.worktree_dependency_links);
    assert_eq!(settings.value_of(SettingKey::WorktreeDependencyLinks), "on");
}

#[test]
fn every_setting_round_trips_through_its_stored_text() {
    let original = Settings {
        quiet_period_ms: 1,
        focus_shortcut: "Ctrl+Shift+K".to_owned(),
        last_view: Some(r#"{"project": "p1", "agent": "a1", "terminal_open": true}"#.to_owned()),
        ..Settings::default()
    };

    let mut restored = Settings::default();
    for key in SettingKey::ALL {
        restored
            .set_from_text(key, &original.value_of(key))
            .expect("valid");
    }

    assert_eq!(restored, original);
}

#[test]
fn a_last_view_of_the_documented_shape_stores_and_reads_back() {
    let mut settings = Settings::default();
    let value = r#"{"project": null, "agent": "a-1", "terminal_open": false}"#;

    settings
        .set_from_text(SettingKey::LastView, value)
        .expect("valid");

    assert_eq!(settings.value_of(SettingKey::LastView), value);
}

#[test]
fn an_empty_last_view_clears_the_stored_view() {
    let mut settings = Settings::default();
    settings
        .set_from_text(
            SettingKey::LastView,
            r#"{"project": null, "agent": null, "terminal_open": false}"#,
        )
        .expect("valid");

    settings
        .set_from_text(SettingKey::LastView, "")
        .expect("valid");

    assert_eq!(settings.last_view, None);
    assert_eq!(settings.value_of(SettingKey::LastView), "");
}

#[test]
fn a_last_view_past_one_kib_is_refused() {
    let mut settings = Settings::default();
    let id = "p".repeat(1_024);
    let value = format!(r#"{{"project": "{id}", "agent": null, "terminal_open": false}}"#);

    assert_eq!(
        settings.set_from_text(SettingKey::LastView, &value),
        Err(AgentError::TooLarge {
            field: "last_view",
            max: 1_024
        })
    );
    assert_eq!(settings.last_view, None);
}

#[test]
fn a_last_view_that_is_not_the_documented_shape_is_refused() {
    let mut settings = Settings::default();
    let wrong_shapes = [
        "not json",
        "[]",
        r#"{"project": null, "agent": null}"#,
        r#"{"project": null, "agent": null, "terminal_open": "yes"}"#,
        r#"{"project": 3, "agent": null, "terminal_open": false}"#,
    ];
    for value in wrong_shapes {
        assert!(
            matches!(
                settings.set_from_text(SettingKey::LastView, value),
                Err(AgentError::InvalidLastView {
                    field: "last_view",
                    ..
                })
            ),
            "{value}"
        );
    }
    assert_eq!(settings.last_view, None);
}

#[test]
fn a_last_view_may_carry_keys_that_a_later_change_adds() {
    let mut settings = Settings::default();
    // 0005 adds a "view" field; the server validates the documented keys only.
    let value = r#"{"project": null, "agent": null, "terminal_open": false, "view": "scene"}"#;

    settings
        .set_from_text(SettingKey::LastView, value)
        .expect("valid");

    assert_eq!(settings.value_of(SettingKey::LastView), value);
}

#[test]
fn a_focus_shortcut_that_a_fixed_shortcut_takes_names_the_conflict() {
    for (shortcut, reserved_for) in RESERVED_SHORTCUTS {
        let mut settings = Settings::default();
        assert_eq!(
            settings.set_from_text(SettingKey::FocusShortcut, shortcut),
            Err(AgentError::ReservedShortcut {
                field: "focus_shortcut",
                shortcut,
                reserved_for,
            })
        );
        assert_eq!(settings.focus_shortcut, "Ctrl+Shift+F12");
    }
}

#[test]
fn a_focus_shortcut_outside_the_fixed_table_is_stored() {
    let mut settings = Settings::default();

    settings
        .set_from_text(SettingKey::FocusShortcut, "Ctrl+Alt+F11")
        .expect("valid");

    assert_eq!(settings.focus_shortcut, "Ctrl+Alt+F11");
}

#[test]
fn a_reserved_shortcut_is_refused_in_every_spelling_the_desktop_matches() {
    let spellings = [
        "ctrl+shift+f1",
        "Shift+Ctrl+F1",
        "Ctrl + Shift + F1",
        "  Ctrl+Shift+F1  ",
    ];
    for value in spellings {
        let mut settings = Settings::default();
        assert_eq!(
            settings.set_from_text(SettingKey::FocusShortcut, value),
            Err(AgentError::ReservedShortcut {
                field: "focus_shortcut",
                shortcut: "Ctrl+Shift+F1",
                reserved_for: "agent 1",
            }),
            "{value}"
        );
    }
}

#[test]
fn the_flag_tokens_are_the_ones_the_desktop_reads() {
    // "control" is a modifier name in the desktop's key check, but it does
    // not set the ctrl flag, so this spelling is a different combination.
    let mut settings = Settings::default();
    settings
        .set_from_text(SettingKey::FocusShortcut, "Control+shift+f1")
        .expect("stored as the desktop reads it");
}

#[test]
fn a_text_that_names_no_key_stores_as_before_and_conflicts_nothing() {
    let mut settings = Settings::default();

    // Only modifiers: the desktop reads no key here, so no conflict applies,
    // and the 0001 behavior of storing any non-blank text stays.
    for value in ["Ctrl+Shift", "+", "Ctrl++Shift"] {
        settings
            .set_from_text(SettingKey::FocusShortcut, value)
            .expect("stored");
        assert_eq!(settings.focus_shortcut, value);
    }

    settings
        .set_from_text(SettingKey::FocusShortcut, "Ctrl+Shift+F13")
        .expect("a key outside the table is free");
}

#[test]
fn a_stored_jump_shortcut_is_kept_in_every_spelling() {
    for stored in ["Ctrl+Shift+F11", "Shift+Ctrl+F11", " ctrl + shift + f11 "] {
        let mut settings = Settings::default();

        settings
            .set_from_stored_text(SettingKey::FocusShortcut, stored)
            .expect("kept");

        assert_eq!(settings.focus_shortcut, stored);
    }
}

#[test]
fn a_stored_value_loads_as_a_new_value_does_except_the_jump_shortcut() {
    let stored_values = [
        ("Ctrl+Shift+F1", false),
        ("Ctrl+Shift+F10", false),
        ("Ctrl+Shift+H", false),
        ("Ctrl+Shift+F12", true),
        ("Ctrl+Alt+F11", true),
        ("Ctrl+Shift+F13", true),
        ("Ctrl+Shift", true),
    ];
    for (stored, accepted) in stored_values {
        let mut settings = Settings::default();

        let outcome = settings.set_from_stored_text(SettingKey::FocusShortcut, stored);

        assert_eq!(outcome.is_ok(), accepted, "{stored}");
    }
}

#[test]
fn a_reserved_shortcut_with_an_extra_modifier_is_not_the_same_combination() {
    let mut settings = Settings::default();

    settings
        .set_from_text(SettingKey::FocusShortcut, "Ctrl+Alt+Shift+F1")
        .expect("a different combination is free");

    assert_eq!(settings.focus_shortcut, "Ctrl+Alt+Shift+F1");
}

#[test]
fn combinations_that_lose_or_gain_a_modifier_are_not_reserved() {
    let mut settings = Settings::default();

    // A subset of the modifiers: a different combination, so the desktop
    // matches nothing in the fixed table there either.
    settings
        .set_from_text(SettingKey::FocusShortcut, "Ctrl+F1")
        .expect("stored");
    settings
        .set_from_text(SettingKey::FocusShortcut, "Alt+Ctrl+Shift+F1")
        .expect("stored");

    assert_eq!(settings.focus_shortcut, "Alt+Ctrl+Shift+F1");
}

#[test]
fn each_reserved_shortcut_normalizes_to_its_canonical_form() {
    for (text, reserved_for) in RESERVED_SHORTCUTS {
        let key = text.rsplit('+').next().expect("the key is the last part");
        let canonical = format!("ctrl+shift+{}", key.to_lowercase());
        assert_eq!(
            normalized_shortcut(text).as_deref(),
            Some(canonical.as_str()),
            "{reserved_for}"
        );
    }
    assert_eq!(
        normalized_shortcut("Ctrl+Shift+F10").as_deref(),
        Some("ctrl+shift+f10"),
    );
}

#[test]
fn a_value_normalizes_with_each_modifier_counted_once() {
    assert_eq!(
        normalized_shortcut("Shift+Ctrl+Shift+F1").as_deref(),
        Some("ctrl+shift+f1"),
    );
}

#[test]
fn rejects_text_that_is_not_a_positive_number() {
    let mut settings = Settings::default();
    assert_eq!(
        settings.set_from_text(SettingKey::StopGraceMs, "soon"),
        Err(AgentError::InvalidNumber {
            field: "stop_grace_ms",
            text: "soon".to_owned()
        })
    );
    assert_eq!(
        settings.set_from_text(SettingKey::QuietPeriodMs, "0"),
        Err(AgentError::NotPositive {
            field: "quiet_period_ms"
        })
    );
}

#[test]
fn rejects_an_unknown_setting_name() {
    assert_eq!(
        SettingKey::parse("volume"),
        Err(AgentError::UnknownSetting("volume".to_owned()))
    );
}

const LIMIT_KEYS: [SettingKey; 5] = [
    SettingKey::LimitDirectivesPerAgent,
    SettingKey::LimitDirectivesPerTarget,
    SettingKey::LimitCreatesPerAgent,
    SettingKey::LimitLifecyclePerAgent,
    SettingKey::LimitReportsPerAgent,
];

#[test]
fn the_limit_defaults_follow_the_plan() {
    let settings = Settings::default();
    assert_eq!(settings.limit_directives_per_agent, 10);
    assert_eq!(settings.limit_directives_per_target, 3);
    assert_eq!(settings.limit_creates_per_agent, 5);
    assert_eq!(settings.limit_lifecycle_per_agent, 10);
    assert_eq!(settings.limit_reports_per_agent, 5);
}

#[test]
fn a_limit_takes_every_whole_number_from_one_to_a_thousand() {
    for key in LIMIT_KEYS {
        for value in ["1", "1000"] {
            let mut settings = Settings::default();
            settings.set_from_text(key, value).expect("valid");
            assert_eq!(settings.value_of(key), value);
        }
    }
}

#[test]
fn a_limit_outside_that_range_or_not_a_whole_number_is_refused() {
    for key in LIMIT_KEYS {
        for value in ["0", "-1", "1001", "abc", "2.5", "", " 3"] {
            let mut settings = Settings::default();
            let error = settings.set_from_text(key, value).expect_err(value);
            assert_eq!(
                error,
                AgentError::InvalidLimit {
                    field: key.as_str()
                }
            );
            assert!(
                error
                    .to_string()
                    .contains("a limit is a whole number of at least 1 and at most 1000"),
                "{value}"
            );
            assert_eq!(settings, Settings::default());
        }
    }
}

#[test]
fn settings_stored_before_the_limits_existed_read_the_default_limits() {
    let stored = r#"{"quiet_period_ms": 7, "output_line_limit": 8, "focus_shortcut": "Ctrl+Alt+K", "stop_grace_ms": 9, "last_view": null}"#;

    let settings: Settings = serde_json::from_str(stored).expect("old form loads");

    assert_eq!(settings.quiet_period_ms, 7);
    assert_eq!(settings.limit_directives_per_target, 3);
}

#[test]
fn the_focus_shortcut_cannot_take_the_office_hold() {
    let mut settings = Settings::default();
    assert_eq!(
        settings.set_from_text(SettingKey::FocusShortcut, "ctrl+shift+h"),
        Err(AgentError::ReservedShortcut {
            field: "focus_shortcut",
            shortcut: "Ctrl+Shift+H",
            reserved_for: "the office hold",
        })
    );
}

#[test]
fn idle_sleep_takes_zero_or_five_to_fourteen_forty_minutes() {
    for value in ["0", "5", "30", "1440"] {
        let mut settings = Settings::default();
        settings
            .set_from_text(SettingKey::IdleSleepMinutes, value)
            .expect("valid");
        assert_eq!(settings.value_of(SettingKey::IdleSleepMinutes), value);
    }
}

#[test]
fn idle_sleep_outside_that_range_is_refused_and_keeps_the_stored_value() {
    for value in ["4", "1", "1441", "2.5", "-5", "abc", ""] {
        let mut settings = Settings::default();
        settings
            .set_from_text(SettingKey::IdleSleepMinutes, "30")
            .expect("valid");

        let error = settings
            .set_from_text(SettingKey::IdleSleepMinutes, value)
            .expect_err(value);

        assert_eq!(
            error.to_string(),
            "idle_sleep_minutes is 0 (off) or a whole number of minutes from 5 to 1440",
            "{value}"
        );
        assert_eq!(settings.idle_sleep_minutes, 30, "{value}");
    }
}

struct TempSettingCase {
    key: SettingKey,
    name: &'static str,
    accepted: [&'static str; 2],
    refused: [&'static str; 7],
    message: &'static str,
}

const TEMP_SETTINGS: [TempSettingCase; 2] = [
    TempSettingCase {
        key: SettingKey::TempCap,
        name: "temp_cap",
        accepted: ["1", "100"],
        refused: ["0", "101", "-1", "2.5", "abc", "", " 5"],
        message: "temp_cap is a whole number from 1 to 100",
    },
    TempSettingCase {
        key: SettingKey::TempIdleMinutes,
        name: "temp_idle_minutes",
        accepted: ["5", "1440"],
        refused: ["0", "4", "1441", "2.5", "-5", "abc", ""],
        message: "temp_idle_minutes is a whole number from 5 to 1440",
    },
];

#[test]
fn a_temp_setting_takes_both_ends_of_its_range() {
    for case in TEMP_SETTINGS {
        assert_eq!(case.key.as_str(), case.name);
        assert_eq!(SettingKey::parse(case.name), Ok(case.key));
        for value in case.accepted {
            let mut settings = Settings::default();
            settings.set_from_text(case.key, value).expect(value);
            assert_eq!(settings.value_of(case.key), value);
        }
    }
}

#[test]
fn a_temp_setting_outside_its_range_is_refused_with_the_range_and_keeps_the_stored_value() {
    for case in TEMP_SETTINGS {
        for value in case.refused {
            let mut settings = Settings::default();
            settings
                .set_from_text(case.key, case.accepted[0])
                .expect("valid");

            let error = settings.set_from_text(case.key, value).expect_err(value);

            assert_eq!(error.to_string(), case.message, "{value}");
            assert_eq!(settings.value_of(case.key), case.accepted[0], "{value}");
        }
    }
}

#[test]
fn worktree_dependency_links_take_on_or_off() {
    let mut settings = Settings::default();
    settings
        .set_from_text(SettingKey::WorktreeDependencyLinks, "off")
        .expect("valid");
    assert!(!settings.worktree_dependency_links);
    assert_eq!(
        settings.value_of(SettingKey::WorktreeDependencyLinks),
        "off"
    );
    settings
        .set_from_text(SettingKey::WorktreeDependencyLinks, "on")
        .expect("valid");
    assert!(settings.worktree_dependency_links);
}

#[test]
fn worktree_dependency_links_refuse_other_text_and_keep_the_stored_value() {
    for value in ["", "On", "true", "1", " off", "no"] {
        let mut settings = Settings::default();
        settings
            .set_from_text(SettingKey::WorktreeDependencyLinks, "off")
            .expect("valid");

        let error = settings
            .set_from_text(SettingKey::WorktreeDependencyLinks, value)
            .expect_err(value);

        assert_eq!(
            error.to_string(),
            "worktree_dependency_links is on or off",
            "{value}"
        );
        assert!(!settings.worktree_dependency_links, "{value}");
    }
}

#[test]
fn hook_status_is_on_by_default_and_takes_on_or_off() {
    let mut settings = Settings::default();
    assert!(settings.hook_status);
    assert_eq!(settings.value_of(SettingKey::HookStatus), "on");
    settings
        .set_from_text(SettingKey::HookStatus, "off")
        .expect("off");
    assert!(!settings.hook_status);
    assert_eq!(settings.value_of(SettingKey::HookStatus), "off");
    settings
        .set_from_text(SettingKey::HookStatus, "on")
        .expect("on");
    assert!(settings.hook_status);
}

#[test]
fn hook_status_refuses_other_text_and_keeps_the_stored_value() {
    let mut settings = Settings::default();
    settings
        .set_from_text(SettingKey::HookStatus, "off")
        .expect("off");
    for value in ["", "yes", "ON", "1"] {
        assert!(
            settings
                .set_from_text(SettingKey::HookStatus, value)
                .is_err(),
            "{value}"
        );
        assert!(!settings.hook_status, "{value}");
    }
}
