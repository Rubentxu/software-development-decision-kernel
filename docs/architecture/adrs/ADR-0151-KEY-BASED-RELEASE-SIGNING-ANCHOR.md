---
id: ADR-0151
title: Key-based release signing anchor, replacing the keyless OIDC identity
status: accepted
date: 2026-10-02
supersedes: null
superseded_by: null
cycle: p-63676b11dc0ef88f/trust-anchor-key-based-signing
---

# ADR-0151 — Key-based release signing anchor

## Context

SDDK releases are signed with `cosign` and verified by two consumers:
`scripts/install.sh` (fresh installs) and `sddk dev update`
(`crates/sddk-cli/src/dev/update.rs`). Until v2.5.2 the anchor was **keyless**:
a Fulcio certificate minted through GitHub Actions' OIDC provider, pinned as
the pair

- identity regexp `^https://github\.com/Rubentxu/software-development-decision-kernel/\.github/workflows/release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$`
- issuer `https://token.actions.githubusercontent.com`

That identity **only exists inside a GitHub Actions runner**. Running
`scripts/release.sh` on a workstation reaches step 8c and dies:

```
✗ the project's signing identity does not exist on this host.
  Required issuer: https://token.actions.githubusercontent.com
  This host: not a GitHub Actions runner (GITHUB_ACTIONS != true).
```

Observed on the v2.5.3 release attempt (session-65f), which completed steps
0–8b and aborted there. The script's behaviour is correct — signing from a
laptop would mint a certificate for a *person*, which the installers pin
against — but the consequence is that **no release can be produced without a
hosted OIDC runner**.

Two governance facts make that untenable:

1. The project no longer uses GitHub Actions as its CI. PipelineK is the CI
   authority (operator decision, this cycle).
2. Actions cannot produce a keyless SDDK signature, so keeping them solely as
   a signer would mean keeping the one dependency being removed.

## Decision

The anchor becomes **key-based**: the project's public signing key is pinned
as `cosign verify-blob --key`, and signatures are produced by PipelineK
against a private key held in a **KMS** (AWS/GCP/Azure/Vault), so the key is
never exported to any host.

Concretely:

- `assets/trust/release-verify-key.pub` holds the public key
  **as the base64 body of its PEM, on one line**. `cosign::RELEASE_VERIFY_KEY_BODY`
  embeds it via `include_str!`; `install.sh` carries a copy; the drift guard
  in `tests/test_install_asset_contract.sh` fails if they diverge.
- The body is stored flat because the drift guard compares with `sed`, which
  is line-oriented: a multi-line PEM extracts as empty on **both** sides and
  the guard passes without ever comparing a key. Each consumer rebuilds the
  `-----BEGIN/END PUBLIC KEY-----` framing, which cosign v3.1.3 requires —
  verified: a bare one-line PEM fails with `PEM decoding failed`.
- The key lives in a KMS, not a file, because a file is a secret that can be
  copied out of a backup.
- The anchor file lives under the root `assets/` surface, **not** beside the
  crate that embeds it. `assets/` is a `MANIFEST_SURFACES` entry, so the
  published key travels in the bundle; a file under `crates/` is not a
  surface and would have been invisible to `git ls-files`-derived manifests
  — the first attempt put it in `crates/sddk-cli/assets/trust/` and the
  manifest silently stayed at 394 entries. The public key *should* travel:
  it is public material, and the installers need it. It travels as **code**
  (embedded in the binary and in `install.sh`), never as a downloaded
  artifact beside the payload — an attacker controlling the download origin
  could otherwise replace key and artifact together.
- Both consumers try the key-based anchor first and fall back to the legacy
  keyless anchor, which stays pinned for releases v2.2.11…v2.5.2 so
  `install.sh --version v2.5.2` keeps verifying during the transition.

## The property this trades away

**This is a real trade, not a free upgrade.** Keyless has a property
key-based does not: the signing key never exists at rest. Fulcio mints a
short-lived certificate for an identity the OIDC provider vouches for; there
is no long-lived secret to steal, and compromising the runner yields a
credential that expires.

Key-based inverts that. There is a long-lived private key, and whoever can
invoke it can sign artifacts — including, in the worst case, permanently.

| | keyless (was) | key-based (now) |
|---|---|---|
| key material at rest | never | no, **if** the KMS is used correctly |
| forgeable by compromising CI | yes, transiently | yes, persistently |
| requires a hosted OIDC runner | **yes** | no |
| survives losing cloud CI | no | **yes** |

The last row is why the trade was taken; the row above it is the price.
Keeping the key in a KMS rather than a file keeps the "at rest" cell as close
to "never" as key-based can get.

## Consequences

- **v2.5.3 cannot be published until the key exists.** The anchor file
  currently holds the marker `@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@`.
  `install.sh` refuses to verify against it, and
  `cosign::tests::the_anchor_is_provisioned_not_placeholder` is `#[ignore]`d
  (not deleted) so that provisioning the key turns the check loud instead of
  silently green. Publishing against the placeholder would ship a release
  nobody can install — the exact outcome this project's policy forbids.
- The transition window must be closed deliberately. Once no published
  release needs it, `LEGACY_CERT_IDENTITY_REGEXP`, `LEGACY_CERT_ISSUER` and
  both fallback branches go, and the guard's "both anchors" assertions become
  "key-based only".
- `pipelines/certify-sddk-release.pipeline.kts` (PipelineK harness, outside
  this repo) has a `verify-signature` stage that requires `.sig` + `.pem` and
  reads the pins from `install.sh`. A key-based signature has **no `.pem`**,
  so that pipeline must be updated before it can certify a v2.5.3 release.

## Verification

Observed against cosign v3.1.3 on this host:

- key-based sign → `verify-blob --key` with the correct key → `Verified OK`
- same bundle verified with a **different** public key → rejected with
  `transparency log certificate does not match` — a *different* error than a
  malformed key (`PEM decoding failed`), which is what makes the negative
  control discriminating rather than vacuous
- tampered payload with the original signature → rejected
- no `--key` at all → rejected

The drift guard was falsified with four mutations, all detected: anchor
divergence between file and `install.sh`; empty anchor file (the
`empty == empty` case the flat-body design exists to prevent); removal of
`--key` from `install.sh`; and removal of the both-anchors-failed `bail!` from
`verify_bundle_signature`.

That last mutation is why this ADR records the guard's own history: it took
three attempts to make that assertion discriminate. It first anchored on a
status-check shape that no longer existed, then counted any `bail!` in a
1400-line file (17 of them), then counted any `bail!` in the function (three).
All three passed while the bail it was supposed to protect was deleted.
