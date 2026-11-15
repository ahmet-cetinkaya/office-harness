// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::additions_input::AdditionsInput;
use serde_json::Value;

/// Reads the `additions` object of a request: `flags` (a list of text),
/// `env` (a list of `{name, value}`), and `startup_prompt` (text or null).
/// Every failure names the part at fault. Whether a name or a flag is valid
/// is the domain's rule, which the use case applies.
///
/// # Errors
/// The text names the field at fault.
pub fn read_additions(value: &Value) -> Result<AdditionsInput, String> {
    let Value::Object(body) = value else {
        return Err("additions must be an object".to_owned());
    };
    let flags = match body.get("flags") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(index, item)| text_of(item, &format!("additions.flags[{index}]")))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err("additions.flags must be a list".to_owned()),
    };
    let env = match body.get("env") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(index, item)| read_entry(item, index))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err("additions.env must be a list".to_owned()),
    };
    let startup_prompt = match body.get("startup_prompt") {
        None | Some(Value::Null) => None,
        Some(item) => Some(text_of(item, "additions.startup_prompt")?),
    };
    Ok(AdditionsInput {
        flags,
        env,
        startup_prompt,
    })
}

fn read_entry(item: &Value, index: usize) -> Result<(String, String), String> {
    let Value::Object(entry) = item else {
        return Err(format!("additions.env[{index}] must be an object"));
    };
    let field = |name: &str| {
        let label = format!("additions.env[{index}].{name}");
        entry
            .get(name)
            .ok_or_else(|| format!("{label} is required"))
            .and_then(|value| text_of(value, &label))
    };
    Ok((field("name")?, field("value")?))
}

fn text_of(value: &Value, label: &str) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        _ => Err(format!("{label} must be text")),
    }
}

#[cfg(test)]
#[path = "additions_body.tests.rs"]
mod tests;
