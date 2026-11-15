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
