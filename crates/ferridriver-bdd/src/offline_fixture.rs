//! Offline-safe stand-ins for canonical public harness URLs (`https://example.org`,
//! `https://www.google.com`).
//!
//! Real network access to those hosts is flaky in CI and air-gapped dev (TLS, DNS).
//! Steps that navigate to exact harness URLs transparently redirect to loopback HTTP
//! pages that mirror the DOM/title assertions in `tests/features/`.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::thread;

static FIXTURE_ORIGIN_BASE: OnceLock<String> = OnceLock::new();

/// HTML loosely matching https://example.org ("Example Domain" title, dual `<p>`,
/// illustrative copy) so BDD assertions stay unchanged.
static EXAMPLE_DOMAIN_HTML: &[u8] = br#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"/><title>Example Domain</title></head><body><div><h1>Example Domain</h1><p>This domain is for use in illustrative examples in documents without prior coordination or asking for permission.</p><p><a href="/">Learn more</a></p></div></body></html>"#;

/// Minimal page whose title contains "Google", with `h1` / `p` / `body` for visibility outlines.
static GOOGLE_MIRROR_HTML: &[u8] = br#"<!DOCTYPE html><html><head><meta charset="utf-8"/><title>Google Search</title></head><body><h1>Google</h1><p>Harness mirror for offline scenarios.</p></body></html>"#;

pub(crate) fn ensure_fixture_origin_base() -> String {
  FIXTURE_ORIGIN_BASE
    .get_or_init(|| spawn_fixture_listener())
    .clone()
}

/// Public helper for crates that bypass the `Given I navigate to {string}` step (e.g. custom steps).
///
/// Rewrites **`https://example.org`**, **`https://example.org/`**, and **`https://www.google.com`**
/// to the live loopback harness. Any other URL is returned unchanged.
pub fn substitute_public_navigation_url(url: &str) -> String {
  let t = trim_url(url);
  map_https_example_or_google(t).unwrap_or_else(|| t.to_string())
}

pub(crate) fn map_https_example_or_google(url: &str) -> Option<String> {
  let base = ensure_fixture_origin_base();
  match url {
    "https://example.org" | "https://example.org/" => Some(format!("{base}/")),
    "https://www.google.com" | "https://www.google.com/" => Some(format!("{base}/google")),
    _ => None,
  }
}

pub(crate) fn resolve_url_expectation(expected: &str) -> String {
  let trimmed = trim_url(expected);
  map_https_example_or_google(trimmed).unwrap_or_else(|| expected.to_string())
}

/// `Then the URL should contain "example.org"` — tolerate the loopback shim.
pub(crate) fn url_contains_accepts_substitution(expected: &str, actual: &str) -> bool {
  if expected.contains("example.org") {
    if let Some(pref) = FIXTURE_ORIGIN_BASE.get().map(String::as_str) {
      return actual.starts_with(pref);
    }
  }
  false
}

pub(crate) fn rewrite_saved_storage_aliases(state: &mut serde_json::Value) {
  let base = ensure_fixture_origin_base();
  let cookie_host = cookie_host_from_origin(&base);

  if let Some(origins) = state.get_mut("origins").and_then(|x| x.as_array_mut()) {
    for origin_entry in origins {
      if origin_entry.get("origin").and_then(|o| o.as_str()) == Some("https://example.org") {
        origin_entry["origin"] = serde_json::json!(base);
      }
    }
  }

  if let Some(cookies) = state.get_mut("cookies").and_then(|x| x.as_array_mut()) {
    for c in cookies {
      if c.get("domain").and_then(|x| x.as_str()) == Some("example.org") {
        c["domain"] = serde_json::json!(cookie_host);
      }
    }
  }
}

fn trim_url(url: &str) -> &str {
  url.trim_matches(|c| c == '"' || c == '\'' || char::is_whitespace(c)).trim()
}

fn cookie_host_from_origin(origin: &str) -> String {
  origin
    .strip_prefix("http://")
    .or_else(|| origin.strip_prefix("https://"))
    .unwrap_or(origin)
    .split('/')
    .next()
    .unwrap_or("")
    .split(':')
    .next()
    .unwrap_or("")
    .to_string()
}

fn spawn_fixture_listener() -> String {
  let listener = TcpListener::bind("127.0.0.1:0").expect("offline fixture TcpListener bind");
  let base = format!("http://{}", listener.local_addr().expect("fixture local_addr"));

  thread::spawn(move || {
    for incoming in listener.incoming() {
      let Ok(mut stream) = incoming else {
        continue;
      };
      let _ = handle_fixture_connection(&mut stream);
    }
  });

  base
}

fn handle_fixture_connection(stream: &mut TcpStream) -> std::io::Result<()> {
  let mut buf = [0_u8; 4096];
  let n = stream.read(&mut buf)?;
  let req = core::str::from_utf8(&buf[..n]).unwrap_or("");
  let path_line = req.lines().next().unwrap_or("");
  let path = path_line
    .split_whitespace()
    .nth(1)
    .and_then(|p| Some(p.split('?').next().unwrap_or(p)))
    .unwrap_or("/");

  let (status, ctype, body): (&str, &[u8], &[u8]) = match path {
    "/google" | "/google/" => ("200 OK", b"text/html; charset=utf-8", GOOGLE_MIRROR_HTML),
    "/" | "" => ("200 OK", b"text/html; charset=utf-8", EXAMPLE_DOMAIN_HTML),
    _ => ("404 Not Found", b"text/plain", b"fixture 404"),
  };

  let response = format!(
    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: ",
    body.len(),
  );
  stream.write_all(response.as_bytes())?;
  stream.write_all(ctype)?;
  stream.write_all(b"\r\nConnection: close\r\n\r\n")?;
  stream.write_all(body)?;
  stream.flush()
}
