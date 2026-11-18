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
