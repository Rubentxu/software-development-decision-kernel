//! `sha256.compute` capability task.
//!
//! `Sha256Task` computes the SHA-256 digest of arbitrary bytes or a string.

use serde_json::Value;
use std::collections::BTreeMap;

/// Task operator for the `sha256.compute` capability.
///
/// # Capability
/// `sha256.compute` — computes SHA-256 hash of input data.
///
/// # Inputs
/// - `data` (string or array of bytes, required): Data to hash.
///
/// # Outputs
/// - `hash` (string): 64-character lowercase hex SHA-256 digest.
#[derive(Debug, Clone)]
pub struct Sha256Task {
    /// Data to hash — either a string or an array of bytes.
    pub data: Sha256Data,
}

/// Input data for SHA-256 computation.
#[derive(Debug, Clone)]
pub enum Sha256Data {
    /// UTF-8 text string.
    Text(String),
    /// Array of byte values (0-255).
    Bytes(Vec<u8>),
}

impl Sha256Task {
    /// Returns the capability identifier.
    pub const fn capability() -> &'static str {
        "sha256.compute"
    }

    /// Builds the capability inputs map.
    pub fn inputs(&self) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        match &self.data {
            Sha256Data::Text(s) => {
                m.insert("data".into(), Value::String(s.clone()));
            }
            Sha256Data::Bytes(bytes) => {
                m.insert(
                    "data".into(),
                    Value::Array(bytes.iter().map(|&b| Value::Number(b.into())).collect()),
                );
            }
        }
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

    #[test]
    fn sha256_task_capability_is_sha256_compute() {
        let _task = Sha256Task {
            data: Sha256Data::Text("hello".into()),
        };
        assert_eq!(Sha256Task::capability(), "sha256.compute");
    }

    #[test]
    fn sha256_task_text_inputs() {
        let task = Sha256Task {
            data: Sha256Data::Text("hello".into()),
        };
        let inputs = task.inputs();
        assert_eq!(inputs.get("data").and_then(|v| v.as_str()), Some("hello"));
    }

    #[test]
    fn sha256_task_text_known_hash() {
        let executor = RealTaskExecutor::new();
        let mut inputs = BTreeMap::new();
        inputs.insert("data".into(), Value::String("hello".into()));

        let result =
            <RealTaskExecutor as TaskExecutor>::execute(&executor, "sha256.compute", &inputs);

        assert!(result.is_ok());
        let output = result.unwrap();
        let hash = output.outputs.get("hash").and_then(|v| v.as_str()).unwrap();
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
