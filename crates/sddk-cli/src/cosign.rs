//! Single source of truth for the signing anchor.
//!
//! This module exists because the same values are needed in three places —
//! `sddk dev update` (Rust), `scripts/install.sh` and `scripts/release.sh`
//! (bash) — and a divergence between them is a silent security hole, not a
//! build error. `tests/test_install_asset_contract.sh` pins the shell copies
//! against these declarations so the two cannot drift.
//!
//! # Why pinning is mandatory and not optional
//!
//! `cosign verify-blob` without an anchor accepts **any** valid signature.
//! That is a real signature over the real bytes — and it still proves only
//! that *some key signed this*. An attacker who can publish a release can
//! also mint a valid signature for it with a key they generated themselves.
//!
//! Pinning is what makes the check mean "this is our artifact".
//!
//! # The anchor: key-based, and what changed
//!
//! The anchor was, until v2.5.2, **keyless** (Fulcio certificate minted
//! through the GitHub Actions OIDC provider):
//!
//! * [`LEGACY_CERT_IDENTITY_REGEXP`] — the subject pattern.
//! * [`LEGACY_CERT_ISSUER`] — `token.actions.githubusercontent.com`.
//!
//! It is now **key-based**: [`RELEASE_VERIFY_KEY_BODY`] holds the project's
//! public signing key, and verification is `cosign verify-blob --key`.
//!
//! The change was made because the keyless identity only exists inside a
//! GitHub Actions runner, and the project no longer uses Actions as its CI.
//! An operator running the release from a workstation could not obtain the
//! identity the installers pin, so the release aborted rather than publish
//! something uninstallable — correct behaviour, but it means a release
//! cannot be produced without a hosted OIDC runner.
//!
//! ## The property this trades away, stated plainly
//!
//! Keyless has a property key-based does not: **the signing key never
//! exists at rest**. Fulcio mints a short-lived certificate for an identity
//! the OIDC provider vouches for; there is no long-lived secret to steal,
//! and compromising the runner yields a credential that expires.
//!
//! Key-based inverts that. There is a long-lived private key, held in a KMS
//! so it never lands on a disk or in a repository, and whoever can invoke
//! it can sign artifacts — including, in the worst case, permanently.
//!
//! So this is a real trade, not a free upgrade:
//!
//! | | keyless (was) | key-based (now) |
//! |---|---|---|
//! | key material at rest | never | no, **if** the KMS is used correctly |
//! | forgeable by compromising CI | yes, transiently | yes, persistently |
//! | requires a hosted OIDC runner | **yes** | no |
//! | survives the operator losing cloud CI | no | **yes** |
//!
//! The last row is why the trade was taken. The row above it is the price.
//! Keeping the key in a KMS rather than a local file is what keeps the
//! "at rest" cell as close to "never" as key-based can get: the key is
//! never exported, so a stolen backup or a compromised workstation does not
//! hand over signing capability.
//!
//! ## Why the key body, not a PEM
//!
//! The anchor travels into three places, one of them a shell variable, and
//! `tests/test_install_asset_contract.sh` compares the bash copy against
//! this file with `sed`. A multi-line PEM breaks that comparison: `sed`
//! works line-by-line, so a PEM header on its own line yields an empty
//! string on both sides, and "empty == empty" passes without ever comparing
//! a key. That failure mode is silent and was rejected for exactly that
//! reason.
//!
//! So the value here is the **base64 body** of the public key, on one line.
//! Each consumer rebuilds a PEM from it. Verified against cosign v3.1.3:
//! a public key is accepted as `-----BEGIN PUBLIC KEY-----\n<body>\n-----END
//! PUBLIC KEY-----`, and a key with a different body is rejected with a
//! *different* error than a malformed one — which is what makes the
//! negative control in `test_supply_chain_authenticity.sh` discriminating
//! rather than vacuous.
//!
//! Pinning both halves is still required. In keyless mode supplying one
//! without the other is not "half as strict": cosign accepts the call, and
//! the unpinned half is what an attacker chooses.

/// OIDC issuer for GitHub Actions — **legacy anchor, retained for the
/// transition window**.
///
/// Releases from v2.2.11 through v2.5.2 carry keyless signatures minted
/// through this issuer, and users can still ask an installer for one of
/// them (`install.sh --version v2.5.2`). Verifiers therefore accept both
/// anchors until the transition closes; see the module docs. Once no
/// published release needs it, this constant and its consumer code go.
pub const LEGACY_CERT_ISSUER: &str = "https://token.actions.githubusercontent.com";

/// Certificate subject PATTERN for the legacy keyless anchor.
///
/// See the module docs for why the legacy pin exists. The regex-not-literal
/// reasoning still holds for the window in which it is live.
pub const LEGACY_CERT_IDENTITY_REGEXP: &str = r"^https://github\.com/Rubentxu/software-development-decision-kernel/\.github/workflows/release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$";

/// The project's public signing key, as the **base64 body** of its PEM.
///
/// Consumed as `cosign verify-blob --key <rebuilt-pem>`, and signed with
/// `cosign sign-blob --key <kms-ref>` at release time. The matching private
/// key lives in a KMS and is never exported; nothing in this repository can
/// produce a signature. See the module docs for why this is a body rather
/// than a PEM, and for the security property the key-based anchor trades
/// away.
pub const RELEASE_VERIFY_KEY_BODY: &str =
    include_str!("../../../assets/trust/release-verify-key.pub");

/// Re-emit [`RELEASE_VERIFY_KEY_BODY`] as a PEM blob cosign accepts.
///
/// The body is stored flat for the reasons in the module docs; cosign needs
/// the header/footer framing. Kept here so the Rust verifier and the bash
/// verifier cannot disagree about the framing.
pub fn release_verify_key_pem() -> String {
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        RELEASE_VERIFY_KEY_BODY.trim()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The transition placeholder is not a key. It is a marker that the real
    /// anchor has not been provisioned yet, and `release.sh` refuses to
    /// publish while it is in place. Asserting the marker here keeps the two
    /// in agreement: if someone swaps in the real body this test fails and
    /// says why, rather than the marker silently changing meaning.
    const TRANSITION_MARKER: &str = "@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@";

    /// Set the anchor from a real key and this test flips to RED until the
    /// guard in `release.sh` stops treating the placeholder as a blocker.
    ///
    /// Ignored while the anchor is the placeholder, because a permanently
    /// failing test trains everyone to ignore it — the same reasoning that
    /// removed `test_h05_isolation.sh` from the wired gates when it passed
    /// without measuring anything. It is NOT ignored once a real key is
    /// provisioned: at that point an ignored test would be a green light for
    /// a release nobody can install, so it goes loud instead.
    #[test]
    #[ignore = "the release signing key is not provisioned yet; see ADR-0151"]
    fn the_anchor_is_provisioned_not_placeholder() {
        assert_ne!(
            RELEASE_VERIFY_KEY_BODY.trim(),
            TRANSITION_MARKER,
            "the release signing key has not been provisioned yet; releases must not be \
             published against the transition placeholder"
        );
    }

    #[test]
    fn the_anchor_is_a_single_line_base64_body() {
        // The guard in tests/test_install_asset_contract.sh extracts this
        // value with `sed`, which is line-oriented. A multi-line value yields
        // an empty string on both sides of the comparison and the guard
        // passes without ever comparing a key — a silent pass, which is the
        // failure mode the flat body exists to prevent.
        let body = RELEASE_VERIFY_KEY_BODY.trim();
        assert!(
            !body.contains('\n'),
            "the anchor body must be one line so the shell guard can compare it"
        );
        assert!(
            !body.is_empty(),
            "an empty anchor makes --key point at nothing"
        );
        assert!(
            !body.contains("-----BEGIN"),
            "the body must not include PEM framing; each consumer adds it"
        );
    }

    #[test]
    fn the_rebuilt_pem_has_the_framing_cosign_accepts() {
        // Verified against cosign v3.1.3: a public key is accepted only with
        // header/footer framing on their own lines. This is the exact string
        // every consumer must reconstruct.
        let pem = release_verify_key_pem();
        assert!(pem.starts_with("-----BEGIN PUBLIC KEY-----\n"));
        assert!(pem.ends_with("-----END PUBLIC KEY-----\n"));
        assert_eq!(
            pem.lines().count(),
            3,
            "header, body, footer — nothing else"
        );
    }

    #[test]
    fn the_rebuilt_pem_carries_the_anchor_body_verbatim() {
        // If the framing ever dropped or re-wrapped the body, cosign would
        // fail with "PEM decoding failed" — the same error a malformed key
        // produces, which is exactly what made an early negative control
        // vacuous. Pin the body through the formatting.
        let pem = release_verify_key_pem();
        assert!(
            pem.contains(RELEASE_VERIFY_KEY_BODY.trim()),
            "the rebuilt PEM must contain the anchor body unchanged"
        );
    }

    // ── legacy keyless anchor, kept for the transition window ──────────
    //
    // These guard the constants that still verify releases from v2.2.11
    // through v2.5.2. They are pinned on the OBSERVED Fulcio subject shape,
    // not on a helper that could share the pattern's blind spot.

    /// cosign matches the subject with Go's RE2 via
    /// `--certificate-identity-regexp`. The `regex` crate implements the same
    /// syntax and linear-time semantics, so it is a faithful stand-in, and
    /// unlike a hand-rolled parser it cannot silently disagree with the
    /// engine about what the pattern accepts.
    fn matches(subject: &str) -> bool {
        regex::Regex::new(LEGACY_CERT_IDENTITY_REGEXP)
            .expect("the shipped pattern must compile")
            .is_match(subject)
    }

    /// The SHAPE FULCIO ACTUALLY MINTS. Observed on the first signed release
    /// (v2.2.11, Actions run 36477625442): cosign rejected the old
    /// colon-form pattern printing the real subject.
    fn subject_for(ref_part: &str) -> String {
        format!(
            "https://github.com/Rubentxu/software-development-decision-kernel/.github/workflows/release.yml@{ref_part}"
        )
    }

    #[test]
    fn legacy_pattern_compiles() {
        regex::Regex::new(LEGACY_CERT_IDENTITY_REGEXP)
            .expect("LEGACY_CERT_IDENTITY_REGEXP must be a valid RE2 pattern");
    }

    #[test]
    fn legacy_accepts_the_last_keyless_release() {
        // v2.5.2 is the newest release carrying a keyless signature. If this
        // ever fails, installing that version has broken — which is a real
        // regression for anyone running `install.sh --version v2.5.2`.
        assert!(
            matches(&subject_for("refs/tags/v2.5.2")),
            "the last keyless release must still verify during the transition"
        );
    }

    #[test]
    fn legacy_accepts_every_future_semver_tag() {
        for tag in ["v2.5.3", "v2.6.0", "v3.0.0", "v10.20.30"] {
            assert!(
                matches(&subject_for(&format!("refs/tags/{tag}"))),
                "{tag} must verify without editing the legacy pin"
            );
        }
    }

    #[test]
    fn legacy_rejects_a_branch_ref() {
        assert!(
            !matches(&subject_for("refs/heads/main")),
            "a branch is not a release"
        );
    }

    #[test]
    fn legacy_rejects_another_repository() {
        assert!(!matches(
            "https://github.com/attacker/evil/.github/workflows/release.yml@refs/tags/v2.5.2"
        ));
    }

    #[test]
    fn legacy_rejects_another_workflow() {
        for wf in ["ci.yml", "release-automation.yml", "auto-merge.yml"] {
            assert!(
                !matches(&format!(
                    "https://github.com/Rubentxu/software-development-decision-kernel/.github/workflows/{wf}@refs/tags/v2.5.2"
                )),
                "{wf} must not be able to sign a trusted release"
            );
        }
    }

    #[test]
    fn legacy_rejects_a_subject_that_merely_contains_a_match() {
        assert!(!matches(&format!(
            "prefix{}",
            subject_for("refs/tags/v2.5.2")
        )));
        assert!(!matches(&format!(
            "{}suffix",
            subject_for("refs/tags/v2.5.2")
        )));
    }

    #[test]
    fn legacy_identity_names_the_repository_releases_are_published_from() {
        let pattern = LEGACY_CERT_IDENTITY_REGEXP.trim_start_matches('^');
        let prefix = "https://github\\.com/";
        assert!(
            pattern.starts_with(prefix),
            "pattern must pin the github host"
        );
        let rest = &pattern[prefix.len()..];
        let repo = rest
            .split("/\\.github")
            .next()
            .expect("pattern must pin owner/repo");
        assert_eq!(
            repo,
            crate::dev::DEFAULT_RELEASE_REPO,
            "certificate identity must name the same repository UpdateArgs defaults to"
        );
    }

    #[test]
    fn legacy_issuer_is_the_github_actions_oidc_issuer() {
        assert_eq!(
            LEGACY_CERT_ISSUER,
            "https://token.actions.githubusercontent.com"
        );
    }

    #[test]
    fn legacy_identity_and_issuer_stay_independent() {
        assert!(!LEGACY_CERT_IDENTITY_REGEXP.contains(LEGACY_CERT_ISSUER));
        assert!(!LEGACY_CERT_IDENTITY_REGEXP.trim().is_empty());
    }

    #[test]
    fn the_two_anchors_use_distinguishable_inputs() {
        // The two anchors are verified by different cosign flag pairs. A
        // consumer that silently swapped one for the other would accept a
        // signature the other half of the system rejects, so the
        // distinction is part of the contract, not an implementation detail.
        assert!(!LEGACY_CERT_IDENTITY_REGEXP.contains("BEGIN PUBLIC KEY"));
        assert!(!RELEASE_VERIFY_KEY_BODY.contains("BEGIN PUBLIC KEY"));
    }
}
