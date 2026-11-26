use super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

struct Empty;

impl KeyStore for Empty {
    fn get(&self, _account: &str) -> Result<Option<String>> {
        Ok(None)
    }
    fn set(&self, _account: &str, _value: &str) -> Result<()> {
        bail!("not used")
    }
    fn remove(&self, _account: &str) -> Result<()> {
        bail!("not used")
    }
}

/// Answers one request on loopback and returns the request's header lines.
fn serve(status: &str, extra_header: &str, body: String) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/models", listener.local_addr().unwrap());
    let head = format!("HTTP/1.1 {status}\r\n{extra_header}content-length: {}\r\nconnection: close\r\n\r\n", body.len());
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                break;
            }
            lines.push(line.trim_end().to_ascii_lowercase());
        }
        let mut stream = reader.into_inner();
        // A client that stops reading early closes the connection; that is fine here.
        let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(body.as_bytes()));
        lines
    });
    (url, handle)
}

#[test]
fn each_header_style_is_sent_as_the_provider_expects() {
    for (auth, expected) in [
        (Auth::Bearer, "authorization: bearer sk-test"),
        (Auth::Anthropic, "x-api-key: sk-test"),
        (Auth::Google, "x-goog-api-key: sk-test"),
    ] {
        let (url, server) = serve("200 OK", "", r#"{"data":[]}"#.into());
        assert_eq!(fetch(&url, auth, "sk-test").unwrap(), r#"{"data":[]}"#);
        let headers = server.join().unwrap();
        assert!(headers.iter().any(|line| line == expected), "{auth:?}: {headers:?}");
    }
}

#[test]
fn a_failed_status_reports_only_the_number() {
    let (url, server) = serve("401 Unauthorized", "", "sk-test is wrong".into());
    assert_eq!(fetch(&url, Auth::Bearer, "sk-test").unwrap_err().to_string(), "HTTP 401");
    server.join().unwrap();
}

#[test]
fn a_redirect_is_not_followed() {
    let (url, server) = serve("302 Found", "location: http://127.0.0.1:9/elsewhere\r\n", String::new());
    assert_eq!(fetch(&url, Auth::Bearer, "sk-test").unwrap_err().to_string(), "HTTP 302");
    server.join().unwrap();
}

#[test]
fn a_body_that_repeats_the_key_or_is_too_large_is_not_returned() {
    let (url, server) = serve("200 OK", "", r#"{"echo":"sk-test"}"#.into());
    let error = fetch(&url, Auth::Bearer, "sk-test").unwrap_err().to_string();
    assert!(!error.contains("sk-test"), "{error}");
    server.join().unwrap();
    let (url, server) = serve("200 OK", "", "a".repeat(MAX_LIST_BYTES as usize + 1));
    assert!(fetch(&url, Auth::Bearer, "sk-test").is_err());
    server.join().unwrap();
}

#[test]
fn nothing_is_requested_without_a_stored_key_a_known_list_or_a_person() {
    assert!(list("openai", false, &Empty).unwrap_err().to_string().contains("no key"));
    assert!(list("openai", true, &Empty).unwrap_err().to_string().contains("agent"));
    assert!(list("mistral", false, &Empty).unwrap_err().to_string().contains("no model list"));
    for name in ["custom", "9router", "ollama", "", "../openai"] {
        assert!(list(name, false, &Empty).is_err(), "{name:?}");
    }
}

#[test]
fn every_listed_endpoint_is_a_catalog_provider_over_https() {
    for (name, url, _) in ENDPOINTS {
        assert!(account_for(name).is_ok(), "{name}");
        assert!(url.starts_with("https://"), "{name}");
    }
}
