//! `http.fetch` capability task.
//!
//! `HttpFetchTask` executes an HTTP GET request via `reqwest 0.12` (rustls-tls)
//! through `RealTaskExecutor`'s cached client, dispatched via the internal tokio
//! runtime.

use serde_json::Value;
use std::collections::BTreeMap;

/// Task operator for the `http.fetch` capability.
///
/// # Capability
/// `http.fetch` — performs a synchronous HTTP GET request.
///
/// # Inputs
/// - `url` (string, required): The URL to fetch.
///
/// # Outputs
/// - `body` (string): Response body as UTF-8 string.
/// - `status_code` (number): HTTP status code.
#[derive(Debug, Clone)]
pub struct HttpFetchTask {
    /// URL to fetch via HTTP GET.
    pub url: String,
}

impl HttpFetchTask {
    /// Returns the capability identifier.
    pub const fn capability() -> &'static str {
        "http.fetch"
    }

    /// Builds the capability inputs map.
    pub fn inputs(&self) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("url".into(), Value::String(self.url.clone()));
        m
    }

    /// Returns `(capability, inputs)` tuple for use with `TaskExecutor::execute`.
    pub fn capability_and_inputs(&self) -> (&'static str, BTreeMap<String, Value>) {
        (Self::capability(), self.inputs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_fetch_task_capability_is_http_fetch() {
        let _task = HttpFetchTask {
            url: "http://example.com".into(),
        };
        assert_eq!(HttpFetchTask::capability(), "http.fetch");
    }

    #[test]
    fn http_fetch_task_inputs_contains_url() {
        let task = HttpFetchTask {
            url: "http://example.com".into(),
        };
        let inputs = task.inputs();
        assert_eq!(
            inputs.get("url").and_then(|v| v.as_str()),
            Some("http://example.com")
        );
    }

    #[test]
    fn http_fetch_task_capability_and_inputs() {
        let task = HttpFetchTask {
            url: "http://test.com".into(),
        };
        let (cap, inputs) = task.capability_and_inputs();
        assert_eq!(cap, "http.fetch");
        assert_eq!(
            inputs.get("url").and_then(|v| v.as_str()),
            Some("http://test.com")
        );
    }
}
