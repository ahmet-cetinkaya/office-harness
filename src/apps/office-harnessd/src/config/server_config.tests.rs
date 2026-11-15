// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn arguments(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn the_flag_chooses_the_data_folder() {
    let data_folder = std::env::temp_dir().join("office");

    let config =
        Config::load([OsString::from("--data-folder"), data_folder.clone().into()]).expect("valid");

    assert_eq!(config.data_folder, data_folder);
    assert_eq!(config.database_path(), data_folder.join("office.db"));
    assert_eq!(config.lock_path(), data_folder.join("server.lock"));
    assert_eq!(config.log_path(), data_folder.join("server.log"));
}

#[test]
fn rejects_an_unknown_argument() {
    assert_eq!(
        Config::load(arguments(&["--verbose"])),
        Err(ConfigError::UnknownArgument("--verbose".to_owned()))
    );
}

#[test]
fn rejects_a_flag_without_a_value() {
    assert_eq!(
        Config::load(arguments(&["--data-folder"])),
        Err(ConfigError::MissingValue("--data-folder"))
    );
}
