// SPDX-License-Identifier: AGPL-3.0-or-later

use std::cell::{Cell, RefCell};

use tempfile::TempDir;

use super::super::LoginItemError;
use super::*;

const VERSION: &str = "1.2.3";

/// Records what the real login item would have been asked to do.
#[derive(Default)]
struct FakeLoginItem {
    enabled_program: RefCell<Option<PathBuf>>,
    enable_calls: Cell<u32>,
    refuses: Cell<bool>,
}

impl LoginItem for FakeLoginItem {
    fn enable(&self, program: &Path) -> Result<(), LoginItemError> {
        if self.refuses.get() {
            return Err(LoginItemError("refused".to_owned()));
        }
        self.enable_calls.set(self.enable_calls.get() + 1);
        *self.enabled_program.borrow_mut() = Some(program.to_path_buf());
        Ok(())
    }

    fn disable(&self) -> Result<(), LoginItemError> {
        *self.enabled_program.borrow_mut() = None;
        Ok(())
    }

    fn is_enabled(&self) -> Result<bool, LoginItemError> {
        Ok(self.enabled_program.borrow().is_some())
    }
}

fn running_program(folder: &TempDir, content: &[u8]) -> PathBuf {
    let path = folder.path().join("running-office-harnessd");
    fs::write(&path, content).expect("write the running program");
    path
}

#[test]
fn register_copies_the_program_and_enables_the_copy() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);

    let installed = login_start
        .register(&running_program(&source, b"server"))
        .expect("register");

    assert_eq!(installed, login_start.installed_program());
    assert!(installed.starts_with(data.path().join("bin").join(VERSION)));
    assert_eq!(fs::read(&installed).expect("copy"), b"server");
    assert_eq!(*login_item.enabled_program.borrow(), Some(installed));
}

#[test]
fn register_twice_leaves_one_copy_and_one_entry() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);
    let program = running_program(&source, b"server");

    let first = login_start.register(&program).expect("first");
    let second = login_start.register(&program).expect("second");

    assert_eq!(first, second);
    let version_folder = first.parent().expect("version folder");
    assert_eq!(fs::read_dir(version_folder).expect("list").count(), 1);
    assert!(login_start.is_registered().expect("status"));
}

#[test]
fn register_replaces_a_copy_with_other_bytes() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);
    login_start
        .register(&running_program(&source, b"old build"))
        .expect("first");

    let installed = login_start
        .register(&running_program(&source, b"new build!"))
        .expect("second");

    assert_eq!(fs::read(installed).expect("copy"), b"new build!");
}

#[test]
fn a_new_version_gets_its_own_copy_and_keeps_the_old_one() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let program = running_program(&source, b"server");
    let old = LoginStart::new(data.path(), "1.0.0", &login_item)
        .register(&program)
        .expect("old");

    let new = LoginStart::new(data.path(), "2.0.0", &login_item)
        .register(&program)
        .expect("new");

    assert_ne!(old, new);
    assert!(old.exists());
    assert_eq!(*login_item.enabled_program.borrow(), Some(new));
}

#[test]
fn registering_from_the_installed_copy_keeps_it_whole() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);
    let installed = login_start
        .register(&running_program(&source, b"server"))
        .expect("first");

    login_start.register(&installed).expect("from the copy");

    assert_eq!(fs::read(installed).expect("copy"), b"server");
}

#[test]
fn a_refused_entry_is_an_error() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    login_item.refuses.set(true);
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);

    let outcome = login_start.register(&running_program(&source, b"server"));

    assert!(matches!(outcome, Err(CliError::LoginItem(_))));
    assert!(!login_start.is_registered().expect("status"));
}

#[test]
fn a_missing_program_is_an_error_that_names_the_copy() {
    let data = TempDir::new().expect("data folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);

    let outcome = login_start.register(&data.path().join("missing"));

    assert!(matches!(outcome, Err(CliError::Files { .. })));
    assert!(!login_start.is_registered().expect("status"));
}

#[test]
fn unregister_removes_the_entry_and_the_copies_but_not_other_files() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);
    login_start
        .register(&running_program(&source, b"server"))
        .expect("register");
    fs::write(data.path().join("office.db"), b"data").expect("database");

    login_start.unregister().expect("unregister");

    assert!(!login_start.is_registered().expect("status"));
    assert!(!data.path().join("bin").exists());
    assert!(data.path().join("office.db").exists());
}

#[test]
fn unregister_with_nothing_registered_succeeds() {
    let data = TempDir::new().expect("data folder");
    let login_item = FakeLoginItem::default();

    LoginStart::new(data.path(), VERSION, &login_item)
        .unregister()
        .expect("nothing to remove");
}

#[test]
fn the_app_start_registers_when_login_start_was_never_turned_off() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);

    let installed = login_start
        .register_unless_turned_off(&running_program(&source, b"server"))
        .expect("register at the app start");

    assert_eq!(installed, Some(login_start.installed_program()));
    assert!(login_start.is_registered().expect("status"));
}

#[test]
fn the_app_start_does_not_undo_an_unregister() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);
    let program = running_program(&source, b"server");
    login_start.register(&program).expect("register");
    login_start.unregister().expect("unregister");

    let installed = login_start
        .register_unless_turned_off(&program)
        .expect("app start");

    assert_eq!(installed, None);
    assert!(!login_start.is_registered().expect("status"));
    assert!(!data.path().join("bin").exists());
}

#[test]
fn an_explicit_register_turns_login_start_on_again() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);
    let program = running_program(&source, b"server");
    login_start.unregister().expect("unregister");

    login_start.register(&program).expect("register");
    login_item
        .disable()
        .expect("the user removes the entry by hand");
    let installed = login_start
        .register_unless_turned_off(&program)
        .expect("app start");

    assert!(installed.is_some());
    assert!(login_start.is_registered().expect("status"));
}

#[test]
fn a_leftover_partial_copy_does_not_block_the_next_register() {
    let data = TempDir::new().expect("data folder");
    let source = TempDir::new().expect("source folder");
    let login_item = FakeLoginItem::default();
    let login_start = LoginStart::new(data.path(), VERSION, &login_item);
    let partial = login_start
        .installed_program()
        .with_extension(format!("{}.partial", std::process::id()));
    fs::create_dir_all(partial.parent().expect("version folder")).expect("folder");
    fs::write(&partial, b"half").expect("leftover");

    let installed = login_start
        .register(&running_program(&source, b"server"))
        .expect("register");

    assert_eq!(fs::read(installed).expect("copy"), b"server");
    assert!(!partial.exists());
}
