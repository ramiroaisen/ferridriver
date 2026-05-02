//! Structured error taxonomy for ferridriver's public API.
//!
//! Mirrors Playwright's error shape (`packages/playwright-core/src/client/errors.ts`)
//! so consumers can distinguish classes of failure via [`FerriError::is_timeout_error`]
//! / [`FerriError::is_target_closed_error`] and so NAPI consumers receive an
//! `error.name` that matches Playwright (`"TimeoutError"`, `"TargetClosedError"`).
//!
//! The enum is the single public error type. Internal modules should convert
//! their native errors into a [`FerriError`] variant at the public API boundary.

use thiserror::Error;

/// Every user-facing failure mode in the ferridriver core.
///
/// Any `Display` output of this type is the exact string that flows into
/// `error.message` on the NAPI surface; NAPI consumers use [`FerriError::name`]
/// to dispatch on error class. Keep the message wording aligned with Playwright
/// wherever possible so error-matching tests ported from Playwright keep working.
#[derive(Debug, Error)]
pub enum FerriError {
  /// Operation did not complete within its deadline. Mirrors Playwright's
  /// `TimeoutError` — message format `"Timeout {timeout_ms}ms exceeded"` with
  /// an optional `"while {operation}"` suffix.
  #[error("Timeout {timeout_ms}ms exceeded{}", .operation.as_ref().map(|op| format!(" while {op}")).unwrap_or_default())]
  Timeout {
    /// Short phrase describing what was being waited on, e.g. `"navigating to https://..."`.
    operation: Option<String>,
    timeout_ms: u64,
  },

  /// Target page, context, browser, or session has been closed. Mirrors
  /// Playwright's `TargetClosedError`.
  #[error("Target page, context or browser has been closed{}", .reason.as_ref().map(|r| format!(": {r}")).unwrap_or_default())]
  TargetClosed { reason: Option<String> },

  /// Locator resolved to more than one element under strict-mode evaluation.
  /// Playwright raises a plain `Error` with a specific message format; we
  /// surface it as a dedicated variant and mirror the message.
  #[error("strict mode violation: selector {selector:?} resolved to {count} elements")]
  StrictModeViolation { selector: String, count: usize },

  /// Navigation-specific failure (DNS, TLS, `ERR_ABORTED`, etc.).
  #[error("navigation to {url} failed: {message}")]
  Navigation { url: String, message: String },

  /// CDP/BiDi/WebKit protocol error surfaced by the transport layer.
  #[error("protocol error ({method}): {message}")]
  Protocol { method: String, message: String },

  /// Backend-level failure not otherwise classified (launch, connect, pipe
  /// read failure, etc.).
  #[error("backend error: {0}")]
  Backend(String),

  /// Selector string could not be parsed or contains an unknown engine.
  #[error("invalid selector {selector:?}: {reason}")]
  InvalidSelector { selector: String, reason: String },

  /// Caller is not connected to any browser target.
  #[error("not connected")]
  NotConnected,

  /// Long-running operation was cancelled by the caller or supervisor.
  #[error("interrupted: {0}")]
  Interrupted(String),

  /// Caller passed an argument that did not pass validation.
  #[error("invalid argument {name:?}: {reason}")]
  InvalidArgument { name: String, reason: String },

  /// Feature requested is valid Playwright API but not yet implemented for
  /// the active backend.
  #[error("unsupported operation: {0}")]
  Unsupported(String),

  /// `page.evaluate` / `locator.evaluate` threw inside the page.
  #[error("evaluation error: {0}")]
  Evaluation(String),

  /// Snapshot read/write/compare failure.
  #[error("snapshot error: {0}")]
  Snapshot(String),

  /// Filesystem / non-CDP I/O error.
  #[error("io error: {0}")]
  Io(#[from] std::io::Error),

  /// JSON (de)serialization error from the protocol layer.
  #[error("json error: {0}")]
  Json(#[from] serde_json::Error),

  /// Rare catch-all when no other variant applies (prefer [`Self::Backend`],
  /// [`Self::Protocol`], etc. at construction sites).
  #[error("{0}")]
  Other(String),
}

impl Clone for FerriError {
  fn clone(&self) -> Self {
    match self {
      Self::Timeout { operation, timeout_ms } => Self::Timeout {
        operation: operation.clone(),
        timeout_ms: *timeout_ms,
      },
      Self::TargetClosed { reason } => Self::TargetClosed { reason: reason.clone() },
      Self::StrictModeViolation { selector, count } => Self::StrictModeViolation {
        selector: selector.clone(),
        count: *count,
      },
      Self::Navigation { url, message } => Self::Navigation {
        url: url.clone(),
        message: message.clone(),
      },
      Self::Protocol { method, message } => Self::Protocol {
        method: method.clone(),
        message: message.clone(),
      },
      Self::Backend(msg) => Self::Backend(msg.clone()),
      Self::InvalidSelector { selector, reason } => Self::InvalidSelector {
        selector: selector.clone(),
        reason: reason.clone(),
      },
      Self::NotConnected => Self::NotConnected,
      Self::Interrupted(msg) => Self::Interrupted(msg.clone()),
      Self::InvalidArgument { name, reason } => Self::InvalidArgument {
        name: name.clone(),
        reason: reason.clone(),
      },
      Self::Unsupported(msg) => Self::Unsupported(msg.clone()),
      Self::Evaluation(msg) => Self::Evaluation(msg.clone()),
      Self::Snapshot(msg) => Self::Snapshot(msg.clone()),
      Self::Io(e) => Self::Io(std::io::Error::new(e.kind(), e.to_string())),
      Self::Json(e) => Self::Backend(format!("json error: {e}")),
      Self::Other(msg) => Self::Other(msg.clone()),
    }
  }
}

impl FerriError {
  /// Matches Playwright's `isTimeoutError(err)` helper.
  #[must_use]
  pub fn is_timeout_error(&self) -> bool {
    matches!(self, Self::Timeout { .. })
  }

  /// Matches Playwright's `TargetClosedError` detection.
  #[must_use]
  pub fn is_target_closed_error(&self) -> bool {
    matches!(self, Self::TargetClosed { .. })
  }

  /// Matches Playwright's strict-mode violation detection.
  #[must_use]
  pub fn is_strict_mode_violation(&self) -> bool {
    matches!(self, Self::StrictModeViolation { .. })
  }

  /// The `name` attribute mirrored on the JS side via NAPI.
  ///
  /// `"TimeoutError"` / `"TargetClosedError"` match Playwright's names exactly;
  /// everything else reports `"FerriError"` so TS consumers can fall back to
  /// message-based matching without colliding with Playwright class names.
  #[must_use]
  pub fn name(&self) -> &'static str {
    match self {
      Self::Timeout { .. } => "TimeoutError",
      Self::TargetClosed { .. } => "TargetClosedError",
      _ => "FerriError",
    }
  }

  /// Builder for [`FerriError::Timeout`] with an operation description.
  #[must_use]
  pub fn timeout(operation: impl Into<String>, timeout_ms: u64) -> Self {
    Self::Timeout {
      operation: Some(operation.into()),
      timeout_ms,
    }
  }

  /// Builder for [`FerriError::Timeout`] with no operation description.
  #[must_use]
  pub fn timeout_plain(timeout_ms: u64) -> Self {
    Self::Timeout {
      operation: None,
      timeout_ms,
    }
  }

  /// Builder for [`FerriError::StrictModeViolation`].
  #[must_use]
  pub fn strict(selector: impl Into<String>, count: usize) -> Self {
    Self::StrictModeViolation {
      selector: selector.into(),
      count,
    }
  }

  /// Builder for [`FerriError::TargetClosed`] with optional reason.
  #[must_use]
  pub fn target_closed(reason: Option<String>) -> Self {
    Self::TargetClosed { reason }
  }

  /// Builder for [`FerriError::Protocol`].
  #[must_use]
  pub fn protocol(method: impl Into<String>, message: impl Into<String>) -> Self {
    Self::Protocol {
      method: method.into(),
      message: message.into(),
    }
  }

  /// Builder for [`FerriError::InvalidArgument`].
  #[must_use]
  pub fn invalid_argument(name: impl Into<String>, reason: impl Into<String>) -> Self {
    Self::InvalidArgument {
      name: name.into(),
      reason: reason.into(),
    }
  }

  /// Builder for [`FerriError::InvalidSelector`].
  #[must_use]
  pub fn invalid_selector(selector: impl Into<String>, reason: impl Into<String>) -> Self {
    Self::InvalidSelector {
      selector: selector.into(),
      reason: reason.into(),
    }
  }

  /// Builder for [`FerriError::Evaluation`].
  #[must_use]
  pub fn evaluation(message: impl Into<String>) -> Self {
    Self::Evaluation(message.into())
  }

  /// Builder for [`FerriError::Backend`].
  #[must_use]
  pub fn backend(message: impl Into<String>) -> Self {
    Self::Backend(message.into())
  }

  /// Builder for [`FerriError::Navigation`].
  #[must_use]
  pub fn navigation(url: impl Into<String>, message: impl Into<String>) -> Self {
    Self::Navigation {
      url: url.into(),
      message: message.into(),
    }
  }

  /// Used by locator retry polling — matches historical string matching on
  /// transient DOM/session failures (`error:not*` signals from page scripts).
  #[must_use]
  pub(crate) fn is_locator_poll_retriable(&self) -> bool {
    match self {
      Self::NotConnected => true,
      Self::Backend(msg) | Self::Evaluation(msg) | Self::Other(msg) | Self::Interrupted(msg) => {
        msg.contains("not connected")
          || msg.contains("not found")
          || msg.contains("detached")
          || msg.starts_with("error:not")
      },
      Self::Protocol { message, .. } => {
        message.contains("not found") || message.contains("detached") || message.contains("not connected")
      },
      Self::Timeout { .. }
      | Self::TargetClosed { .. }
      | Self::StrictModeViolation { .. }
      | Self::Navigation { .. }
      | Self::InvalidSelector { .. }
      | Self::InvalidArgument { .. }
      | Self::Unsupported(_)
      | Self::Snapshot(_)
      | Self::Io(_)
      | Self::Json(_) => false,
    }
  }
}

/// Bridges bare strings from legacy call sites into typed errors — prefers
/// [`FerriError::Unsupported`] when the payload carries the `unsupported:` prefix.
impl From<String> for FerriError {
  fn from(s: String) -> Self {
    if let Some(reason) = s.strip_prefix("unsupported:") {
      return Self::Unsupported(reason.trim().to_string());
    }
    Self::Backend(s)
  }
}

impl From<&str> for FerriError {
  fn from(s: &str) -> Self {
    if let Some(reason) = s.strip_prefix("unsupported:") {
      return Self::Unsupported(reason.trim().to_string());
    }
    Self::Backend(s.to_string())
  }
}

impl From<FerriError> for String {
  fn from(e: FerriError) -> Self {
    e.to_string()
  }
}

/// Convenience alias. Every new public API function should return this.
pub type Result<T> = std::result::Result<T, FerriError>;

pub(crate) trait IntoFerriError {
  fn into_ferri_error(self) -> FerriError;
}

impl IntoFerriError for String {
  fn into_ferri_error(self) -> FerriError {
    self.into()
  }
}

impl IntoFerriError for FerriError {
  fn into_ferri_error(self) -> FerriError {
    self
  }
}

/// Normalize action closures used by the locator retry macro (`String` or [`FerriError`]).
#[inline]
pub(crate) fn normalize_action_result<R, E: IntoFerriError>(r: std::result::Result<R, E>) -> Result<R> {
  r.map_err(|e| e.into_ferri_error())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn timeout_message_matches_playwright_shape() {
    let err = FerriError::timeout("navigating to https://example.org", 30_000);
    assert_eq!(
      err.to_string(),
      "Timeout 30000ms exceeded while navigating to https://example.org"
    );
    assert_eq!(err.name(), "TimeoutError");
    assert!(err.is_timeout_error());
    assert!(!err.is_target_closed_error());
  }

  #[test]
  fn timeout_without_operation_omits_while_clause() {
    let err = FerriError::timeout_plain(5_000);
    assert_eq!(err.to_string(), "Timeout 5000ms exceeded");
  }

  #[test]
  fn target_closed_with_reason() {
    let err = FerriError::target_closed(Some("browser crashed".into()));
    assert_eq!(
      err.to_string(),
      "Target page, context or browser has been closed: browser crashed"
    );
    assert_eq!(err.name(), "TargetClosedError");
    assert!(err.is_target_closed_error());
  }

  #[test]
  fn target_closed_without_reason() {
    let err = FerriError::target_closed(None);
    assert_eq!(err.to_string(), "Target page, context or browser has been closed");
  }

  #[test]
  fn strict_mode_violation_reports_selector_and_count() {
    let err = FerriError::strict("button.primary", 3);
    assert_eq!(
      err.to_string(),
      r#"strict mode violation: selector "button.primary" resolved to 3 elements"#
    );
    assert_eq!(err.name(), "FerriError");
    assert!(err.is_strict_mode_violation());
  }

  #[test]
  fn name_dispatch_covers_all_named_variants() {
    assert_eq!(FerriError::timeout_plain(1).name(), "TimeoutError");
    assert_eq!(FerriError::target_closed(None).name(), "TargetClosedError");
    assert_eq!(FerriError::Backend("x".into()).name(), "FerriError");
    assert_eq!(FerriError::NotConnected.name(), "FerriError");
  }

  #[test]
  fn from_string_and_str_bridge_maps_plain_messages_to_backend() {
    let from_string: FerriError = String::from("legacy").into();
    let from_str: FerriError = "legacy".into();
    assert!(matches!(from_string, FerriError::Backend(ref s) if s == "legacy"));
    assert!(matches!(from_str, FerriError::Backend(ref s) if s == "legacy"));
  }

  #[test]
  fn io_and_json_errors_convert_via_question_mark() {
    fn io_fail() -> Result<()> {
      let _: std::fs::File = std::fs::File::open("/definitely/does/not/exist/ferri-test")?;
      Ok(())
    }
    fn json_fail() -> Result<()> {
      let _: serde_json::Value = serde_json::from_str("{")?;
      Ok(())
    }
    assert!(matches!(io_fail().unwrap_err(), FerriError::Io(_)));
    assert!(matches!(json_fail().unwrap_err(), FerriError::Json(_)));
  }

  #[test]
  fn navigation_error_formats_url_and_message() {
    let err = FerriError::Navigation {
      url: "https://example.org".into(),
      message: "net::ERR_NAME_NOT_RESOLVED".into(),
    };
    assert_eq!(
      err.to_string(),
      "navigation to https://example.org failed: net::ERR_NAME_NOT_RESOLVED"
    );
  }

  #[test]
  fn protocol_error_formats_method() {
    let err = FerriError::protocol("Page.navigate", "session detached");
    assert_eq!(err.to_string(), "protocol error (Page.navigate): session detached");
  }

  #[test]
  fn invalid_argument_quotes_name() {
    let err = FerriError::invalid_argument("timeout", "must be non-negative");
    assert_eq!(err.to_string(), r#"invalid argument "timeout": must be non-negative"#);
  }

  #[test]
  fn invalid_selector_quotes_selector() {
    let err = FerriError::invalid_selector("???", "unknown engine");
    assert_eq!(err.to_string(), r#"invalid selector "???": unknown engine"#);
  }
}
