//! Integration tests for the four concrete task types.
//!
//! Tests cover: happy path, error path, and invariant checks for each task.

use sddk_domain::TaskExecutor;
use sddk_engine::RealTaskExecutor;
use sddk_engine::tasks::FileWriteTask;
use sddk_engine::tasks::HttpFetchTask;
use sddk_engine::tasks::SleepTask;
use sddk_engine::tasks::sha256::Sha256Data;
use sddk_engine::tasks::sha256::Sha256Task;

// ── HttpFetch tests ──────────────────────────────────────────────────────────

#[test]
fn http_fetch_happy_path_no_such_host() {
    // No network in test environment — use invalid host to test dispatch path.
    let executor = RealTaskExecutor::new();
    let task = HttpFetchTask {
        url: "http://this-domain-does-not-exist-12345.invalid/".to_string(),
    };
    let (cap, inputs) = task.capability_and_inputs();
    assert_eq!(cap, "http.fetch");
    let result = executor.execute(cap, &inputs);
    // Should be an error (DNS failure or connection refused), not a panic.
    assert!(
        result.is_err(),
        "invalid host should error, got: {:?}",
        result
    );
}

#[test]
fn http_fetch_error_path_missing_url() {
    let executor = RealTaskExecutor::new();
    let task = HttpFetchTask {
        url: "http://example.com".to_string(),
    };
    let mut inputs = task.inputs();
    inputs.remove("url"); // Remove required field.
    let result = executor.execute("http.fetch", &inputs);
    assert!(result.is_err(), "missing url should error");
    let err_msg = result.unwrap_err().message;
    assert!(
        err_msg.contains("url"),
        "should mention missing url: {err_msg}"
    );
}

#[test]
fn http_fetch_invariant_capability_name() {
    let task = HttpFetchTask {
        url: "http://example.com".to_string(),
    };
    assert_eq!(HttpFetchTask::capability(), "http.fetch");
    let (cap, _) = task.capability_and_inputs();
    assert_eq!(cap, "http.fetch");
}

// ── FileWrite tests ─────────────────────────────────────────────────────────

#[test]
fn file_write_happy_path_writes_and_returns_sha256() {
    let executor = RealTaskExecutor::new();
    let task = FileWriteTask {
        path: "/tmp/sddk_test_file_write.txt".to_string(),
        content: "test content".to_string(),
    };
    let (cap, inputs) = task.capability_and_inputs();
    assert_eq!(cap, "file.write");
    let result = executor.execute(cap, &inputs);
    assert!(result.is_ok(), "file.write should succeed: {:?}", result);
    let output = result.unwrap();
    let sha256 = output.outputs.get("sha256").and_then(|v| v.as_str());
    assert!(sha256.is_some(), "should return sha256 output");
    assert_eq!(sha256.unwrap().len(), 64, "sha256 is 64 hex chars");
}

#[test]
fn file_write_error_path_missing_path() {
    let executor = RealTaskExecutor::new();
    let task = FileWriteTask {
        path: "/tmp/test.txt".to_string(),
        content: "content".to_string(),
    };
    let mut inputs = task.inputs();
    inputs.remove("path");
    let result = executor.execute("file.write", &inputs);
    assert!(result.is_err(), "missing path should error");
}

#[test]
fn file_write_invariant_sha256_is_deterministic() {
    let task1 = FileWriteTask {
        path: "/a.txt".to_string(),
        content: "deterministic".to_string(),
    };
    let task2 = FileWriteTask {
        path: "/b.txt".to_string(),
        content: "deterministic".to_string(),
    };
    let (cap1, inputs1) = task1.capability_and_inputs();
    let (cap2, inputs2) = task2.capability_and_inputs();
    assert_eq!(cap1, "file.write");
    assert_eq!(cap2, "file.write");
    assert_eq!(inputs1.get("content"), inputs2.get("content"));
}

// ── Sha256 tests ─────────────────────────────────────────────────────────────

#[test]
fn sha256_happy_path_text_hash() {
    let executor = RealTaskExecutor::new();
    let task = Sha256Task {
        data: Sha256Data::Text("hello".to_string()),
    };
    let (cap, inputs) = task.capability_and_inputs();
    assert_eq!(cap, "sha256.compute");
    let result = executor.execute(cap, &inputs);
    assert!(
        result.is_ok(),
        "sha256.compute should succeed: {:?}",
        result
    );
    let output = result.unwrap();
    let hash = output.outputs.get("hash").and_then(|v| v.as_str()).unwrap();
    assert_eq!(hash.len(), 64);
    // Known SHA-256 of "hello"
    assert_eq!(
        hash,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn sha256_error_path_missing_data() {
    let executor = RealTaskExecutor::new();
    let task = Sha256Task {
        data: Sha256Data::Text("hello".to_string()),
    };
    let mut inputs = task.inputs();
    inputs.remove("data");
    let result = executor.execute("sha256.compute", &inputs);
    assert!(result.is_err(), "missing data should error");
}

#[test]
fn sha256_invariant_hex_is_lowercase() {
    let executor = RealTaskExecutor::new();
    let task = Sha256Task {
        data: Sha256Data::Text("test".to_string()),
    };
    let (_, inputs) = task.capability_and_inputs();
    let result = executor.execute("sha256.compute", &inputs).unwrap();
    let hash = result.outputs.get("hash").and_then(|v| v.as_str()).unwrap();
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "hash should be lowercase hex: {hash}"
    );
}

// ── Sleep tests ─────────────────────────────────────────────────────────────

#[test]
fn sleep_happy_path_elapses() {
    let executor = RealTaskExecutor::new();
    let task = SleepTask { ms: 20 };
    let (cap, inputs) = task.capability_and_inputs();
    assert_eq!(cap, "sleep");
    let result = executor.execute(cap, &inputs);
    assert!(result.is_ok(), "sleep should succeed: {:?}", result);
    let elapsed = result
        .unwrap()
        .outputs
        .get("elapsed_ms")
        .and_then(|v| v.as_u64())
        .unwrap();
    assert!(
        elapsed >= 15,
        "sleep(20ms) should elapse at least 15ms, got {elapsed}ms"
    );
}

#[test]
fn sleep_error_path_missing_ms() {
    let executor = RealTaskExecutor::new();
    let task = SleepTask { ms: 10 };
    let mut inputs = task.inputs();
    inputs.remove("ms");
    let result = executor.execute("sleep", &inputs);
    assert!(result.is_err(), "missing ms should error");
}

#[test]
fn sleep_invariant_elapsed_at_least_requested() {
    let executor = RealTaskExecutor::new();
    let task = SleepTask { ms: 15 };
    let (_, inputs) = task.capability_and_inputs();
    let result = executor.execute("sleep", &inputs).unwrap();
    let elapsed = result
        .outputs
        .get("elapsed_ms")
        .and_then(|v| v.as_u64())
        .unwrap();
    let requested = inputs.get("ms").and_then(|v| v.as_u64()).unwrap();
    assert!(
        elapsed >= requested - 5, // Allow 5ms tolerance for scheduling variance
        "elapsed({elapsed}ms) should be >= requested({requested}ms) - 5ms tolerance"
    );
}
