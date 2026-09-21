//! AIW-S8 X02 — integration tests for the host denial surface.
//!
//! Mirrors the unit tests in `crates/sddk-gateway/src/denial_surface.rs`
//! plus a full-path test from CLI-shaped input through `evaluate()`.

use sddk_gateway::denial_surface::{DenialPolicy, DenialReason, DenialVerdict, HostActionRequest};

#[test]
fn integration_empty_action_id_denied() {
    let req = HostActionRequest {
        action_id: String::new(),
        args: vec!["--verbose".into()],
        payload: None,
    };
    assert_eq!(
        req.evaluate(&DenialPolicy::default()),
        DenialVerdict::Denied(DenialReason::EmptyActionId)
    );
}

#[test]
fn integration_secret_prefix_denied() {
    let req = HostActionRequest {
        action_id: "git.push".into(),
        args: vec!["PASSWORD_hunter2".into()],
        payload: None,
    };
    match req.evaluate(&DenialPolicy::default()) {
        DenialVerdict::Denied(DenialReason::SecretPrefixedArg { prefix }) => {
            assert_eq!(prefix, "PASSWORD_");
        }
        other => panic!("expected secret-prefix denial, got {other:?}"),
    }
}

#[test]
fn integration_oversized_payload_denied() {
    let req = HostActionRequest {
        action_id: "artifact.write".into(),
        args: vec![],
        payload: Some(vec![0xAA; 128 * 1024]),
    };
    match req.evaluate(&DenialPolicy::default()) {
        DenialVerdict::Denied(DenialReason::OversizedPayload {
            size_bytes,
            max_bytes,
        }) => {
            assert_eq!(size_bytes, 128 * 1024);
            assert_eq!(max_bytes, 64 * 1024);
        }
        other => panic!("expected oversized denial, got {other:?}"),
    }
}

#[test]
fn integration_raw_bytes_arg_denied() {
    let req = HostActionRequest {
        action_id: "exec".into(),
        args: vec!["arg\u{0}with-nul".into()],
        payload: None,
    };
    assert_eq!(
        req.evaluate(&DenialPolicy::default()),
        DenialVerdict::Denied(DenialReason::RawBytesArg)
    );
}

#[test]
fn integration_clean_request_allowed() {
    let req = HostActionRequest {
        action_id: "cycle.status".into(),
        args: vec!["--cycle".into(), "c-123".into()],
        payload: Some(b"{\"status\":\"open\"}".to_vec()),
    };
    assert_eq!(
        req.evaluate(&DenialPolicy::default()),
        DenialVerdict::Allowed
    );
}

#[test]
fn integration_with_host_action_request() {
    // Full path: a CLI invocation (argv + optional stdin payload) is mapped
    // to a HostActionRequest and evaluated. Each malformed shape is denied
    // with a reason that carries no raw content.
    let cli_invocations: Vec<(Vec<String>, Option<Vec<u8>>)> = vec![
        (
            ["sddk", "exec", "--token", "SECRET_K=leaky-value"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            None,
        ),
        (
            ["sddk", "exec", "raw\u{0}bytes"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            None,
        ),
        (
            ["sddk", "ingest"].iter().map(|s| s.to_string()).collect(),
            Some(vec![0u8; 64 * 1024 + 10]),
        ),
    ];
    let policy = DenialPolicy::default();
    for (argv, payload) in cli_invocations {
        let req = HostActionRequest {
            action_id: argv.get(1).cloned().unwrap_or_default(),
            args: argv.iter().skip(2).cloned().collect(),
            payload,
        };
        let verdict = req.evaluate(&policy);
        assert!(
            matches!(verdict, DenialVerdict::Denied(_)),
            "expected denial for {argv:?}, got {verdict:?}"
        );
        let rendered = format!("{verdict:?}");
        assert!(
            !rendered.contains("leaky-value"),
            "verdict rendered raw secret content: {rendered}"
        );
        assert!(
            !rendered.contains('\u{0}'),
            "verdict rendered raw byte content: {rendered}"
        );
    }

    // Clean invocation passes.
    let clean = HostActionRequest {
        action_id: "exec".into(),
        args: vec!["--flag".into()],
        payload: None,
    };
    assert_eq!(clean.evaluate(&policy), DenialVerdict::Allowed);
}

#[test]
fn integration_denial_zero_leak_no_payload_in_reason() {
    let req = HostActionRequest {
        action_id: "ok".into(),
        args: vec!["SECRET_TOKEN=hello".into()],
        payload: None,
    };
    let verdict = req.evaluate(&DenialPolicy::default());
    match verdict {
        DenialVerdict::Denied(DenialReason::SecretPrefixedArg { prefix }) => {
            assert!(
                ["SECRET_", "TOKEN_", "PASSWORD_", "API_KEY_"].contains(&prefix.as_str()),
                "prefix leaked raw arg content: {prefix:?}"
            );
        }
        other => panic!("expected secret-prefix denial, got {other:?}"),
    }
}
