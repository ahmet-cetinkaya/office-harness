// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_sidecar_sits_next_to_the_app() {
    let app = Path::new("install").join("office-harness-desktop");

    let sidecar = sidecar_path(&app);

    assert_eq!(sidecar.parent(), app.parent());
    assert_eq!(
        sidecar.file_name().and_then(|name| name.to_str()),
        Some(format!("office-harnessd{}", std::env::consts::EXE_SUFFIX).as_str())
    );
}

#[cfg(unix)]
#[test]
fn a_detached_program_still_runs_to_completion() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "exit 0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);

    assert!(command.status().expect("sh runs").success());
}

fn answer(exit_code: Option<i32>, stderr: &str) -> SidecarAnswer {
    SidecarAnswer {
        exit_code,
        stderr: stderr.to_owned(),
    }
}

#[test]
fn a_register_that_exits_with_zero_succeeded() {
    assert!(answer_to_success(&answer(Some(0), "")).is_ok());
}

#[test]
fn a_register_failure_carries_the_reason_of_the_sidecar() {
    let error = answer_to_success(&answer(Some(1), "login item: refused")).unwrap_err();

    assert!(error.to_string().contains("login item: refused"));
}

#[test]
fn a_register_that_was_killed_is_a_failure() {
    assert!(answer_to_success(&answer(None, "")).is_err());
}

#[test]
fn status_tells_registered_from_not_registered_and_from_failure() {
    let not_registered = i32::from(LOGIN_START_NOT_REGISTERED_EXIT_CODE);

    assert!(answer_to_registered(&answer(Some(0), "")).unwrap());
    assert!(!answer_to_registered(&answer(Some(not_registered), "")).unwrap());
    assert!(answer_to_registered(&answer(Some(1), "boom")).is_err());
}
