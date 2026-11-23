// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn binding(name: &str, version: u32) -> RecordBinding {
    RecordBinding {
        id: SecretId::from_stored("id-1".to_owned()),
        name: SecretName::parse(name).expect("name"),
        version: SecretVersion::from_stored(version),
    }
}

#[test]
fn the_data_differs_when_the_name_the_version_or_the_id_differs() {
    let base = binding("KEY_A", 1).authenticated_data();
    assert_ne!(base, binding("KEY_B", 1).authenticated_data());
    assert_ne!(base, binding("KEY_A", 2).authenticated_data());
    let mut other_id = binding("KEY_A", 1);
    other_id.id = SecretId::from_stored("id-2".to_owned());
    assert_ne!(base, other_id.authenticated_data());
}
