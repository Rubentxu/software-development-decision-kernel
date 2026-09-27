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
//! * [`DEFAULT_CERT_IDENTITY`] — the exact subject. For a keyless workflow
//!   identity this is the workflow *ref*, not a human account.
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

/// Certificate subject that must have signed the artifacts.
///
/// This is a keyless workflow identity of the form
/// `<repo>:<workflow>@<ref>`. Two consequences worth stating plainly:
///
/// * The `ref` is part of the subject, so a signature made from a
///   developer's feature branch is a *different* subject and will not
///   verify. Releases must be signed from a release workflow on the
///   default branch.
/// * Changing the workflow file path or the repository name changes the
///   subject, and the default here must be updated in the same commit that
///   changes them. `tests/test_install_asset_contract.sh` asserts that the
///   shell copies carry the same value, so a partial update fails the
///   suite rather than shipping.
///
/// The owner/repo half MUST match the repository the releases are
/// actually published from. It was briefly wrong here (`1jehuang/sddk-framework`
/// while origin is `Rubentxu/software-development-decision-kernel`), which
/// would have made every future release fail its own installer's pinning.
/// `identity_matches_the_release_repository` pins that against
/// `UpdateArgs::repo`, so the two cannot drift apart again.
pub const DEFAULT_CERT_IDENTITY: &str =
    "Rubentxu/software-development-decision-kernel:.github/workflows/release.yml@refs/heads/main";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_a_github_workflow_ref_not_a_human_account() {
        // A bare `user` or `user@host` value would silently widen or break
        // matching. The keyless form is `owner/repo:workflow@ref`.
        assert!(
            DEFAULT_CERT_IDENTITY.contains('@'),
            "must be a full keyless subject"
        );
        assert!(
            DEFAULT_CERT_IDENTITY.contains(":https://") || DEFAULT_CERT_IDENTITY.contains(".yml@"),
            "identity must carry a workflow file and ref, got: {DEFAULT_CERT_IDENTITY}"
        );
        assert!(
            !DEFAULT_CERT_IDENTITY.contains("keyless:"),
            "trust-root style 'keyless:' prefix does not belong in a certificate identity"
        );
    }

    #[test]
    fn issuer_is_the_github_actions_oidc_issuer() {
        assert_eq!(
            DEFAULT_CERT_ISSUER,
            "https://token.actions.githubusercontent.com"
        );
        assert!(DEFAULT_CERT_ISSUER.starts_with("https://"));
    }

    #[test]
    fn identity_names_the_repository_releases_are_published_from() {
        // The owner/repo prefix of a keyless subject is the repository. If
        // it does not match the repo we actually publish from, every
        // release we ship fails its own installer — a self-inflicted
        // uninstallable release, discovered by users.
        let identity_repo = DEFAULT_CERT_IDENTITY
            .split(':')
            .next()
            .expect("identity must be owner/repo:workflow@ref");
        assert_eq!(
            identity_repo,
            crate::dev::DEFAULT_RELEASE_REPO,
            "certificate identity must name the same repository that UpdateArgs defaults to"
        );
    }

    #[test]
    fn identity_and_issuer_are_independent_values() {
        // The original bug: one variable whose meaning depended on whether
        // the value contained '@'. Guard that the two never collapse into
        // one another, and that the issuer is not a substring trap.
        assert_ne!(DEFAULT_CERT_IDENTITY, DEFAULT_CERT_ISSUER);
        assert!(
            !DEFAULT_CERT_IDENTITY.contains(DEFAULT_CERT_ISSUER),
            "identity must not embed the issuer; that recreates the conflation"
        );
        assert!(!DEFAULT_CERT_IDENTITY.trim().is_empty());
        assert!(!DEFAULT_CERT_ISSUER.trim().is_empty());
    }
}
