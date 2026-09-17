# HANDOFF — A5-PLAN BASE_PRODUCTION_READY Hardening Programme

## Certified state

| | |
|---|---|
| **Cycle** | `p-63676b11dc0ef88f/a5-plan-base-production-ready` (CLOSED) |
| **Budget** | `PLANNING / RISK INVENTORY / ACCEPTANCE DESIGN ONLY` |
| **Certified semantic baseline** | `A4_CERTIFIED` (v1.169.68) |
| **Released baseline** | `v1.169.68` → `3bad25275212f77c3d0d4d664f4d49293aa779c9` |
| **Development head (at open, full)** | `40863474d65874cad5e5a0005fdcb4fa4a962392` |
| **Production release** | **none** (planning-only cycle) |
| **Next** | `A5-1` (not auto-opened) |

## What this cycle delivered (planning only)

Nine documents under `docs/architecture/a5/`:

1. `README.md` — index.
2. `A5-PRODUCTION-READINESS-CONTRACT.md` — the falsifiable definition of
   `BASE_PRODUCTION_READY` (gates G1–G16), severity rules, A4 semantic
   freeze.
3. `A5-RISK-REGISTER.md` — 20 risks with severity, gate, workstream.
4. `A5-DEBT-DISPOSITION.md` — disposition of every inherited item, 9
   lints, 14 ignored tests, 4 compat surfaces.
5. `A5-WORKSTREAM-DAG.md` — A5-1..A5-5 + A5-C, dependency DAG, per-cycle
   contract, feature freeze.
6. `A5-UAT-MATRIX.md` — A4 regression, fresh-machine distribution,
   operator diagnostics, onboarding, authority, concurrency, certification.
7. `A5-FALSIFICATION-MATRIX.md` — durability, concurrency, authority,
   security, property/stress, test reliability, release/distribution.
8. `A5-RELEASE-CERTIFICATION-PROTOCOL.md` — exact-revision certification
   identity set + release governance + A5-PLAN release policy.
9. `A5-PUSH-CONTRACT-INVESTIGATION.md` — the ceremonial-marker finding.

Plus: `docs/debt/INC-A5-PUSH-RELEASE-MARKER-FRICTION.md`, roadmap update,
followups ledger update.

## Key answers (the §26 exit questions)

- **What is `BASE_PRODUCTION_READY`?** Every mandatory gate (G1–G16) has
  durable evidence and zero undisposed blocker. No score.
- **Which risks can invalidate it?** The 20 in the risk register; 7 are
  P0 (data loss, authority bypass, atomicity/TOCTOU, secret leak, A4
  regression, silent overwrite, corrupt CAS).
- **Which debts block it?** None today (0 undisposed P1). Two P1s are
  pre-registered from ignored tests (restart survival; Parallel
  sender-drop) and must be resolved in A5.
- **Which legacy surfaces must disappear?** `paradigm_lens::evaluate_lens`
  (DELETE); `paradigm_lens::LensEvaluation` type and `EvidenceAttachmentV1`
  (MIGRATE_A5).
- **Which production failure modes are tested?** The full falsification
  matrix (durability, concurrency, authority, security, release).
- **How are crash/recovery/rebuild validated?** A5-2 falsification probes
  + projection rebuild equivalence.
- **How are concurrency/authority races validated?** A5-3 probes; the
  decision→effect atomicity boundary must be named or declared a blocker.
- **How is the public binary tested independently of the repo?** UAT-1
  fresh-machine distribution (no checkout, no `cargo run`).
- **How is a certified release tied to an immutable revision?** The
  exact-revision identity set; `certified SHA == tag SHA == binary
  provenance SHA`.
- **What is the cycle DAG?** A5-1 → {A5-2 → A5-3, A5-4, A5-5} → A5-C.
- **What is explicitly POST_BASE?** Everything in
  `A5-DEFERRED-POST-BASE.md`, plus the §21 feature freeze list.

## Release/push investigation outcome

The repeated ceremonial empty `chore(release): bump version (cycle close
marker)` is classified **C — an independent operational defect**, not a
consequence of `INC-A4-RELEASE-VERSION-DRIFT`. Root cause:
`release.sh` pushes the bump commit before the handoff docs exist, so the
docs-only range fails the pre-push hook. Registered as
`INC-A5-PUSH-RELEASE-MARKER-FRICTION` (P2, owner A5-1).

**Note for A5-1:** for a planning/docs-only cycle *without* a release,
a single honest workspace-version bump commit in the same push satisfies
the hook's semantic check via the `[workspace.package]` path — no
ceremonial marker is needed. The friction specifically arises when
`release.sh` pushes the bump separately. This is recorded as evidence.

## Roadmap Delta

```text
A4-CLOSEOUT  CLOSED v1.169.68
A4           CLOSED / CERTIFIED
A5-PLAN      CLOSED (planning)
A5-1         NEXT (not auto-opened)
A5-2..A5-5   blocked_by (per DAG)
A5-C         blocked_by A5-1..A5-5
A5           blocked_by A5-C
```

## Anti-encroachment

This cycle changed only `docs/**` and `.sddk/**`. No runtime semantics,
no compatibility deletion, no flake fix, no `release.sh` change, no lint
promotion, no `AuthorityEngine`/CAS/storage change, no provider.
`BASE_PRODUCTION_READY` is **not** claimed.

## STOP

`A5-1` is **not** auto-opened.
