use super::*;
use serde_json::json;

fn line(patch: &[(&str, Value)]) -> String {
    let mut message = json!({
        "v": 1, "id": "m-1", "stream": "chat", "type": "message.append",
        "origin": "yana-app", "object": "chat-1", "rev": 2, "body": {"text": "xin chào"}
    });
    for (key, value) in patch {
        message[*key] = value.clone();
    }
    message.to_string()
}

fn hello(protocols: &[u32]) -> Hello {
    Hello {
        app: "yana-studio".into(),
        app_version: "1.0.0".into(),
        protocols: protocols.to_vec(),
        capabilities: vec![],
    }
}

#[test]
fn reads_a_valid_message() {
    let envelope = parse_envelope(&line(&[])).expect("valid line");
    assert_eq!(envelope.stream, Stream::Chat);
    assert_eq!(envelope.kind, "message.append");
    assert_eq!(envelope.rev, Some(2));
}

#[test]
fn ignores_fields_it_does_not_know() {
    assert!(parse_envelope(&line(&[("from_the_future", json!([1, 2]))])).is_ok());
}

#[test]
fn refuses_empty_and_broken_lines() {
    for bad in ["", " ", "{", "null", "[]", "{\"v\":1}", "\u{0}"] {
        assert!(
            matches!(parse_envelope(bad), Err(ProtocolError::Malformed(_))),
            "{bad:?}"
        );
    }
}

#[test]
fn refuses_a_line_over_the_limit_before_parsing_it() {
    let huge = "a".repeat(MAX_LINE_BYTES + 1);
    assert_eq!(
        parse_envelope(&huge),
        Err(ProtocolError::TooLarge {
            bytes: MAX_LINE_BYTES + 1
        })
    );
    assert!(matches!(
        parse_hello(&huge),
        Err(ProtocolError::TooLarge { .. })
    ));
}

#[test]
fn deep_nesting_is_an_error_not_a_crash() {
    let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    assert!(parse_envelope(&deep).is_err());
}

#[test]
fn errors_never_repeat_the_content_of_the_line() {
    let error = parse_envelope("{\"api_key\": \"sk-very-secret\", oops").expect_err("broken");
    assert!(!error.to_string().contains("sk-very-secret"), "{error}");
}

#[test]
fn refuses_unsafe_names() {
    let long = "a".repeat(129);
    let bad = [
        "",
        "..",
        ".",
        "../etc/passwd",
        "a b",
        "a\u{0}b",
        "a\r\nb",
        "$(id)",
        "tên",
        long.as_str(),
    ];
    for field in ["id", "type", "origin", "object"] {
        for value in bad {
            let result = parse_envelope(&line(&[(field, json!(value))]));
            assert!(
                matches!(result, Err(ProtocolError::BadName { .. })),
                "{field}={value:?}"
            );
        }
    }
    assert!(parse_envelope(&line(&[("id", json!("a".repeat(128)))])).is_ok());
}

#[test]
fn refuses_another_protocol_version() {
    for version in [0, 2, u32::MAX] {
        let result = parse_envelope(&line(&[("v", json!(version))]));
        assert_eq!(result, Err(ProtocolError::UnsupportedVersion(version)));
    }
    assert!(parse_envelope(&line(&[("v", json!(-1))])).is_err());
}

#[test]
fn control_stream_only_carries_known_messages() {
    for kind in CONTROL_KINDS {
        let allowed = line(&[("stream", json!("control")), ("type", json!(kind))]);
        assert!(parse_envelope(&allowed).is_ok(), "{kind}");
    }
    let unknown = line(&[("stream", json!("control")), ("type", json!("run.command"))]);
    assert_eq!(
        parse_envelope(&unknown),
        Err(ProtocolError::UnknownControl("run.command".into()))
    );
    assert!(parse_envelope(&line(&[("stream", json!("root"))])).is_err());
}

#[test]
fn hello_is_checked_and_capped() {
    let ok = json!({"app": "notch", "app_version": "1.0.0", "protocols": [1], "capabilities": ["approve"]});
    assert!(parse_hello(&ok.to_string()).is_ok());
    let many = json!({"app": "notch", "app_version": "1", "protocols": vec![1u32; 17]});
    assert_eq!(
        parse_hello(&many.to_string()),
        Err(ProtocolError::TooMany { field: "protocols" })
    );
    let bad = json!({"app": "../x", "app_version": "1", "protocols": [1]});
    assert_eq!(
        parse_hello(&bad.to_string()),
        Err(ProtocolError::BadName { field: "app" })
    );
}

#[test]
fn negotiation_picks_the_highest_shared_version() {
    assert_eq!(negotiate(&[1, 2], &hello(&[1, 2, 3])), Ok(2));
    assert_eq!(negotiate(&[1], &hello(&[1])), Ok(1));
}

#[test]
fn negotiation_names_the_side_that_must_update() {
    let newer_app = negotiate(&[1], &hello(&[2])).expect_err("app is newer");
    assert!(
        newer_app.to_string().contains("update yana-rt"),
        "{newer_app}"
    );
    let older_app = negotiate(&[2], &hello(&[1])).expect_err("app is older");
    assert!(
        older_app.to_string().contains("update the app"),
        "{older_app}"
    );
    assert!(negotiate(&[1], &hello(&[])).is_err());
}
