// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn every_result_survives_its_text() {
    let results = [
        ExitResult::Code(0),
        ExitResult::Code(137),
        ExitResult::Signal,
        ExitResult::Interrupted,
        ExitResult::Lost,
        ExitResult::SpawnError("not found".to_owned()),
    ];
    for result in results {
        assert_eq!(result.to_string().parse(), Ok(result));
    }
}

#[test]
fn an_unknown_text_is_refused() {
    assert_eq!(
        "later".parse::<ExitResult>(),
        Err(AgentError::UnknownExitResult("later".to_owned()))
    );
}

#[test]
fn lost_is_stored_as_lost_and_differs_from_interrupted() {
    assert_eq!(ExitResult::Lost.to_string(), "lost");
    assert_eq!("lost".parse(), Ok(ExitResult::Lost));
    assert_ne!(ExitResult::Lost, ExitResult::Interrupted);
}
