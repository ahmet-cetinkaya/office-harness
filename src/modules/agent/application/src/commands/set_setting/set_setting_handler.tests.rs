// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::test_support::MemorySettings;

#[test]
fn a_valid_value_is_saved_and_returned() {
    let settings = Arc::new(MemorySettings::default());
    let handler = SetSettingHandler::new(settings.clone());

    let changed = handler.handle("quiet_period_ms", "7000").expect("set");

    assert_eq!(changed.quiet_period_ms, 7000);
    assert_eq!(settings.current().quiet_period_ms, 7000);
}

#[test]
fn a_bad_value_names_the_setting_and_changes_nothing() {
    let settings = Arc::new(MemorySettings::default());
    let handler = SetSettingHandler::new(settings.clone());

    let error = handler
        .handle("stop_grace_ms", "soon")
        .expect_err("invalid");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("stop_grace_ms")));
    assert_eq!(settings.current(), Settings::default());
}

#[test]
fn an_unknown_key_is_invalid() {
    let handler = SetSettingHandler::new(Arc::new(MemorySettings::default()));

    let error = handler.handle("volume", "11").expect_err("unknown");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("volume")));
}
