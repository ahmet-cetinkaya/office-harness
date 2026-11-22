// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};

use super::Fields;

const LIST_REQUEST: &str = "needs_you.list";
const DISMISS_REQUEST: &str = "needs_you.dismiss";

/// `needs_you.list`: takes no field.
#[derive(Debug)]
pub struct ListNeedsYouRequest;

impl ListNeedsYouRequest {
    /// # Errors
    /// The text names the field that the request does not take.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Fields(body).refuse_other_than(LIST_REQUEST, &[])?;
        Ok(Self)
    }
}

/// `needs_you.dismiss`.
#[derive(Debug)]
pub struct DismissNeedsYouRequest {
    pub agent_id: String,
}

impl DismissNeedsYouRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        fields.refuse_other_than(DISMISS_REQUEST, &["agent_id"])?;
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
        })
    }
}
