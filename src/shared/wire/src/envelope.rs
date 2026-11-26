// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;
use serde_json::{Map, Value, json};

/// A client request: `{"id":<u32>,"type":"<name>",...}`. The fields beyond
/// `id` and `type` belong to the handler of the type.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Request {
    pub id: u32,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

impl Request {
    /// Reads a text frame; anything that is not a request is `None`.
    #[must_use]
    pub fn decode(frame: &str) -> Option<Self> {
        serde_json::from_str(frame).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Invalid,
    Conflict,
    NotFound,
    Refused,
    Internal,
}

impl ErrorCode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::Conflict => "conflict",
            Self::NotFound => "not_found",
            Self::Refused => "refused",
            Self::Internal => "internal",
        }
    }
}

/// The answer to one request, with the same `id`.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Success {
        id: u32,
        result: Value,
    },
    Failure {
        id: u32,
        code: ErrorCode,
        message: String,
        /// The failing fields of an `invalid` answer, each with its rule;
        /// left out of the frame when `None`.
        fields: Option<Value>,
    },
}

impl Response {
    #[must_use]
    pub fn success(id: u32, result: Value) -> Self {
        Self::Success { id, result }
    }

    #[must_use]
    pub fn failure(id: u32, code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Failure {
            id,
            code,
            message: message.into(),
            fields: None,
        }
    }

    /// A failure that also names the failing fields (`error.fields`).
    #[must_use]
    pub fn failure_with_fields(
        id: u32,
        code: ErrorCode,
        message: impl Into<String>,
        fields: Option<Value>,
    ) -> Self {
        Self::Failure {
            id,
            code,
            message: message.into(),
            fields,
        }
    }

    /// The JSON text frame.
    #[must_use]
    pub fn encode(&self) -> String {
        match self {
            Self::Success { id, result } => json!({"id": id, "ok": true, "result": result}),
            Self::Failure {
                id,
                code,
                message,
                fields,
            } => {
                let mut error = json!({"code": code.as_str(), "message": message});
                if let Some(fields) = fields {
                    error["fields"] = fields.clone();
                }
                json!({"id": id, "ok": false, "error": error})
            }
        }
        .to_string()
    }
}

/// A server-to-client message without an `id`.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    kind: String,
    fields: Map<String, Value>,
}

impl Event {
    #[must_use]
    pub fn new(kind: &str, fields: Map<String, Value>) -> Self {
        Self {
            kind: kind.to_owned(),
            fields,
        }
    }

    /// The JSON text frame: the fields plus `type`.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut body = self.fields.clone();
        body.insert("type".to_owned(), Value::String(self.kind.clone()));
        Value::Object(body).to_string()
    }
}

#[cfg(test)]
#[path = "envelope.tests.rs"]
mod tests;
