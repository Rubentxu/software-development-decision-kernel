//! Concrete task implementations for Phase 4 capability routing.
//!
//! This module re-exports the four concrete task types:
//! - [`HttpFetchTask`]  — capability: `http.fetch`
//! - [`FileWriteTask`]  — capability: `file.write`
//! - [`Sha256Task`]     — capability: `sha256.compute`
//! - [`SleepTask`]      — capability: `sleep`
//!
//! Each task wraps the capability name and its input parameters.
//! They are used by `WorkflowRuntime` to build `Task` IR operators.

pub mod file_write;
pub mod http_fetch;
pub mod sha256;
pub mod sleep;

pub use file_write::FileWriteTask;
pub use http_fetch::HttpFetchTask;
pub use sha256::Sha256Task;
pub use sleep::SleepTask;
