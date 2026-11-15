// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::http::HeaderValue;

use super::*;

const PORT: u16 = 4000;

fn headers(host: Option<&str>, origin: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(host) = host {
        headers.insert(HOST, HeaderValue::from_str(host).expect("host"));
    }
    if let Some(origin) = origin {
        headers.insert(ORIGIN, HeaderValue::from_str(origin).expect("origin"));
    }
    headers
}

#[test]
fn the_loopback_host_with_the_port_and_no_origin_passes() {
    assert_eq!(
        check_upgrade(&headers(Some("127.0.0.1:4000"), None), PORT),
        Ok(())
    );
}

#[test]
fn any_other_host_is_refused() {
    for host in [
        Some("localhost:4000"),
        Some("127.0.0.1:4001"),
        Some("127.0.0.1"),
        Some("evil.example:4000"),
        Some("127.0.0.1:4000.evil.example"),
        None,
    ] {
        assert_eq!(
            check_upgrade(&headers(host, None), PORT),
            Err(UpgradeRefusal::WrongHost),
            "{host:?}"
        );
    }
}

#[test]
fn any_origin_is_refused_even_a_loopback_one() {
    for origin in ["http://127.0.0.1:4000", "null", ""] {
        assert_eq!(
            check_upgrade(&headers(Some("127.0.0.1:4000"), Some(origin)), PORT),
            Err(UpgradeRefusal::OriginPresent),
            "{origin}"
        );
    }
}
