// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const SHIM: &str = "C:\\npm\\claude.cmd";

const CLASSIC: &str = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n  SET PATHEXT=%PATHEXT:;.JS;=;%\r\n)\r\n\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\@x\\cli\\cli.js\" %*\r\n";

const NEWER: &str = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\r\nSET \"_prog=node\"\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\x\\cli.js\" %*\r\n";

fn script_target(interpreters: Vec<Interpreter>, script: &str) -> ShimTarget {
    ShimTarget::Script {
        interpreters,
        script: script.to_owned(),
    }
}

#[test]
fn the_classic_shim_tries_its_own_node_and_then_node_on_the_path() {
    assert_eq!(
        parse_npm_shim(SHIM, CLASSIC),
        Some(script_target(
            vec![
                Interpreter::Path("C:\\npm\\\\node.exe".to_owned()),
                Interpreter::Name("node".to_owned()),
            ],
            "C:\\npm\\\\node_modules\\@x\\cli\\cli.js",
        ))
    );
}

#[test]
fn the_newer_shim_uses_node_on_the_path() {
    assert_eq!(
        parse_npm_shim(SHIM, NEWER),
        Some(script_target(
            vec![Interpreter::Name("node".to_owned())],
            "C:\\npm\\\\node_modules\\x\\cli.js",
        ))
    );
}

#[test]
fn a_quoted_interpreter_and_a_bare_word_are_read_without_the_variable() {
    let quoted = "\"%~dp0\\bun.exe\"  \"%~dp0\\app\\run.mjs\" %*";
    assert_eq!(
        parse_npm_shim(SHIM, quoted),
        Some(script_target(
            vec![Interpreter::Path("C:\\npm\\\\bun.exe".to_owned())],
            "C:\\npm\\\\app\\run.mjs",
        ))
    );
    let bare = "@deno \"%dp0%\\app\\run.cjs\" %*";
    assert_eq!(
        parse_npm_shim(SHIM, bare),
        Some(script_target(
            vec![Interpreter::Name("deno".to_owned())],
            "C:\\npm\\\\app\\run.cjs",
        ))
    );
}

#[test]
fn a_shim_that_points_at_an_exe_gives_that_exe() {
    let content = "@ECHO off\r\n\"%dp0%\\node_modules\\x\\bin\\x.exe\" %*\r\n";
    assert_eq!(
        parse_npm_shim(SHIM, content),
        Some(ShimTarget::Executable(
            "C:\\npm\\\\node_modules\\x\\bin\\x.exe".to_owned()
        ))
    );
}

#[test]
fn the_last_line_with_the_arguments_placeholder_wins() {
    let content = format!("echo \"%dp0%\\other.js\" %*\r\n{NEWER}");
    assert_eq!(parse_npm_shim(SHIM, &content), parse_npm_shim(SHIM, NEWER));
}

#[test]
fn a_launch_line_that_cmd_would_skip_is_rejected() {
    let launch = "node \"%dp0%\\node_modules\\x\\cli.js\" %*";
    for guard in [
        "IF 1==2 ",
        "if not exist x ",
        "GOTO end & ",
        "@REM ",
        "rem ",
        ":: ",
    ] {
        assert_eq!(
            parse_npm_shim(SHIM, &format!("{guard}{launch}")),
            None,
            "{guard:?}"
        );
    }
    assert!(
        parse_npm_shim(SHIM, launch).is_some(),
        "the plain line stays"
    );
}

#[test]
fn an_unknown_interpreter_is_rejected() {
    let content = "SET \"_prog=python\"\r\n\"%_prog%\" \"%dp0%\\x.js\" %*";
    assert_eq!(parse_npm_shim(SHIM, content), None);
    assert_eq!(
        parse_npm_shim(SHIM, "\"C:\\tools\\perl.exe\" \"%dp0%\\x.js\" %*"),
        None
    );
}

#[test]
fn a_shim_without_the_arguments_placeholder_is_rejected() {
    assert_eq!(
        parse_npm_shim(
            SHIM,
            "SET \"_prog=node\"\r\n\"%_prog%\" \"%dp0%\\x.js\"\r\n"
        ),
        None
    );
}

#[test]
fn a_variable_that_is_left_is_rejected() {
    let in_script = "SET \"_prog=node\"\r\n\"%_prog%\" \"%dp0%\\%APP%\\x.js\" %*";
    assert_eq!(parse_npm_shim(SHIM, in_script), None);
    let in_interpreter = "SET \"_prog=%NODE_HOME%\\node.exe\"\r\n\"%_prog%\" \"%dp0%\\x.js\" %*";
    assert_eq!(parse_npm_shim(SHIM, in_interpreter), None);
}

#[test]
fn a_relative_path_is_rejected() {
    assert_eq!(
        parse_npm_shim(SHIM, "node \"node_modules\\x\\cli.js\" %*"),
        None
    );
    assert_eq!(
        parse_npm_shim("claude.cmd", NEWER),
        None,
        "a shim path with no folder cannot give %dp0%"
    );
}

#[test]
fn a_script_of_another_type_is_rejected() {
    assert_eq!(parse_npm_shim(SHIM, "node \"%dp0%\\run.ps1\" %*"), None);
}

#[test]
fn an_oversize_or_nul_file_is_rejected() {
    let padding = " ".repeat(MAX_SHIM_BYTES);
    assert_eq!(parse_npm_shim(SHIM, &format!("{padding}{NEWER}")), None);
    assert_eq!(parse_npm_shim(SHIM, &format!("{NEWER}\0")), None);
}

#[test]
fn text_that_is_not_a_shim_never_panics() {
    for content in [
        "",
        "\"",
        "\"\"",
        "%*",
        "\" %*",
        "\"a\" %*",
        "ß\"ß\"%*",
        "\"\u{1F600}\"%*",
    ] {
        assert_eq!(parse_npm_shim(SHIM, content), None, "{content:?}");
    }
}
