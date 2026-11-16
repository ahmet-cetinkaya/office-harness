// SPDX-License-Identifier: AGPL-3.0-or-later

//! npm installs a CLI on Windows as a `.cmd` shim that starts the real
//! program. Running the shim through `cmd.exe` makes a stop end slowly, since
//! the interrupt key turns into the question "Terminate batch job (Y/N)?". So
//! the launch reads the shim, and when it has the known npm shape, starts the
//! interpreter and the script it names directly.
//!
//! Paths here are Windows paths held as text, so the parser behaves the same
//! on every OS and the tests run everywhere.

use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_SHIM_BYTES: usize = 8 * 1024;
const ARGUMENTS_PLACEHOLDER: &str = "%*";
const PROGRAM_VARIABLE: &str = "%_prog%";
const PROGRAM_ASSIGNMENT_PREFIX: &str = "SET \"_prog=";
const SHIM_FOLDER_VARIABLES: [&str; 2] = ["%~dp0", "%dp0%"];
const PATH_VARIABLE: &str = "PATH";
const EXECUTABLE_SUFFIX: &str = ".exe";
const ALLOWED_INTERPRETERS: [&str; 3] = ["node", "bun", "deno"];
const SCRIPT_SUFFIXES: [&str; 3] = [".js", ".cjs", ".mjs"];
const INTERPRETER_SUFFIXES: [&str; 2] = [".exe", ".com"];

/// Where the shim sends the launch.
#[derive(Debug, PartialEq, Eq)]
pub enum ShimTarget {
    /// The shim starts an executable file, which then gets the arguments.
    Executable(String),
    /// The shim starts a script with an interpreter. The candidates come in
    /// the order to try them.
    Script {
        interpreters: Vec<Interpreter>,
        script: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Interpreter {
    /// An absolute path to the interpreter.
    Path(String),
    /// A bare name, such as `node`, to find on the `PATH` of the child.
    Name(String),
}

/// Reads the target of an npm shim and checks that it exists. `None` when
/// anything is missing, so the caller runs the shim through `cmd.exe`.
#[must_use]
pub fn resolve_npm_shim(
    shim_path: &str,
    arguments: &[String],
    environment: &[(String, String)],
) -> Option<(String, Vec<String>)> {
    let content = read_small_text_file(Path::new(shim_path))?;
    match parse_npm_shim(shim_path, &content)? {
        ShimTarget::Executable(program) => Path::new(&program)
            .is_file()
            .then(|| (program, arguments.to_vec())),
        ShimTarget::Script {
            interpreters,
            script,
        } => {
            if !Path::new(&script).is_file() {
                return None;
            }
            let interpreter = interpreters
                .iter()
                .find_map(|candidate| find_interpreter(candidate, environment))?;
            let mut launch_arguments = vec![script];
            launch_arguments.extend_from_slice(arguments);
            Some((interpreter, launch_arguments))
        }
    }
}

/// Parses the last line of the shim that passes the arguments on (`%*`).
/// Returns `None` for anything that is not an npm shim of a known
/// interpreter, or that cannot be resolved to absolute paths without
/// variables.
#[must_use]
pub fn parse_npm_shim(shim_path: &str, content: &str) -> Option<ShimTarget> {
    if content.len() > MAX_SHIM_BYTES || content.contains('\0') {
        return None;
    }
    let shim_folder = folder_of(shim_path)?;
    let line = content
        .lines()
        .rfind(|line| line.contains(ARGUMENTS_PLACEHOLDER))?;
    let before_arguments = &line[..line.rfind(ARGUMENTS_PLACEHOLDER)?];
    let (head, script) = split_last_quoted(before_arguments)?;
    let script = expand_absolute(script, shim_folder)?;
    let script_lower = script.to_ascii_lowercase();
    if script_lower.ends_with(EXECUTABLE_SUFFIX) {
        return Some(ShimTarget::Executable(script));
    }
    if !SCRIPT_SUFFIXES
        .iter()
        .any(|suffix| script_lower.ends_with(suffix))
    {
        return None;
    }
    let interpreters = interpreters_before(head, content, shim_folder)?;
    Some(ShimTarget::Script {
        interpreters,
        script,
    })
}

/// The interpreter token that stands right before the script: a quoted one,
/// or else the single bare word.
fn interpreters_before(head: &str, content: &str, shim_folder: &str) -> Option<Vec<Interpreter>> {
    let head = head.trim_end();
    let token = match split_last_quoted(head) {
        Some((_, quoted)) if head.ends_with('"') => quoted,
        // A batch line may start its command with `@` to hide the echo.
        _ => head.split_whitespace().next_back()?.trim_start_matches('@'),
    };
    let values = if token == PROGRAM_VARIABLE {
        program_assignments(content)
    } else {
        vec![token]
    };
    let interpreters: Vec<Interpreter> = values
        .into_iter()
        .map(|value| interpreter_of(value, shim_folder))
        .collect::<Option<_>>()?;
    (!interpreters.is_empty()).then_some(interpreters)
}

fn interpreter_of(value: &str, shim_folder: &str) -> Option<Interpreter> {
    let interpreter = if value.contains(['\\', '/', ':']) || value.contains('%') {
        Interpreter::Path(expand_absolute(value, shim_folder)?)
    } else {
        Interpreter::Name(value.to_owned())
    };
    let name = match &interpreter {
        Interpreter::Path(path) => file_name_of(path),
        Interpreter::Name(name) => name.as_str(),
    };
    let name = name.to_ascii_lowercase();
    let name = name.strip_suffix(EXECUTABLE_SUFFIX).unwrap_or(&name);
    ALLOWED_INTERPRETERS.contains(&name).then_some(interpreter)
}

/// The values of every `SET "_prog=..."` line, in file order.
fn program_assignments(content: &str) -> Vec<&str> {
    content
        .lines()
        .filter_map(|line| {
            let start = line.find(PROGRAM_ASSIGNMENT_PREFIX)? + PROGRAM_ASSIGNMENT_PREFIX.len();
            let rest = &line[start..];
            Some(&rest[..rest.find('"')?])
        })
        .collect()
}

/// Splits `text` at its last double-quoted token: what stands before the
/// opening quote, and what the quotes hold.
fn split_last_quoted(text: &str) -> Option<(&str, &str)> {
    let closing = text.rfind('"')?;
    let opening = text[..closing].rfind('"')?;
    Some((&text[..opening], &text[opening + 1..closing]))
}

/// The folder of the shim, without a trailing separator.
fn folder_of(shim_path: &str) -> Option<&str> {
    shim_path.rsplit_once(['\\', '/']).map(|(folder, _)| folder)
}

fn file_name_of(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// Gives the shim folder (with a backslash) for its variables, and accepts
/// only an absolute drive path with no variable left.
fn expand_absolute(value: &str, shim_folder: &str) -> Option<String> {
    let folder_with_separator = format!("{shim_folder}\\");
    let mut expanded = value.to_owned();
    for variable in SHIM_FOLDER_VARIABLES {
        expanded = expanded.replace(variable, &folder_with_separator);
    }
    (!expanded.contains('%') && is_absolute_windows_path(&expanded)).then_some(expanded)
}

fn is_absolute_windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn find_interpreter(candidate: &Interpreter, environment: &[(String, String)]) -> Option<String> {
    let path = match candidate {
        Interpreter::Path(path) => path.clone(),
        Interpreter::Name(name) => {
            find_on_path(&format!("{name}{EXECUTABLE_SUFFIX}"), environment)?
        }
    };
    let lower = path.to_ascii_lowercase();
    let is_program = INTERPRETER_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix));
    (is_program && Path::new(&path).is_file()).then_some(path)
}

/// Looks only at the `PATH` of the child, because that is the one the
/// shim would have used.
fn find_on_path(file_name: &str, environment: &[(String, String)]) -> Option<String> {
    let (_, search_path) = environment
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(PATH_VARIABLE))?;
    std::env::split_paths(search_path)
        .map(|folder| folder.join(file_name))
        .find(|candidate| candidate.is_file())
        .and_then(|found| found.to_str().map(str::to_owned))
}

/// At most `MAX_SHIM_BYTES` of a regular file, so a large file or a device is
/// never read in full. A longer file gives `None`.
fn read_small_text_file(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(u64::try_from(MAX_SHIM_BYTES).ok()? + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > MAX_SHIM_BYTES {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
#[path = "npm_shim.tests.rs"]
mod tests;
