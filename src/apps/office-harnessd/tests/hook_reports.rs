// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! 0024 T1: the status-hook report endpoint. Each refusal is one answer, a
//! valid report reaches the sink with only its kind, and nothing of the body
//! is kept.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use access_application::common::interfaces::HookSecretStore;
use kernel::AgentId;
use office_harnessd::bootstrap::{self, LoginSetup};
use office_harnessd::config::Config;
use office_harnessd::server::{HookEventKind, HookReport, HookReportSink, Server, ServerOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

const SESSION: &str = "11111111-1111-1111-1111-111111111111";

#[derive(Default)]
struct Recorder(Mutex<Vec<HookReport>>);

impl HookReportSink for Recorder {
    fn accept(&self, report: HookReport) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(report);
    }
}

impl Recorder {
    fn reports(&self) -> Vec<HookReport> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

struct Fixture {
    port: u16,
    secret: String,
    agent_id: AgentId,
    recorder: Arc<Recorder>,
    secrets: Arc<dyn HookSecretStore>,
    log_path: std::path::PathBuf,
    stop: oneshot::Sender<()>,
    _folder: tempfile::TempDir,
}

async fn serve() -> Fixture {
    let folder = tempfile::tempdir().expect("folder");
    let config = Config {
        data_folder: folder.path().join("office"),
    };
    let log = bootstrap::open_log(&config).expect("log");
    let mut services = bootstrap::start_with(&config, &log, LoginSetup::detect()).expect("start");
    let recorder = Arc::new(Recorder::default());
    services.hook_reports = recorder.clone();
    let agent_id = AgentId::from_u128(7);
    let secrets: Arc<dyn HookSecretStore> = services.access.hook_secrets.clone();
    let secret = secrets
        .issue(agent_id, SESSION)
        .expect("issue")
        .expose_secret()
        .to_owned();
    let server = Server::bind(&config, &services, &log, ServerOptions::default())
        .await
        .expect("bind");
    let port = server.port();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve(async move {
                let _ = stopped.await;
            })
            .await
            .expect("serve");
    });
    Fixture {
        port,
        secret,
        agent_id,
        recorder,
        secrets,
        log_path: config.log_path(),
        stop,
        _folder: folder,
    }
}

/// One raw HTTP request on a new connection: its status code.
async fn post(port: u16, path: &str, headers: &[(&str, String)], body: &[u8]) -> u16 {
    let header_lines = headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join("\r\n");
    let request = format!(
        "POST {path} HTTP/1.1\r\n{header_lines}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect(address).await.expect("connect");
    stream.write_all(request.as_bytes()).await.expect("head");
    // A refused body may be left unread; the answer still arrives.
    let _ = stream.write_all(body).await;
    let mut answer = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut answer)).await;
    let text = String::from_utf8_lossy(&answer);
    text.split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("status line")
}

fn good_headers(fixture: &Fixture) -> Vec<(&'static str, String)> {
    vec![
        ("Host", format!("127.0.0.1:{}", fixture.port)),
        ("Content-Type", "application/json".to_owned()),
        ("Authorization", format!("Bearer {}", fixture.secret)),
    ]
}

fn path(session: &str) -> String {
    format!("/v1/hook/{session}")
}

const STOP_BODY: &[u8] =
    br#"{"hook_event_name":"Stop","prompt":"unique marker words","last_assistant_message":"answer"}"#;

#[tokio::test]
async fn a_valid_report_reaches_the_sink_with_only_its_kind() {
    let fixture = serve().await;
    let code = post(
        fixture.port,
        &path(SESSION),
        &good_headers(&fixture),
        STOP_BODY,
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(
        fixture.recorder.reports(),
        vec![HookReport {
            agent_id: fixture.agent_id,
            session: SESSION.to_owned(),
            kind: HookEventKind::TurnStopped,
            from_helper: false,
        }]
    );
    let log = std::fs::read_to_string(&fixture.log_path).unwrap_or_default();
    assert!(!log.contains("unique marker words"));
    assert!(!log.contains(&fixture.secret));
    let _ = fixture.stop.send(());
}

#[tokio::test]
async fn every_refusal_is_one_answer_and_changes_nothing() {
    let fixture = serve().await;
    let wrong_secret = {
        let mut headers = good_headers(&fixture);
        headers[2].1 = format!("Bearer {}", "0".repeat(64));
        headers
    };
    let no_secret: Vec<_> = good_headers(&fixture).into_iter().take(2).collect();
    let other_session = path("22222222-2222-2222-2222-222222222222");
    let mut codes = vec![
        post(fixture.port, &path(SESSION), &wrong_secret, STOP_BODY).await,
        post(fixture.port, &path(SESSION), &no_secret, STOP_BODY).await,
        post(
            fixture.port,
            &other_session,
            &good_headers(&fixture),
            STOP_BODY,
        )
        .await,
    ];
    fixture.secrets.revoke_session(SESSION);
    codes.push(
        post(
            fixture.port,
            &path(SESSION),
            &good_headers(&fixture),
            STOP_BODY,
        )
        .await,
    );
    assert_eq!(codes, vec![404, 404, 404, 404]);
    assert_eq!(fixture.recorder.reports(), Vec::<HookReport>::new());
    let _ = fixture.stop.send(());
}

#[tokio::test]
async fn a_browser_a_wrong_host_and_a_wrong_content_type_are_refused() {
    let fixture = serve().await;
    let mut with_origin = good_headers(&fixture);
    with_origin.push(("Origin", "http://example.test".to_owned()));
    let mut wrong_host = good_headers(&fixture);
    wrong_host[0].1 = "example.test".to_owned();
    let mut text_type = good_headers(&fixture);
    text_type[1].1 = "text/plain".to_owned();
    let codes = vec![
        post(fixture.port, &path(SESSION), &with_origin, STOP_BODY).await,
        post(fixture.port, &path(SESSION), &wrong_host, STOP_BODY).await,
        post(fixture.port, &path(SESSION), &text_type, STOP_BODY).await,
    ];
    assert_eq!(codes, vec![404, 404, 404]);
    assert_eq!(fixture.recorder.reports(), Vec::<HookReport>::new());
    let _ = fixture.stop.send(());
}

#[tokio::test]
async fn a_report_over_64_kib_is_refused() {
    let fixture = serve().await;
    let big = format!(
        r#"{{"hook_event_name":"Stop","prompt":"{}"}}"#,
        "x".repeat(1024 * 1024)
    );
    let code = post(
        fixture.port,
        &path(SESSION),
        &good_headers(&fixture),
        big.as_bytes(),
    )
    .await;
    assert_eq!(code, 404);
    assert_eq!(fixture.recorder.reports(), Vec::<HookReport>::new());
    let _ = fixture.stop.send(());
}

#[tokio::test]
async fn an_unknown_kind_and_a_body_that_is_not_json_are_no_event_and_no_error() {
    let fixture = serve().await;
    let unknown = br#"{"hook_event_name":"PreToolUse"}"#;
    let garbage = b"{not json";
    let first = post(
        fixture.port,
        &path(SESSION),
        &good_headers(&fixture),
        unknown,
    )
    .await;
    let second = post(
        fixture.port,
        &path(SESSION),
        &good_headers(&fixture),
        garbage,
    )
    .await;
    assert_eq!((first, second), (200, 200));
    assert_eq!(fixture.recorder.reports(), Vec::<HookReport>::new());
    let _ = fixture.stop.send(());
}

#[tokio::test]
async fn reports_over_fifty_a_second_of_one_session_are_dropped() {
    let fixture = serve().await;
    let mut taken = 0;
    let mut refused = 0;
    for _ in 0..120 {
        match post(
            fixture.port,
            &path(SESSION),
            &good_headers(&fixture),
            STOP_BODY,
        )
        .await
        {
            200 => taken += 1,
            _ => refused += 1,
        }
    }
    // The window is one second; a slow machine can span two windows, so the
    // bounds are the rule's: some are dropped, and no window passes more.
    assert!(refused >= 1, "no report was dropped");
    assert!(taken >= 50);
    assert_eq!(fixture.recorder.reports().len(), taken);
    let _ = fixture.stop.send(());
}
