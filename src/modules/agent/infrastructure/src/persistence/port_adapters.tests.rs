// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::errors::AgentError;

use super::*;

#[test]
fn each_persistence_error_has_a_port_error() {
    let duplicate = PersistenceError::Duplicate {
        what: "agent Ana".to_owned(),
    };
    assert_eq!(
        PortError::from(duplicate),
        PortError::Duplicate {
            what: "agent Ana".to_owned()
        }
    );

    let missing = PersistenceError::NotFound {
        what: "agent 1".to_owned(),
    };
    assert_eq!(
        PortError::from(missing),
        PortError::NotFound {
            what: "agent 1".to_owned()
        }
    );

    let in_use = PersistenceError::ProjectInUse {
        project: "Docs".to_owned(),
        agents: vec!["Ana".to_owned()],
    };
    assert_eq!(
        PortError::from(in_use),
        PortError::ProjectInUse {
            project: "Docs".to_owned(),
            agents: vec!["Ana".to_owned()]
        }
    );
}

#[test]
fn rows_that_the_domain_rejects_become_invalid_data() {
    let invalid = PersistenceError::Invalid(AgentError::Empty { field: "command" });
    let converted = PortError::from(invalid);
    assert!(
        matches!(&converted, PortError::InvalidData { message } if message.contains("command"))
    );

    let bad_id = PersistenceError::InvalidAgentId(
        "not-an-id"
            .parse::<kernel::AgentId>()
            .expect_err("not an id"),
    );
    assert!(matches!(
        PortError::from(bad_id),
        PortError::InvalidData { .. }
    ));
}

#[test]
fn a_full_disk_is_flagged_and_other_storage_failures_are_not() {
    let disk_full =
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL), None);
    assert!(matches!(
        PortError::from(PersistenceError::Storage(disk_full)),
        PortError::Storage {
            is_disk_failure: true,
            ..
        }
    ));

    let other = PersistenceError::Storage(rusqlite::Error::QueryReturnedNoRows);
    assert!(matches!(
        PortError::from(other),
        PortError::Storage {
            is_disk_failure: false,
            ..
        }
    ));
}
