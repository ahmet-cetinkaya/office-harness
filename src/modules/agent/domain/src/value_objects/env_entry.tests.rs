// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn accepts_letters_digits_and_underscores_after_a_letter_or_underscore() {
    for name in ["A", "_x", "PATH", "my_var_2"] {
        assert!(EnvEntry::parse(name, "v").is_ok(), "{name}");
    }
}

#[test]
fn refuses_a_malformed_name_and_names_it() {
    for name in ["", "1ABC", "A-B", "A B", "A=B", "é"] {
        let error = EnvEntry::parse(name, "v").expect_err(name);
        assert_eq!(
            error,
            AgentError::InvalidEnvName {
                name: name.to_owned()
            }
        );
        assert!(error.to_string().contains(&format!("{name:?}")));
    }
}

#[test]
fn refuses_a_value_with_a_nul_character() {
    assert!(matches!(
        EnvEntry::parse("KEY", "a\0b"),
        Err(AgentError::ContainsNul { .. })
    ));
}

#[test]
fn a_name_in_the_reserved_prefix_is_refused() {
    assert_eq!(
        EnvEntry::parse("OFFICE_HARNESS_AGENT_TOKEN", "x"),
        Err(AgentError::ReservedEnvName {
            name: "OFFICE_HARNESS_AGENT_TOKEN".to_owned()
        })
    );
    assert!(EnvEntry::parse("OFFICE_HARNESS", "x").is_ok());
    assert!(EnvEntry::parse("MY_OFFICE_HARNESS_KEY", "x").is_ok());
}
