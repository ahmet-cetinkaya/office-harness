// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn defaults_follow_the_data_model() {
    let settings = Settings::default();
    assert_eq!(settings.quiet_period_ms, 5_000);
    assert_eq!(settings.output_line_limit, 10_000);
    assert_eq!(settings.stop_grace_ms, 10_000);
    assert_eq!(settings.focus_shortcut, "Ctrl+Shift+F12");
}

#[test]
fn every_setting_round_trips_through_its_stored_text() {
    let original = Settings {
        quiet_period_ms: 1,
        focus_shortcut: "Ctrl+Shift+K".to_owned(),
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
