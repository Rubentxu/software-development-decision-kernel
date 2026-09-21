use sddk_gateway::redact;
use serde_json::json;

// Adversarial probe of the secret-key redactor. The redactor matches
// keys in SECRET_KEY_PATTERN (9 names) either as the whole key or as
// the suffix after an underscore. Each case below was picked to
// characterise one specific rule edge.

// 1. Caps token (TOKEN=) - lowercase normalize should match
#[test]
fn adv_01_token_caps_eq() {
    let input = json!({"stdout": "TOKEN=PRIVATE_VALUE_ABC"});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_01 TOKEN=: {s}");
    assert!(!s.contains("PRIVATE_VALUE_ABC"));
}

// 2. Colon shape (token:) - the regex covers `:` and `=` separators
#[test]
fn adv_02_token_colon() {
    let input = json!({"stdout": "token:PRIVATE_VALUE_DEF"});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_02 token: {s}");
    assert!(!s.contains("PRIVATE_VALUE_DEF"));
}

// 3. Suffix match: my_token= matches because ends_with("_token")
#[test]
fn adv_03_suffix_my_token_eq() {
    let input = json!({"stdout": "my_token=PRIVATE_VALUE_GHI"});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_03 my_token=: {s}");
    assert!(!s.contains("PRIVATE_VALUE_GHI"));
}

// 4. Adversarial: "stoken" - DOES NOT match (no underscore before "token")
//    Pinned: this is the current contract, redactor intentionally does
//    NOT match substring prefixes.
#[test]
fn adv_04_stoken_no_underscore_passes_through() {
    let input = json!({"stdout": "stoken=PRIVATE_VALUE_JKL"});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_04 stoken=: {s}");
    // Pin the documented contract: stoken passes through.
    assert!(s.contains("PRIVATE_VALUE_JKL"));
}

// 5. Multi-line input: api_key= on second line is masked
#[test]
fn adv_05_multiline_api_key_eq() {
    let input = json!({"stdout": "line1\napi_key=PRIVATE_VALUE_MNO\nline3"});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_05 multiline: {s}");
    assert!(!s.contains("PRIVATE_VALUE_MNO"));
}

// 6. Empty value: secret= should still match the key shape
//    Pinned contract: redactor only emits <redacted:N> when N > 0
//    (an empty value carries no credential, so nothing to mask).
//    The key+separator stay verbatim.
#[test]
fn adv_06_empty_value_secret_eq() {
    let input = json!({"stdout": "secret="});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_06 empty: {s}");
    // Empty value: key shape detected but no redacted tag emitted
    // (nothing to mask, value_len == 0 short-circuits the formatter).
    assert!(
        s.contains("secret="),
        "key+sep must stay verbatim, got: {s}"
    );
    assert!(
        !s.contains("<redacted:"),
        "no redacted tag for empty value, got: {s}"
    );
}

// 7. Two secrets on the same line: api_key=AAA password=BBB
#[test]
fn adv_07_two_secrets_same_line() {
    let input = json!({"stdout": "api_key=AAA password=BBB"});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_07 two: {s}");
    assert!(!s.contains("AAA"));
    assert!(!s.contains("BBB"));
}

// 8. Uppercase: TOKEN= - lowercase normalize matches
#[test]
fn adv_08_uppercase_token_eq() {
    let input = json!({"stdout": "TOKEN=PRIVATE_VALUE_PQR"});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_08 TOKEN caps: {s}");
    assert!(!s.contains("PRIVATE_VALUE_PQR"));
}

// 9. False positive guard: 'auth' is not in SECRET_KEY_PATTERN
//    Pinned: must pass through verbatim.
#[test]
fn adv_09_partial_key_auth_passes_through() {
    let input = json!({"stdout": "auth=some_value"});
    let out = redact(input);
    let s = out["stdout"].as_str().unwrap();
    println!("adv_09 auth=: {s}");
    // 'auth' is NOT a secret_key — should pass through
    assert!(s.contains("some_value"));
}
