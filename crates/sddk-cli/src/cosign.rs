//! Single source of truth for the Sigstore certificate pinning.
//!
//! This module exists because the same three values are needed in three
//! places — `sddk dev update` (Rust), `scripts/install.sh` and
//! `scripts/release.sh` (bash) — and a divergence between them is a silent
//! security hole, not a build error. `tests/test_install_asset_contract.sh`
//! pins the shell copies against these declarations so the two cannot drift.
//!
//! # Why pinning is mandatory and not optional
//!
//! `cosign verify-blob` without `--certificate-identity` and
//! `--certificate-oidc-issuer` accepts **any** valid Sigstore certificate.
//! That is a real signature over the real bytes, by a real Fulcio-issued
//! certificate, recorded in the real transparency log — and it still proves
//! only that *some GitHub account signed this*. Since a GitHub account is
//! roughly as easy to obtain as a GitHub Actions runner, an attacker who
//! can publish a release can also mint a valid keyless signature for it.
//!
//! Pinning both halves is what makes the check mean "this is our artifact":
//!
//! * [`DEFAULT_CERT_IDENTITY_REGEXP`] — the subject pattern. For a keyless
//!   workflow identity this is the workflow file and ref, not a human
//!   account. It is a REGEX rather than a literal: see its docs for why a
//!   literal pin breaks on the second release.
//! * [`DEFAULT_CERT_ISSUER`] — the OIDC issuer that vouched for the subject.
//!
//! Supplying one without the other is not "half as strict": cosign will
//! reject the call, and where it does not, the unpinned half is what an
//! attacker chooses. Both are required.

/// OIDC issuer for GitHub Actions.
///
/// This is a fixed URL defined by GitHub; it is not a secret and not a
/// project-specific value. It is pinned so a certificate minted by some
/// other OIDC provider cannot satisfy the check.
pub const DEFAULT_CERT_ISSUER: &str = "https://token.actions.githubusercontent.com";

/// Certificate subject PATTERN that must have signed the artifacts.
///
/// GitHub Actions mints the Fulcio subject as `<repo>:<workflow
/// file>@<GITHUB_REF>`. The ref is the part that moves: releases are
/// built by dispatching `release.yml` on a TAG
/// (`gh workflow run release.yml --ref v2.0.7`, in
/// `.github/workflows/release-automation.yml`), so the ref is
/// `refs/tags/v2.0.7`.
///
/// Three candidate pins, and why only this one survives:
///
/// * `@refs/heads/main` — matches nothing. Every install fails with a
///   signature error that reads like tampering. This was the value
///   shipped in the first cut of the policy.
/// * `@refs/tags/v2.0.7` — matches that release and silently breaks on
///   2.0.8, which is worse: green until the day it is not.
/// * a pattern — correct for every release, no edit required.
///
/// `cosign` exposes `--certificate-identity-regexp` (confirmed against
/// `cosign verify-blob --help` on the pinned version; RE2 syntax), and
/// both consumers use it. RE2 has no backreferences, so this is a real
/// regular-language match rather than a hand-rolled glob.
///
/// What the pattern still pins, and it is the part carrying the weight:
///
///   - the repository, so only this project's workflow may sign;
///   - the workflow FILE, so `ci.yml` or any other workflow holding
///     `id-token: write` cannot produce a trusted signature;
///   - that the ref is a SemVer TAG, not a branch. Pushing a tag is the
///     release act; a feature branch is not.
pub const DEFAULT_CERT_IDENTITY_REGEXP: &str = r"^Rubentxu/software-development-decision-kernel:\.github/workflows/release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$";

#[cfg(test)]
mod tests {
    use super::*;

    /// cosign matches the subject with Go's RE2 via
    /// `--certificate-identity-regexp`. The `regex` crate implements the
    /// same syntax and the same linear-time semantics, so asserting with it
    /// here is a faithful stand-in — and unlike a hand-rolled parser, it
    /// cannot silently disagree with the engine about what the pattern
    /// accepts.
    fn matches(subject: &str) -> bool {
        regex::Regex::new(DEFAULT_CERT_IDENTITY_REGEXP)
            .expect("the shipped pattern must compile")
            .is_match(subject)
    }

    fn subject_for(ref_part: &str) -> String {
        format!(
            "Rubentxu/software-development-decision-kernel:.github/workflows/release.yml@{ref_part}"
        )
    }

    #[test]
    fn the_shipped_pattern_compiles() {
        // Every other test in this module is meaningless if the pattern is
        // not valid RE2, and cosign would reject it at install time with a
        // message that does not obviously mean "your default is malformed".
        regex::Regex::new(DEFAULT_CERT_IDENTITY_REGEXP)
            .expect("DEFAULT_CERT_IDENTITY_REGEXP must be a valid RE2 pattern");
    }

    #[test]
    fn accepts_the_subject_the_release_workflow_actually_mints() {
        // Derived by reading .github/workflows/release-automation.yml: it
        // runs `gh workflow run release.yml --ref v2.0.7`, so GITHUB_REF is
        // refs/tags/v2.0.7 and GitHub mints the subject with that ref.
        assert!(
            matches(&subject_for("refs/tags/v2.0.7")),
            "the subject the release workflow actually produces must verify"
        );
    }

    #[test]
    fn accepts_every_future_semver_tag() {
        // The entire reason this is a pattern: v2.0.8 must verify without a
        // source edit. A literal pin was green on one release and broken on
        // the next, which is the worst failure mode available.
        for tag in ["v2.0.8", "v2.1.0", "v3.0.0", "v10.20.30"] {
            assert!(
                matches(&subject_for(&format!("refs/tags/{tag}"))),
                "{tag} must verify without editing the pin"
            );
        }
    }

    #[test]
    fn rejects_a_branch_ref() {
        // This is the exact value shipped in the first cut. It matches
        // nothing, so every install failed with a signature error that
        // reads like tampering.
        assert!(
            !matches(&subject_for("refs/heads/main")),
            "a branch is not a release; the first shipped pin matched nothing"
        );
    }

    #[test]
    fn rejects_another_repository() {
        // The threat model: an attacker obtains a genuine Fulcio
        // certificate from the same issuer. Only the repo pin stops it.
        assert!(
            !matches("attacker/evil:.github/workflows/release.yml@refs/tags/v2.0.7"),
            "another repository must not produce a trusted signature"
        );
    }

    #[test]
    fn rejects_another_workflow() {
        // Any workflow holding id-token: write can mint a valid certificate.
        // Pinning the workflow file is what keeps ci.yml from signing a
        // release, and a developer's branch from being a release.
        for wf in ["ci.yml", "release-automation.yml", "auto-merge.yml"] {
            assert!(
                !matches(&format!(
                    "Rubentxu/software-development-decision-kernel:.github/workflows/{wf}@refs/tags/v2.0.7"
                )),
                "{wf} must not be able to sign a trusted release"
            );
        }
    }

    #[test]
    fn rejects_a_ref_that_is_not_a_plain_semver_tag() {
        // A pre-release tag is signed by the same workflow, but this policy
        // only trusts published versions. If prereleases must be trusted,
        // that is a decision to make explicitly, not an accident of a
        // pattern being loose.
        assert!(!matches(&subject_for("refs/tags/v2.0.7-rc1")));
        assert!(!matches(&subject_for("refs/tags/2.0.7")));
    }

    #[test]
    fn rejects_a_subject_that_merely_contains_a_match() {
        // The pattern is anchored. Without ^ and $ a subject like
        // `evil-repo/...:ci.yml@refs/heads/x` with our string appended would
        // pass, which is the same hole in a different position.
        assert!(!matches(&format!(
            "prefix{}",
            subject_for("refs/tags/v2.0.7")
        )));
        assert!(!matches(&format!(
            "{}suffix",
            subject_for("refs/tags/v2.0.7")
        )));
    }

    #[test]
    fn identity_names_the_repository_releases_are_published_from() {
        let pattern_repo = DEFAULT_CERT_IDENTITY_REGEXP
            .trim_start_matches('^')
            .split(":")
            .next()
            .expect("pattern must be ^owner/repo:workflow@ref");
        assert_eq!(
            pattern_repo,
            crate::dev::DEFAULT_RELEASE_REPO,
            "certificate identity must name the same repository UpdateArgs defaults to"
        );
    }

    #[test]
    fn issuer_is_the_github_actions_oidc_issuer() {
        assert_eq!(
            DEFAULT_CERT_ISSUER,
            "https://token.actions.githubusercontent.com"
        );
    }

    #[test]
    fn identity_and_issuer_stay_independent() {
        assert!(!DEFAULT_CERT_IDENTITY_REGEXP.contains(DEFAULT_CERT_ISSUER));
        assert!(!DEFAULT_CERT_IDENTITY_REGEXP.trim().is_empty());
    }
}
