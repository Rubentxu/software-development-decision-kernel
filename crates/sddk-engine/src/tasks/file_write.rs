//! `file.write` capability task.
//!
//! `FileWriteTask` writes content to a file path and computes its SHA-256 digest.

use serde_json::Value;
use std::collections::BTreeMap;

/// Task operator for the `file.write` capability.
///
/// # Capability
/// `file.write` — writes UTF-8 text content to a file path.
///
/// # Inputs
/// - `path` (string, required): Destination file path.
/// - `content` (string, required): Content to write.
///
/// # Outputs
/// - `path` (string): The written file path.
/// - `sha256` (string): SHA-256 hex digest of the content.
#[derive(Debug, Clone)]
pub struct FileWriteTask {
    /// Destination file path.
    pub path: String,
    /// Content to write.
    pub content: String,
}

impl FileWriteTask {
    /// Returns the capability identifier.
    pub const fn capability() -> &'static str {
        "file.write"
    }

    /// Builds the capability inputs map.
    pub fn inputs(&self) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("path".into(), Value::String(self.path.clone()));
        m.insert("content".into(), Value::String(self.content.clone()));
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
    use crate::RealTaskExecutor;
    use sddk_domain::TaskExecutor;
    use tempfile::NamedTempFile;

    #[test]
    fn file_write_task_capability_is_file_write() {
        let _task = FileWriteTask {
            path: "/tmp/test.txt".into(),
            content: "hello".into(),
        };
        assert_eq!(FileWriteTask::capability(), "file.write");
    }

    #[test]
    fn file_write_task_inputs_contains_path_and_content() {
        let task = FileWriteTask {
            path: "/tmp/test.txt".into(),
            content: "hello".into(),
        };
        let inputs = task.inputs();
        assert_eq!(
            inputs.get("path").and_then(|v| v.as_str()),
            Some("/tmp/test.txt")
        );
        assert_eq!(
            inputs.get("content").and_then(|v| v.as_str()),
            Some("hello")
        );
    }

    #[test]
    fn file_write_task_writes_and_returns_sha256() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);

        let executor = RealTaskExecutor::new();
        let mut inputs = BTreeMap::new();
        inputs.insert("path".into(), Value::String(path.clone()));
        inputs.insert("content".into(), Value::String("hello world".into()));

        let result = <RealTaskExecutor as TaskExecutor>::execute(&executor, "file.write", &inputs);

        assert!(result.is_ok(), "file.write should succeed: {:?}", result);
        let output = result.unwrap();
        let sha256 = output
            .outputs
            .get("sha256")
            .and_then(|v| v.as_str())
            .unwrap();
        // SHA-256 of "hello world" is known.
        assert_eq!(
            sha256,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
