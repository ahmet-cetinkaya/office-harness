// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use super::*;

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("an object")
    };
    map
}

#[test]
fn a_missing_required_field_is_named() {
    let body = map(json!({}));

    assert_eq!(
        Fields(&body).required_text("name"),
        Err("name is required".to_owned())
    );
}

#[test]
fn a_field_of_the_wrong_type_is_named() {
    let body = map(json!({"name": 5, "folder": ["x"]}));

    assert_eq!(
        Fields(&body).required_text("name"),
        Err("name must be text".to_owned())
    );
    assert_eq!(
        Fields(&body).optional_text("folder"),
        Err("folder must be text".to_owned())
    );
}

#[test]
fn an_optional_field_may_be_absent_or_null() {
    let body = map(json!({"project_id": null}));

    assert_eq!(Fields(&body).optional_text("project_id"), Ok(None));
    assert_eq!(Fields(&body).optional_text("working_folder"), Ok(None));
}
