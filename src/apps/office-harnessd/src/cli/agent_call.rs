// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Map, Value};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use wire::{ClientHandshake, WS_PATH};

/// The subcommand name: `office-harnessd agent-call <type> [<json fields>]`.
pub const AGENT_CALL: &str = "agent-call";

const AGENT_TOKEN_VARIABLE: &str = "OFFICE_HARNESS_AGENT_TOKEN";
const SERVER_FILE_VARIABLE: &str = "OFFICE_HARNESS_SERVER_FILE";
const REQUEST_ID: u32 = 1;
/// Prints the value of `secret.get` alone, so it can go into a pipe or a file
/// descriptor and never into an argument (0026 contract, helper).
const VALUE_ONLY_FLAG: &str = "--value-only";
const SECRET_GET: &str = "secret.get";
const USAGE_EXIT_CODE: u8 = 2;
const FAILURE_EXIT_CODE: u8 = 1;

/// Calls the office from inside an agent's session (0002). It reads the
/// session's call credential and the path of `server.json` from the
/// environment that the server gave the agent, proves the credential, sends
/// one request, prints the answer on standard output, and exits 0 when the
/// answer is a success. A failure goes to standard error and never names the
/// credential.
pub async fn run(arguments: &[OsString]) -> ExitCode {
    match call(arguments).await {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(FAILURE_EXIT_CODE),
        Err(error) => {
            eprintln!("office-harnessd {AGENT_CALL}: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum CallError {
    #[error(
        "usage: office-harnessd {AGENT_CALL} <request type> [<json fields>] ({VALUE_ONLY_FLAG} only with {SECRET_GET})"
    )]
    Usage,
    #[error("the fields are not a JSON object")]
    BadFields,
    #[error("{0} is not set: this command works only inside an agent session")]
    NotInSession(&'static str),
    #[error("the call credential has the wrong shape")]
    BadCredential,
    #[error("cannot read the server file: {0}")]
    ServerFile(String),
    #[error("cannot reach the server: {0}")]
    Connect(String),
    #[error("the server did not accept the credential")]
    NotAccepted,
    #[error("the server did not answer as the office server")]
    NotProven,
    #[error("the connection ended before an answer")]
    NoAnswer,
}

impl CallError {
    fn exit_code(&self) -> u8 {
        match self {
            Self::Usage | Self::BadFields | Self::NotInSession(_) => USAGE_EXIT_CODE,
            _ => FAILURE_EXIT_CODE,
        }
    }
}

/// The request frame: the type and the id, then the caller's fields. The
/// caller cannot override the type or the id.
fn request_frame(kind: &str, fields: Map<String, Value>) -> String {
    let mut body = fields;
    body.insert("type".to_owned(), Value::String(kind.to_owned()));
    body.insert("id".to_owned(), Value::from(REQUEST_ID));
    Value::Object(body).to_string()
}

fn read_fields(text: Option<&OsString>) -> Result<Map<String, Value>, CallError> {
    let Some(text) = text else {
        return Ok(Map::new());
    };
    let text = text.to_str().ok_or(CallError::BadFields)?;
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(fields)) => Ok(fields),
        _ => Err(CallError::BadFields),
    }
}

/// `agent_id:secret`, as the server wrote it.
fn split_credential(text: &str) -> Result<(&str, &str), CallError> {
    text.split_once(':').ok_or(CallError::BadCredential)
}

fn read_port(path: &Path) -> Result<u16, CallError> {
    let text =
        std::fs::read_to_string(path).map_err(|error| CallError::ServerFile(error.to_string()))?;
    let file: Value =
        serde_json::from_str(&text).map_err(|error| CallError::ServerFile(error.to_string()))?;
    file["port"]
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
        .ok_or_else(|| CallError::ServerFile("no port".to_owned()))
}

/// Takes `--value-only` out of the arguments; it is only for `secret.get`.
fn split_value_only(arguments: &[OsString]) -> Result<(bool, Vec<OsString>), CallError> {
    let value_only = arguments.iter().any(|argument| argument == VALUE_ONLY_FLAG);
    let rest: Vec<OsString> = arguments
        .iter()
        .filter(|argument| *argument != VALUE_ONLY_FLAG)
        .cloned()
        .collect();
    let is_secret_get = rest.first().is_some_and(|kind| kind == SECRET_GET);
    if value_only && !is_secret_get {
        return Err(CallError::Usage);
    }
    Ok((value_only, rest))
}

/// What the helper writes for one answer.
#[derive(Debug, PartialEq, Eq)]
struct Rendered {
    accepted: bool,
    stdout: Option<String>,
    stderr: Option<String>,
}

/// With `--value-only` an accepted read gives the value alone, with no line
/// break, and any other answer gives nothing for standard output: its error
/// goes to standard error. Without it the answer is printed as it came.
fn render_answer(answer: &str, value_only: bool) -> Rendered {
    let parsed = serde_json::from_str::<Value>(answer).ok();
    let accepted = parsed
        .as_ref()
        .and_then(|value| value["ok"].as_bool())
        .unwrap_or(false);
    if !value_only {
        return Rendered {
            accepted,
            stdout: Some(format!("{answer}\n")),
            stderr: None,
        };
    }
    let value = parsed
        .as_ref()
        .and_then(|value| value["result"]["value"].as_str());
    if let (true, Some(value)) = (accepted, value) {
        return Rendered {
            accepted,
            stdout: Some(value.to_owned()),
            stderr: None,
        };
    }
    let message = parsed
        .as_ref()
        .and_then(|value| value["error"]["message"].as_str())
        .unwrap_or("the office did not answer with a value");
    Rendered {
        accepted: false,
        stdout: None,
        stderr: Some(format!("office-harnessd {AGENT_CALL}: {message}\n")),
    }
}

fn print_answer(answer: &str, value_only: bool) -> bool {
    let rendered = render_answer(answer, value_only);
    if let Some(text) = &rendered.stdout {
        print!("{text}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
    if let Some(text) = &rendered.stderr {
        eprint!("{text}");
    }
    rendered.accepted
}

async fn call(arguments: &[OsString]) -> Result<bool, CallError> {
    let (value_only, arguments) = split_value_only(arguments)?;
    let (kind, rest) = arguments.split_first().ok_or(CallError::Usage)?;
    let kind = kind.to_str().ok_or(CallError::Usage)?;
    if rest.len() > 1 {
        return Err(CallError::Usage);
    }
    let fields = read_fields(rest.first())?;
    let credential = std::env::var(AGENT_TOKEN_VARIABLE)
        .map_err(|_| CallError::NotInSession(AGENT_TOKEN_VARIABLE))?;
    let server_file = std::env::var_os(SERVER_FILE_VARIABLE)
        .ok_or(CallError::NotInSession(SERVER_FILE_VARIABLE))?;
    let (agent_id, secret) = split_credential(&credential)?;
    let port = read_port(Path::new(&server_file))?;

    let (mut socket, _) = connect_async(format!("ws://127.0.0.1:{port}{WS_PATH}"))
        .await
        .map_err(|error| CallError::Connect(error.to_string()))?;
    let mut handshake = ClientHandshake::for_agent(agent_id, secret, port);
    let challenge = next_text(&mut socket).await.ok_or(CallError::NoAnswer)?;
    let answer = handshake
        .answer_challenge(&challenge)
        .map_err(|_| CallError::NotProven)?;
    socket
        .send(Message::Text(answer.into()))
        .await
        .map_err(|error| CallError::Connect(error.to_string()))?;
    let accepted = next_text(&mut socket).await.ok_or(CallError::NotAccepted)?;
    handshake
        .confirm_server(&accepted)
        .map_err(|_| CallError::NotAccepted)?;

    socket
        .send(Message::Text(request_frame(kind, fields).into()))
        .await
        .map_err(|error| CallError::Connect(error.to_string()))?;
    let answer = next_text(&mut socket).await.ok_or(CallError::NoAnswer)?;
    let accepted = print_answer(&answer, value_only);
    let _ = socket.close(None).await;
    Ok(accepted)
}

async fn next_text<S>(socket: &mut S) -> Option<String>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        match socket.next().await? {
            Ok(Message::Text(text)) => return Some(text.as_str().to_owned()),
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(_) => {}
        }
    }
}

#[cfg(test)]
#[path = "agent_call.tests.rs"]
mod tests;
