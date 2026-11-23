// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The secret service over a real SQLite file, with a fake keyring, the real
//! sealer, and the real key file vault (0026, T2).

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use kernel::{AgentId, Timestamp};
use rusqlite::Connection;
use secret_application::{
    AgentDirectory, SecretClock, SecretPorts, SecretService, SecretServiceError, SecretStore,
    StoreError, StoreKeyVault, StoreState, VaultError,
};
use secret_domain::{
    Custody, LockedReason, MAX_SECRETS, SecretName, SecretValue, SecretVersion, StoreKey,
};
use secret_infrastructure::persistence::{SCRIPTS, SecretRepository};
use secret_infrastructure::{ChaChaRecordSealer, KeyFileVault, OsStoreKeyGenerator};

const MARKER: &str = "unique-marker-value-0026";
const NEW_MARKER: &str = "second-marker-value-0026";

/// A keyring in memory that can refuse, or be locked after it took the key.
#[derive(Default)]
struct FakeKeyring {
    held: Mutex<Option<[u8; 32]>>,
    refuses: AtomicBool,
    locked: AtomicBool,
}

impl StoreKeyVault for FakeKeyring {
    fn custody(&self) -> Custody {
        Custody::OsKeyring
    }

    fn load(&self) -> Result<Option<StoreKey>, VaultError> {
        if self.locked.load(Ordering::SeqCst) {
            return Err(VaultError::Locked(LockedReason::KeyringLocked));
        }
        Ok((*self.held.lock().expect("lock")).map(StoreKey::from_bytes))
    }

    fn store(&self, key: &StoreKey) -> Result<(), VaultError> {
        if self.refuses.load(Ordering::SeqCst) {
            return Err(VaultError::Locked(LockedReason::KeyringNotRunning));
        }
        *self.held.lock().expect("lock") = Some(*key.expose());
        Ok(())
    }

    fn remove(&self) -> Result<(), VaultError> {
        *self.held.lock().expect("lock") = None;
        Ok(())
    }
}

/// Every agent exists, except the ones a test deleted.
#[derive(Default)]
struct Agents {
    deleted: Mutex<Vec<AgentId>>,
}

impl AgentDirectory for Agents {
    fn exists(&self, agent: &AgentId) -> Result<bool, StoreError> {
        Ok(!self.deleted.lock().expect("lock").contains(agent))
    }
}

struct FixedClock;

impl SecretClock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(1_000)
    }
}

struct Office {
    folder: tempfile::TempDir,
    keyring: Arc<FakeKeyring>,
    agents: Arc<Agents>,
    service: SecretService,
    repository: SecretRepository,
}

fn office() -> Office {
    let folder = tempfile::tempdir().expect("folder");
    let connection = Connection::open(folder.path().join("office.db")).expect("open");
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON;",
        )
        .expect("pragmas");
    for script in SCRIPTS {
        connection.execute_batch(script).expect("script");
    }
    let repository = SecretRepository::new(Arc::new(Mutex::new(connection)));
    let keyring = Arc::new(FakeKeyring::default());
    let agents = Arc::new(Agents::default());
    let service = SecretService::new(SecretPorts {
        store: Arc::new(repository.clone()),
        keyring: keyring.clone(),
        key_file: Arc::new(KeyFileVault::new(folder.path().join("secrets.key"))),
        generator: Arc::new(OsStoreKeyGenerator),
        sealer: Arc::new(ChaChaRecordSealer),
        agents: agents.clone(),
        clock: Arc::new(FixedClock),
    });
    Office {
        folder,
        keyring,
        agents,
        service,
        repository,
    }
}

fn name(text: &str) -> SecretName {
    SecretName::parse(text).expect("name")
}

fn value(text: &str) -> SecretValue {
    SecretValue::parse(text.to_owned()).expect("value")
}

fn agent(number: u128) -> AgentId {
    AgentId::from_u128(number)
}

fn save(office: &Office, secret: &str, text: &str) {
    office
        .service
        .save(name(secret), &value(text))
        .expect("save");
}

/// Everything the database writes to disk: the file and its write log.
fn database_bytes(folder: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    for file in ["office.db", "office.db-wal", "office.db-shm"] {
        if let Ok(content) = fs::read(folder.join(file)) {
            bytes.extend(content);
        }
    }
    bytes
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn the_first_save_chooses_the_keyring_and_the_value_is_not_in_the_files() {
    let office = office();
    assert_eq!(office.service.state().expect("state"), StoreState::Empty);

    save(&office, "API_KEY", MARKER);

    assert_eq!(
        office.service.state().expect("state"),
        StoreState::Ready(Custody::OsKeyring)
    );
    assert!(!office.folder.path().join("secrets.key").exists());
    assert!(!contains(
        &database_bytes(office.folder.path()),
        MARKER.as_bytes()
    ));
}

#[test]
fn a_keyring_that_refuses_the_key_sends_it_to_an_owner_only_key_file() {
    let office = office();
    office.keyring.refuses.store(true, Ordering::SeqCst);

    save(&office, "API_KEY", MARKER);

    assert_eq!(
        office.service.state().expect("state"),
        StoreState::Ready(Custody::KeyFile)
    );
    assert!(office.folder.path().join("secrets.key").exists());
}

#[test]
fn the_custody_is_never_chosen_again_when_the_keyring_returns_later() {
    let office = office();
    office.keyring.refuses.store(true, Ordering::SeqCst);
    save(&office, "API_KEY", MARKER);

    office.keyring.refuses.store(false, Ordering::SeqCst);
    save(&office, "OTHER_KEY", NEW_MARKER);

    assert_eq!(
        office.service.state().expect("state"),
        StoreState::Ready(Custody::KeyFile)
    );
    assert!(office.keyring.load().expect("load").is_none());
}

#[test]
fn a_replace_raises_the_version_keeps_the_grants_and_answers_the_new_value() {
    let office = office();
    save(&office, "API_KEY", MARKER);
    office
        .service
        .grant(&name("API_KEY"), &agent(1))
        .expect("grant");
    let first = office
        .service
        .read(&agent(1), &name("API_KEY"))
        .expect("read");
    assert_eq!((first.value.reveal(), first.version.number()), (MARKER, 1));

    let summary = office
        .service
        .save(name("API_KEY"), &value(NEW_MARKER))
        .expect("replace");

    assert_eq!(summary.version.number(), 2);
    assert_eq!(summary.grants.len(), 1);
    let second = office
        .service
        .read(&agent(1), &name("API_KEY"))
        .expect("read");
    assert_eq!(
        (second.value.reveal(), second.version.number()),
        (NEW_MARKER, 2)
    );
    assert!(!contains(
        &database_bytes(office.folder.path()),
        MARKER.as_bytes()
    ));
}

#[test]
fn the_dialog_data_shows_the_version_each_agent_read_last() {
    let office = office();
    save(&office, "API_KEY", MARKER);
    office
        .service
        .grant(&name("API_KEY"), &agent(1))
        .expect("grant");
    office
        .service
        .grant(&name("API_KEY"), &agent(2))
        .expect("grant");
    office
        .service
        .read(&agent(1), &name("API_KEY"))
        .expect("read");
    save(&office, "API_KEY", NEW_MARKER);

    let (_, secrets) = office.service.list().expect("list");

    let grants = &secrets[0].grants;
    assert_eq!(
        grants[0].last_read_version.map(SecretVersion::number),
        Some(1)
    );
    assert_eq!(grants[1].last_read_version, None);
    assert_eq!(secrets[0].version.number(), 2);
}

#[test]
fn a_missing_a_deleted_and_an_ungranted_secret_read_the_same() {
    let office = office();
    save(&office, "GRANTED_KEY", MARKER);
    save(&office, "OTHER_KEY", NEW_MARKER);
    office
        .service
        .grant(&name("GRANTED_KEY"), &agent(1))
        .expect("grant");
    office
        .service
        .grant(&name("OTHER_KEY"), &agent(1))
        .expect("grant");
    office.service.delete(&name("OTHER_KEY")).expect("delete");

    let texts: Vec<String> = ["NO_SUCH_KEY", "OTHER_KEY"]
        .iter()
        .map(|missing| {
            office
                .service
                .read(&agent(1), &name(missing))
                .expect_err("refused")
        })
        .map(|error| error.to_string())
        .collect();
    let ungranted = office
        .service
        .read(&agent(2), &name("GRANTED_KEY"))
        .expect_err("refused");

    assert_eq!(
        texts[0].replace("NO_SUCH_KEY", "X"),
        texts[1].replace("OTHER_KEY", "X")
    );
    assert_eq!(
        ungranted,
        SecretServiceError::NotAvailable {
            name: "GRANTED_KEY".to_owned()
        }
    );
    assert_eq!(
        ungranted.to_string().replace("GRANTED_KEY", "X"),
        texts[0].replace("NO_SUCH_KEY", "X")
    );
}

#[test]
fn an_agent_lists_only_what_it_may_read_and_revoke_takes_effect_at_once() {
    let office = office();
    save(&office, "A_KEY", MARKER);
    save(&office, "B_KEY", NEW_MARKER);
    office
        .service
        .grant(&name("B_KEY"), &agent(1))
        .expect("grant");

    let names: Vec<String> = office
        .service
        .available(&agent(1))
        .expect("available")
        .iter()
        .map(|secret| secret.name.to_string())
        .collect();
    assert_eq!(names, ["B_KEY"]);

    office
        .service
        .revoke(&name("B_KEY"), &agent(1))
        .expect("revoke");
    assert!(office.service.read(&agent(1), &name("B_KEY")).is_err());
    assert_eq!(
        office
            .service
            .revoke(&name("B_KEY"), &agent(1))
            .expect_err("none"),
        SecretServiceError::NotGranted
    );
    assert_eq!(
        office
            .service
            .grant(&name("A_KEY"), &agent(3))
            .and_then(|_| office.service.grant(&name("A_KEY"), &agent(3)))
            .expect_err("twice"),
        SecretServiceError::AlreadyGranted
    );
}

#[test]
fn a_delete_removes_the_secret_its_grants_and_the_sealed_bytes() {
    let office = office();
    save(&office, "API_KEY", MARKER);
    office
        .service
        .grant(&name("API_KEY"), &agent(1))
        .expect("grant");
    let sealed = office
        .repository
        .find(&name("API_KEY"))
        .expect("find")
        .expect("secret")
        .sealed;

    office.service.delete(&name("API_KEY")).expect("delete");

    assert_eq!(office.service.available(&agent(1)).expect("available"), []);
    assert_eq!(
        office.service.delete(&name("API_KEY")).expect_err("gone"),
        SecretServiceError::NoSuchSecret {
            name: "API_KEY".to_owned()
        }
    );
    save(&office, "API_KEY", NEW_MARKER);
    let (_, secrets) = office.service.list().expect("list");
    assert_eq!(secrets[0].version.number(), 1);
    assert_eq!(secrets[0].grants, []);
    // The next server start truncates the write log (0023); until then the
    // old pages can stay in it (spec A4 of 0023).
    Connection::open(office.folder.path().join("office.db"))
        .expect("open")
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpoint");
    let sealed_text = &sealed.as_bytes()[24..48];
    assert!(!contains(
        &database_bytes(office.folder.path()),
        sealed_text
    ));
}

#[test]
fn the_grants_of_a_deleted_agent_go() {
    let office = office();
    save(&office, "API_KEY", MARKER);
    office
        .service
        .grant(&name("API_KEY"), &agent(1))
        .expect("grant");

    office.agents.deleted.lock().expect("lock").push(agent(1));
    assert!(office.service.forget_deleted_agents().expect("forget"));
    assert!(!office.service.forget_deleted_agents().expect("again"));

    let (_, secrets) = office.service.list().expect("list");
    assert_eq!(secrets[0].grants, []);
}

#[test]
fn a_locked_store_refuses_reads_and_saves_and_changes_no_record() {
    let office = office();
    save(&office, "API_KEY", MARKER);
    office
        .service
        .grant(&name("API_KEY"), &agent(1))
        .expect("grant");
    let before = office
        .repository
        .find(&name("API_KEY"))
        .expect("find")
        .expect("secret");
    office.keyring.locked.store(true, Ordering::SeqCst);

    let read = office
        .service
        .read(&agent(1), &name("API_KEY"))
        .expect_err("locked");
    let write = office
        .service
        .save(name("API_KEY"), &value(NEW_MARKER))
        .expect_err("locked");

    let locked = SecretServiceError::Locked(LockedReason::KeyringLocked);
    assert_eq!(
        (read, write),
        (
            locked,
            SecretServiceError::Locked(LockedReason::KeyringLocked)
        )
    );
    assert_eq!(
        office.service.state().expect("state"),
        StoreState::Locked(Custody::OsKeyring, LockedReason::KeyringLocked)
    );
    let after = office
        .repository
        .find(&name("API_KEY"))
        .expect("find")
        .expect("secret");
    assert_eq!(before.sealed, after.sealed);
    assert_eq!(before.version, after.version);

    office.keyring.locked.store(false, Ordering::SeqCst);
    let answer = office
        .service
        .read(&agent(1), &name("API_KEY"))
        .expect("read");
    assert_eq!(answer.value.reveal(), MARKER);
}

#[test]
fn a_lost_keyring_entry_is_locked_and_no_new_key_is_made() {
    let office = office();
    save(&office, "API_KEY", MARKER);
    office.keyring.remove().expect("remove");

    let error = office
        .service
        .save(name("OTHER_KEY"), &value(NEW_MARKER))
        .expect_err("locked");

    assert_eq!(
        error,
        SecretServiceError::Locked(LockedReason::KeyringEntryMissing)
    );
    assert!(office.keyring.load().expect("load").is_none());
}

#[test]
fn a_reset_needs_the_confirmed_count_and_removes_everything_and_the_key() {
    let office = office();
    save(&office, "A_KEY", MARKER);
    save(&office, "B_KEY", NEW_MARKER);

    let wrong = office.service.reset(1).expect_err("count");
    assert_eq!(
        wrong,
        SecretServiceError::ConfirmCountDiffers { current: 2 }
    );
    assert_eq!(office.service.list().expect("list").1.len(), 2);

    assert_eq!(office.service.reset(2).expect("reset"), 2);

    assert_eq!(office.service.state().expect("state"), StoreState::Empty);
    assert!(office.keyring.load().expect("load").is_none());
    save(&office, "A_KEY", MARKER);
    assert_eq!(
        office.service.state().expect("state"),
        StoreState::Ready(Custody::OsKeyring)
    );
}

#[test]
fn the_101st_secret_is_refused() {
    let office = office();
    for number in 0..MAX_SECRETS {
        save(&office, &format!("KEY_{number}"), MARKER);
    }

    let error = office
        .service
        .save(name("ONE_TOO_MANY"), &value(MARKER))
        .expect_err("limit");

    assert_eq!(
        error,
        SecretServiceError::TooManySecrets { max: MAX_SECRETS }
    );
}

#[test]
fn a_damaged_record_is_unreadable_and_a_read_gives_no_value() {
    let office = office();
    save(&office, "API_KEY", MARKER);
    office
        .service
        .grant(&name("API_KEY"), &agent(1))
        .expect("grant");
    Connection::open(office.folder.path().join("office.db"))
        .expect("open")
        .execute("UPDATE secret_secrets SET sealed = zeroblob(60)", [])
        .expect("damage");

    let error = office
        .service
        .read(&agent(1), &name("API_KEY"))
        .expect_err("damaged");
    let (_, secrets) = office.service.list().expect("list");

    assert_eq!(error.to_string(), "the secret API_KEY cannot be read");
    assert!(secrets[0].unreadable);
}
