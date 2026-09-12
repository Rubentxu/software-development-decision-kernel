---
id: INC-VAULT-MIRROR-AUTO
title: "vault ADR mirrors are only synced via manual operator invocation of mirror_adrs_to_vault.py"
status: open
severity: low
priority: P3
fingerprint: "release-003-vault-mirror-auto-trigger"
fingerprint_aliases:
  - INC-VAULT-MIRROR-AUTO
  - manual-vault-mirror-sync
cluster_id: CL-RELEASE-PIPELINE-INTEGRITY
created: 2026-09-12
created_by: orchestrator
owner: release-pipeline
closed_at: null
closed_by: null
resolution_note: null
last_updated: 2026-09-12
---

# INC-VAULT-MIRROR-AUTO — vault ADR mirrors are only synced via manual operator invocation

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.
> Cluster: CL-RELEASE-PIPELINE-INTEGRITY (sibling of INC-RELEASE-001 and
> INC-RELEASE-TAG-FIX, both of which closed in cycles 2026-09-12).

## Context

`scripts/mirror_adrs_to_vault.py` (293 LoC, Python) generates vault
ADR mirrors for every accepted ADR in `docs/architecture/adrs/`. The
script is **idempotent** (skips existing mirrors) and is currently
invoked **manually** by the orchestrator after a release cycle ships a
new ADR promotion.

### Mechanism

The script reads repo ADRs (`docs/architecture/adrs/ADR-*.md`), parses
their frontmatter (`id`, `status`, `accepted_at`, `accepted_by_cycle`,
`related_adrs`, `package_provenance`, `implementation_evidence`), and
emits vault mirrors at `~/.sddk-knowledge/sddk-framework/adrs/<slug>.md`
using the canonical template (`template/adr.md`) with `status:
accepted`. Per AGENTS §2.7: the vault is human knowledge source, not
runtime authority — the repo ADR remains canonical, the mirror is a
view.

The manual invocation pattern was established at cycle 8
(v1.168.38-vault-mirror-accepted-adrs) when the operator ran the script
once to seed 18 vault mirrors. Subsequent cycles that promote new ADRs
must re-run the script to keep the vault in sync.

### Why this is debt

1. **Operator overhead** — every cycle that promotes ADRs needs a
   post-release manual step. Easy to forget.
2. **Vault drifts from repo** — if the operator forgets, the vault
   mirror set becomes stale and downstream readers (humans) see the
   vault as authoritative when it actually lags.
3. **Re-runnable check is not in the release contract** — there is no
   gate in `scripts/release.sh` that asserts vault mirrors exist for
   every accepted ADR. The `tests/test_vault_adr_mirror_coverage.sh`
   pin test (added at cycle 8) verifies the existing mirrors match,
   but it does not invoke the mirror script itself.

## Rationale

- The release script is the single source of truth for end-to-end
  release publication (AGENTS.md §8). Any post-release operator action
  violates that contract.
- The mirror script is idempotent and best-effort: a failure should
  not block the release (the vault is human knowledge, not runtime
  authority per AGENTS §2.7). The release must surface the failure
  as a warning so the operator can address it later, but the publish
  step must still execute.
- Cloud CI is disabled (AGENTS.md §2.5). The release script is the
  only place where this gate can be enforced.

## Resolution (planned)

Add a new step `8b/14 — vault ADR mirror sync` in `scripts/release.sh`
between step 8 (sha256 + CHECKSUMS + sbom) and step 9 (publish). The
step:

1. Invokes `python3 scripts/mirror_adrs_to_vault.py`.
2. Captures stdout (counts of created/skipped mirrors).
3. On exit 0: logs the counts as `ok` — no further action.
4. On non-zero exit: logs a `warn` (not `die`) and proceeds. The
   release continues because the vault is human knowledge; the runtime
   authority (repo ADR) is already published via the bump commit.

The step is **always executed**, including under `--skip-tests` and
`--dry-run`. It does not touch the binary or the bundle; it only
generates side-effect files in `~/.sddk-knowledge/.../adrs/`.

A new pin test `tests/test_vault_mirror_auto.sh` verifies:
(a) the script is invoked from release.sh, (b) it lives between step 8
and step 9, (c) the failure mode is `warn`, not `die` (vault is
human-side), (d) the script invocation is unconditional (no
`SKIP_TESTS` guard), (e) the working directory is the repo root (the
script uses `Path(__file__).parent.parent` resolution).

Scope of change (estimated):
- `scripts/release.sh`: +15 LoC (new step 8b)
- `tests/test_vault_mirror_auto.sh`: ~70 LoC (new file, 5 invariants)
- `docs/debt/README.md`: 1 row appended
- Archive-manifest of cycle
  `p-63676b11dc0ef88f/vault-mirror-auto`: evidence + closure

## Evidence

- `scripts/mirror_adrs_to_vault.py` exists and is idempotent (verified
  manually across cycles 8 → 11).
- 18/18 vault mirrors exist as of v1.168.41 (verified by
  `tests/test_vault_adr_mirror_coverage.sh`).
- Manual invocation pattern documented in archive-manifests of cycles
  `vault-mirror-accepted-adrs` (cycle 8), `planning-evidence-migration`
  (cycle 9), and `transition-outcome-m9-2-closeout` (cycle 10).
- No release in cycles 8-11 had a vault-mirror step in release.sh.

## Status

Open. Tracked by cycle `p-63676b11dc0ef88f/vault-mirror-auto`
(sequence 511, B-direct path).
