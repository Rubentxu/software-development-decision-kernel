# A5 — BASE_PRODUCTION_READY Contract

> Cycle: `p-63676b11dc0ef88f-a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (planning only — nothing here is implemented)**
> Certified baseline: `A4_CERTIFIED` (v1.169.68)

## Purpose

A4 certified that SDDK's **semantics are correct**. A5 must certify that
those semantics can **operate reliably in production**. This document
defines, falsifiably, what `BASE_PRODUCTION_READY` means, so the milestone
can be declared without subjectivity.

## The definition

```text
BASE_PRODUCTION_READY
  iff
  every mandatory production-readiness gate below
    has durable evidence (a receipt a third party can re-run), and
  zero undisposed blocker.

No score. No maturity percentage. No weighted quality number.
```

A gate is *mandatory* unless this contract explicitly marks it optional. A
mandatory gate with no evidence is a blocker, not a partial credit.

## §2 A4 semantic freeze

A5 MUST NOT change, for convenience, any of:

- `EvidencePosture` semantics (Supported / Contradicted / Conflicted /
  Insufficient)
- Verify states (`Verified | Contradicted | Unknown | Stale |
  NotApplicable`)
- DebVerify summaries (6 closed variants)
- Alignment states (7 closed states)
- `UniversalConcern` (10 concerns) and applicability semantics
- `AlignmentLens` epistemology (requested concern == contribution concern)
- Intelligence Loop composition semantics (receives, never calls; one
  content-addressed receipt)
- Advisory / WHY semantics (`absence != negation`; WHY explains, never
  strengthens)
- `MISALIGNED != DENY`
- `SpecifiedBy != VerifiedBy`
- namespace identity rules (`Unit("x")` ≠ `Component("x")` ≠ `Entity("x")`)

If a hardening cycle discovers a semantic defect: **STOP** → corrective
semantic slice → recertification impact analysis. It must not be hidden
inside "hardening". A5 may **falsify** A4 in runtime; it may not
**reinterpret** it.

## Mandatory gates

Each gate is falsifiable: it names the failure mode it detects and the
evidence it requires.

### G1 — Semantic correctness (A4 preserved)

- **Falsifies:** an A5 change silently breaking a certified A4 contract.
- **Evidence:** the A4 milestone corpus + the A5 regression gate
  (`A4-MILESTONE-RECEIPT` §5 invariants) run green on every A5 cycle;
  `A4_CERTIFIED` preserved.
- **Gate:** no A5 release ships with the A4 regression gate red.

### G2 — Durability / crash recovery

- **Falsifies:** data loss after a crash between fact append and
  projection rebuild; partial/corrupt/missing CAS objects; stale refs.
- **Evidence:** the durability falsification matrix (see
  `A5-FALSIFICATION-MATRIX.md`) executed with durable receipts.
- **Gate:** canonical facts survive; projections rebuild; **no projection
  becomes authority**.

### G3 — Rebuildability

- **Falsifies:** a projection that cannot be reconstructed from canonical
  inputs, or that drifts across rebuilds.
- **Evidence:** repeated-rebuild equivalence; delete-projection + rebuild.
- **Gate:** same canonical inputs → byte-identical projection.

### G4 — Concurrency correctness

- **Falsifies:** two writers silently overwriting (`last writer wins`),
  stale writers, authority-vs-effect races.
- **Evidence:** concurrency falsification matrix; **conflict → explicit
  typed outcome**, never silent overwrite. ADR-0097 remains the common
  revision substrate authority.
- **Gate:** every conflict path returns a typed outcome and is tested.

### G5 — Authority fail-closed

- **Falsifies:** a denied action producing a side effect; `RequireApproval`
  bypass; retry double-applying; stale approval authorizing a changed
  proposal; a TOCTOU window between decision and effect.
- **Evidence:** authority falsification matrix over the real
  `AuthorityEngine`.
- **Gate:** denied ⇒ zero side effect; the decision→effect atomicity
  boundary is named and tested (or declared a blocker).

### G6 — Migration / compatibility

- **Falsifies:** an upgrade breaking persisted state or a supported
  client surface without a migration path.
- **Evidence:** migration tests + explicit compatibility inventory.
- **Gate:** every retained compatibility surface has a documented reason
  and a removal trigger, or is deleted.

### G7 — Release reproducibility

- **Falsifies:** a release whose tag does not resolve to the certified
  SHA, or whose assets are not reproducible/verifiable.
- **Evidence:** `scripts/release.sh` + `PublicReleaseGate` receipts.
- **Gate:** tag SHA == certified SHA == binary provenance SHA (where the
  build format allows); workspace/binary/bundle versions coherent.

### G8 — Distribution integrity

- **Falsifies:** a corrupt, partial, or stale public asset; a CDN serving
  a previous binary.
- **Evidence:** 9-asset contract, CHECKSUMS, SBOM, public-URL 200s,
  install-from-public-release, doctor, prune, distribution round-trip,
  rollback, corrupt-asset detection.
- **Gate:** the clean-machine UAT (G12) passes from a fresh environment.

### G9 — Installed-binary behaviour

- **Falsifies:** a binary that only works inside the repo checkout or via
  `cargo run`.
- **Evidence:** the clean-machine acceptance UAT runs the **published
  binary**, not `cargo run`.
- **Gate:** no implicit dependency on the repo checkout.

### G10 — Operator diagnostics

- **Falsifies:** an operator who cannot answer "what is running / what
  failed / why / which revision/run/receipt/policy/evidence / can it be
  reproduced / rebuilt".
- **Evidence:** `doctor` probes for storage, CAS, bundle, binary, config,
  projection rebuild, release coherence, optional provider availability,
  reusing existing telemetry/WHY/receipts (no second observability
  system).
- **Gate:** each question has a concrete, testable command.

### G11 — Security / secrets

- **Falsifies:** a secret appearing in a log, receipt, error, telemetry,
  CAS payload, handoff, or becoming semantic identity.
- **Evidence:** the security secrets matrix over production surfaces.
- **Gate:** no secret-leak path proven reachable.

### G12 — Clean-machine acceptance

- **Falsifies:** certification that only holds on the developer's working
  copy.
- **Evidence:** fresh isolated environment → download public release →
  verify checksum → install → doctor → exercise the canonical workflow →
  restart → recover/rebuild → upgrade → rollback.
- **Gate:** the whole sequence passes with no repo-checkout dependency.

### G13 — Test reliability

- **Falsifies:** hidden flakiness; `couldn't reproduce → closed`.
- **Evidence:** full inventory of ignored/flaky/timing/network/release-only
  tests; the stale-detection flake resolved as
  `REPRODUCED_AND_FIXED` \| `PROVEN_INFRA_FLAKE_WITH_MITIGATION` \|
  `OBSOLETE_TEST_REMOVED`.
- **Gate:** every ignored test individually justified; zero unreproduced
  flakes.

### G14 — Deprecation closure

- **Falsifies:** dead compatibility code kept "just in case".
- **Evidence:** `paradigm_lens::evaluate_lens()` deleted, or an external
  consumer named with a removal trigger; deprecated-pattern lints each
  disposed (`PROMOTE_DENY` \| `KEEP_ALLOW_WITH_REASON` \| `REMOVE_OBSOLETE`).
- **Gate:** no zombie authority; no unowned dead surface.

### G15 — Rollback / recovery

- **Falsifies:** an upgrade that cannot be rolled back to the certified
  previous version.
- **Evidence:** previous-version reinstall + doctor coherence.
- **Gate:** rollback to the prior certified release works and is tested.

### G16 — Exact-revision certification

- **Falsifies:** certifying a floating `origin/main`.
- **Evidence:** the immutable revision identity set (see
  `A5-RELEASE-CERTIFICATION-PROTOCOL.md`):
  `released_baseline`, `development_head`, `workspace_version`,
  `actual_release_tag`, `release_sha`, `binary_sha256`, `bundle_digest`,
  `sbom_digest`, `test_receipt_digest`.
- **Gate:** certified SHA == tag SHA == binary provenance SHA.

## Non-goals (explicitly not required for BASE_PRODUCTION_READY)

Multi-region deployment, horizontal scaling, SLA numbers, capacity
planning, and any new product capability. Those are POST_BASE.

## Severity rules (§20)

Defined **before** finding bugs, so enthusiasm cannot reclassify.

| Sev | Definition |
|---|---|
| **P0** | data loss / authority bypass / corrupt certified release |
| **P1** | blocker of `BASE_PRODUCTION_READY` |
| **P2** | production-hardening debt with a workaround |
| **P3** | UX / cleanup / opportunistic improvement |

A P0 or undisposed P1 ⇒ `BASE_PRODUCTION_READY` is **false**.

## Final disposition vocabulary

Only `BASE_PRODUCTION_READY` (all mandatory gates evidenced, zero
undisposed blocker) or not. No intermediate label.
