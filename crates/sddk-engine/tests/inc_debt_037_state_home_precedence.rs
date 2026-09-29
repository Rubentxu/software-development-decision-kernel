//! Contract tests for the `SDDK_STATE_HOME` precedence fix (INC-DEBT-037).
//!
//! Before the fix, `SDDK_STATE_HOME` was read by `admission.rs` only and
//! documented as "used for ledger access", while the ledger was actually
//! resolved from `XDG_STATE_HOME` alone. Setting `SDDK_STATE_HOME` to point
//! an end-to-end verification at a copied ledger therefore wrote to the
//! *real* ledger, silently, with the command reporting success.
//!
//! The contract pinned here: the state root is resolved exactly once, with
//! the precedence `SDDK_STATE_HOME` > `XDG_STATE_HOME` > `HOME`/`.local/state`.
//! Every band is asserted, not just the winner, because the bug was a
//! divergence between two resolvers rather than a wrong order in one.

use std::path::PathBuf;

use sddk_engine::{XdgEnvironment, resolve_xdg_paths};

fn env(
    home: Option<&str>,
    state_home: Option<&str>,
    sddk_state_home: Option<&str>,
) -> XdgEnvironment {
    XdgEnvironment {
        home: home.map(PathBuf::from),
        state_home: state_home.map(PathBuf::from),
        sddk_state_home: sddk_state_home.map(PathBuf::from),
        ..Default::default()
    }
}

fn ledger_of(environment: &XdgEnvironment) -> PathBuf {
    resolve_xdg_paths(environment, "p-test", "w-test")
        .expect("paths must resolve")
        .ledger
}

/// `SDDK_STATE_HOME` must win over `XDG_STATE_HOME`.
///
/// This is the exact scenario that silently hit the real ledger: both were
/// set, the operator set only `SDDK_STATE_HOME`, and the copy was ignored.
#[test]
fn sddk_state_home_takes_precedence_over_xdg_state_home() {
    let ledger = ledger_of(&env(
        Some("/home/op"),
        Some("/tmp/xdg-state"),
        Some("/tmp/sddk-state"),
    ));

    assert!(
        ledger.starts_with("/tmp/sddk-state"),
        "SDDK_STATE_HOME must win, got {ledger:?}"
    );
    assert!(
        !ledger.starts_with("/tmp/xdg-state"),
        "XDG_STATE_HOME must not be used when SDDK_STATE_HOME is set: {ledger:?}"
    );
}

/// With no SDDK override the behaviour must be unchanged: `XDG_STATE_HOME`
/// still wins over `HOME`. Guards against a fix that over-corrects.
#[test]
fn xdg_state_home_still_wins_when_sddk_state_home_absent() {
    let ledger = ledger_of(&env(Some("/home/op"), Some("/tmp/xdg-state"), None));

    assert!(
        ledger.starts_with("/tmp/xdg-state"),
        "XDG_STATE_HOME must still apply, got {ledger:?}"
    );
    assert!(
        !ledger.starts_with("/home/op"),
        "HOME must not win over XDG_STATE_HOME: {ledger:?}"
    );
}

/// With neither override the XDG default under `HOME` applies.
#[test]
fn home_default_applies_when_no_state_override() {
    let ledger = ledger_of(&env(Some("/home/op"), None, None));

    assert_eq!(
        ledger,
        PathBuf::from("/home/op/.local/state/sddk/projects/p-test/ledger.sqlite"),
        "unoverridden state root must fall back to HOME/.local/state"
    );
}

/// The whole point of the variable: the ledger must land under the
/// redirected root, so a copy can be exercised without touching production.
#[test]
fn sddk_state_home_redirects_the_ledger_entirely() {
    let ledger = ledger_of(&env(Some("/home/op"), None, Some("/tmp/isolated")));

    assert_eq!(
        ledger,
        PathBuf::from("/tmp/isolated/sddk/projects/p-test/ledger.sqlite"),
        "the entire ledger path must live under SDDK_STATE_HOME"
    );
}

/// The state override must not leak into the data root.
///
/// `SDDK_DATA_DIR` governs the data root; confusing the two would move
/// artifacts while leaving the ledger behind, which is the opposite of
/// isolation and would be worse than the original bug.
#[test]
fn sddk_state_home_does_not_move_the_data_root() {
    let paths = resolve_xdg_paths(
        &env(Some("/home/op"), None, Some("/tmp/isolated")),
        "p-test",
        "w-test",
    )
    .expect("paths must resolve");

    assert!(
        paths.artifacts.starts_with("/home/op/.local/share"),
        "artifacts must stay under the data root, got {:?}",
        paths.artifacts
    );
    assert!(
        paths.ledger.starts_with("/tmp/isolated"),
        "ledger must move to the state override, got {:?}",
        paths.ledger
    );
}

/// A relative `SDDK_STATE_HOME` must be rejected, not silently accepted.
///
/// Isolation that resolves to a relative path is worse than no isolation:
/// it would be resolved against whatever the CWD happened to be.
#[test]
fn relative_sddk_state_home_is_rejected() {
    let error = resolve_xdg_paths(
        &env(Some("/home/op"), None, Some("relative/path")),
        "p-test",
        "w-test",
    )
    .expect_err("relative SDDK_STATE_HOME must be rejected");

    assert_eq!(
        error,
        sddk_engine::PathResolutionError::NonAbsolute {
            variable: "SDDK_STATE_HOME",
            path: PathBuf::from("relative/path"),
        },
        "must name SDDK_STATE_HOME so the operator knows which knob is wrong"
    );
}
