use sddk_gateway::redact;
use serde_json::json;

#[test]
fn redact_masks_token_value_in_stdout() {
    let input = json!({
        "stdout": "stage=build token=abc123def456 duration=4.2s"
    });
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    assert!(!s.contains("abc123def456"), "value must be masked: {s}");
    assert!(s.contains("<redacted:"), "redaction tag must remain: {s}");
    assert!(
        s.contains("stage=build"),
        "diagnostic prefix preserved: {s}"
    );
}

#[test]
fn redact_does_not_touch_non_secret_keys() {
    let input = json!({
        "stdout": "all good; nothing secret here; exit_status=0",
        "stderr": "another clean line"
    });
    let out = redact(input);
    assert_eq!(
        out["stdout"].as_str().unwrap(),
        "all good; nothing secret here; exit_status=0"
    );
    assert_eq!(out["stderr"].as_str().unwrap(), "another clean line");
}

#[test]
fn redact_keeps_existing_key_level_redaction() {
    let input = json!({
        "credentials": {"password": "hunter2"},
        "stdout": "ok password=hunter2 leaked"
    });
    let out = redact(input);
    // credentials.password is masked at key level
    assert_eq!(out["credentials"]["password"], "<redacted>");
    // stdout: the password=hunter2 substring inside stdout is masked
    assert!(!out["stdout"].as_str().unwrap().contains("hunter2"));
}
