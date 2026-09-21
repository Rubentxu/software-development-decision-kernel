# C0-HANDOFF — Snapshot final de C0 + sustancia C1 para el próximo orquestador

> **Slice id:** `p-63676b11dc0ef88f/c0-reconciliation-baseline` (final handoff)
> **Date (UTC):** 2026-09-21T12:16:00Z
> **Status:** STOP-AND-WAIT-FOR-OPERATOR. C0 cerrado con `PASS_OBSERVED_WITH_NOTES`; C1 SCOPE-CONTRACT pendiente hasta release del operador.
> **Audience:** next orchestrator session (post-release) AND human operator reviewing this state.

## §0 What this document is

This is the final snapshot for the C0 cycle. It consolidates:
- The 5 C0 docs already committed under `docs/roadmap/receipts/c0/e8964ac/`.
- The 3 C1 preflights already committed (substance for future SCOPE-CONTRACT).
- A clear STOP condition (operator must run release).
- A clear CONT condition (post-release, the orchestrator can emit C1 SCOPE-CONTRACT).

This is NOT a SCOPE-CONTRACT for C1. It is a handoff so that C1 SCOPE-CONTRACT is written with full context, not from scratch.

## §1 Current state of `main`

```
$ git log --oneline -10
a15dcdc feat(uat): C1 preflight-3 — executed evidence (cargo test + mock probe)
771804e feat(uat): C1 preflight-2 — H02/H05/H06 verified with evidence
a5babb7 feat(uat): C1 preflight — H01 bug confirmed at crates/sddk-engine/src/structured_work.rs:198
151c8ab feat(uat): C0 annexes — pre-release smoke + C1 research notes
b0d6d40 feat(uat): C0 — emit SCOPE-CONTRACT, UAT-EVIDENCE and C0-RECEIPT
e8964ac docs(roadmap): reconcile CURRENT+STATE+journal to SHA 96f5366 (post PR #7)
96f5366 chore(release): bump version a 1.169.128 — AGENTS.md integration (PR #7 §2.10+§10)
13d4131 docs(roadmap): PR #7 — continuidad única, UAT y certificaciones (docs-only)
2ffff31 chore(release): bump version a 1.169.127
2a3b3aa docs(handoff): v1.169.126 pre-staged — code pushed to origin/main at c0be02f
```

```
$ git status -sb
## main...origin/main
```

HEAD = `a15dcdc`, working tree clean, in sync with origin/main.

## §2 What C0 produced (under `docs/roadmap/receipts/c0/e8964ac/`)

| File | Purpose | Status |
|---|---|---|
| `SCOPE-CONTRACT.md` | Goal, STOP conditions, hard constraints, deliverables, method, OOS | ✅ |
| `UAT-EVIDENCE.yaml` | T01 (8 steps, 7 PASS + 1 PRE-RELEASE) + T02 (23 features, 15 PASS_OBSERVED HISTORICAL, 8 NOT_RUN/BLOCKED) | ✅ |
| `C0-RECEIPT.md` | Status, goals vs results, gates G0..G8, risks, limitations, next action | ✅ |
| `C0-PRE-RELEASE-SMOKE.md` | sha256 of pre-built binary + smoke output (for operator's sanity) | ✅ |
| `C1-RESEARCH-NOTES.md` | Initial questions for H01..H06 (corrected post-preflight) | ✅ |
| `C1-PREFLIGHT.md` | H01 bug verbatim at `crates/sddk-engine/src/structured_work.rs:198` | ✅ |
| `C1-PREFLIGHT-2.md` | H02/H05/H06 with verbatim code evidence + gap confirmation for H05 | ✅ |
| `C1-PREFLIGHT-3.md` | Executed evidence: `cargo test` 5/5 + mock probe + IdempotencyKey clarification | ✅ |

## §3 What's pending (operator-side)

**System-law gate `git.release`:**

```bash
cd ~/Proyectos/agentesIA/sddk-framework
git fetch origin main
git checkout main && git pull --ff-only
bash scripts/release.sh
```

Expected outcome (after release.sh completes):
- Tag `v1.169.128` exists locally.
- GitHub Release `v1.169.128` is published with 9 assets.
- Local install `~/.local/share/sddk/framework/1.169.128/` is in place.
- `sddk version` reports `binary: 1.169.128, resolved: 1.169.128`.

If release fails: open a recovery cycle with the diagnosis. Do not push another bump without resolving the underlying issue.

## §4 What the next orchestrator should do (post-release)

1. **Verify T01 post-release.** Run:
   ```bash
   git rev-parse HEAD
   git tag -l 'v1.169.12*' --sort=-version:refname | head -3
   gh release list --limit 3 --json tagName,createdAt
   grep '^version' Cargo.toml
   ```
   Expected: HEAD = release tag SHA, `Cargo.toml` version == `v1.169.128`, last public release = `v1.169.128`.

2. **Mark C0 CLOSED in `docs/roadmap/STATE.yaml`.** Change status from `IN_PROGRESS_C0` to `CLOSED`.

3. **Emit C1 SCOPE-CONTRACT** at `docs/roadmap/receipts/c1/<sha>/SCOPE-CONTRACT.md`. Use the substance from `C1-PREFLIGHT.md`, `C1-PREFLIGHT-2.md`, `C1-PREFLIGHT-3.md` as input. The 4 H0X items have specific evidence:

   | H0X | Specific evidence already collected | Recommended scope |
   |---|---|---|
   | H01 | bug at `structured_work.rs:206`; `_ => true`; 5 SAW tests pass; mock probe reproduces | Add test `saw007_unknown_descriptor_rejected` (RED→GREEN); change `_ => true` to `_ => false`; re-run 5 SAW + new SAW-007 |
   | H02 | `proposal::IdempotencyKey` already uses `request_hash`; two `IdempotencyKey` types are distinct concepts (NOT a refactor target) | Verify "same-key-different-payload → reject" test exists; add if missing. Check error interpolation patterns in `#[error(…)]` for sensitive values |
   | H05 | `set_process_service_for_tests` at `authority_ticket_service.rs:111`; only `#[doc(hidden)]`; 0 callers but door unlocked | Gate with `#[cfg(test)]` OR move to `test_support` module. Add regression test that public-api surface does not export the seam |
   | H06 | `redact()` at `gateway/src/lib.rs:233` already canonical; SEC1 tests cover stdout/receipts | Audit: are all entry points using `redact()` consistently? Add grep audit to SCOPE-CONTRACT §method |

4. **Cycle plan for C1** (recommended):
   - Slice A: H01 (1 file change, 1 test addition, blast radius = 1 call site, contained).
   - Slice B: H05 (1 attribute addition, contained).
   - Slice C: H02 (audit + test, no refactor; verify existing contract).
   - Slice D: H06 (audit, optional `redact()` application if any entry point is bare).

   Each slice = separate `fix(engine)` commit + test commit + RECEIPT.

5. **C1-RECEIPT** at the end must:
   - Confirm 5+1 SAW tests pass.
   - Confirm public-api surface does not include `set_process_service_for_tests`.
   - Confirm `proposal::IdempotencyKey` semantics unchanged OR explicit migration note.
   - Confirm all entry points audited for redaction.

## §5 What NOT to do (defended boundaries)

- **Do not bump version autonomously.** That's the system-law `git.release` flow. The orchestrator never invokes `cargo release` or any bump command directly.
- **Do not re-publish without operator confirmation.** Even if release.sh fails, the orchestrator's job is to diagnose, not to retry with a fix.
- **Do not push to main without a SCOPE-CONTRACT first.** The C1 cycle must start with a SCOPE-CONTRACT linked to a SHA.
- **Do not modify A5-* certified artifacts.** They are HISTORICAL; the new C1 cert is a fresh cycle, not a re-cert of the old one.
- **Do not run UAT EXT (cognicode-mcp, chronos-mcp) without the binaries installed.** Per [CERTIFICATIONS.md](../../CERTIFICATIONS.md) §2, "No declarar Full mientras falte un proveedor real."

## §6 What the operator should review

Before running `bash scripts/release.sh`, the operator should skim:
- `C0-RECEIPT.md` — confirms the work is honest and risks are documented.
- `C0-PRE-RELEASE-SMOKE.md` — confirms the binary builds and the sha256 is recorded for sanity.
- `STATE.yaml` — confirms `IN_PROGRESS_C0` is the current status.

After running release.sh, the operator should:
- Verify the tag and release appear.
- Run `sddk version` to confirm the local install picked up the new bundle.
- Optionally run `sddk dev doctor` to verify bundle coherence.

## §7 Honest gaps (not closed by C0)

- G11 Security/secrets NOT_VERIFIED (R14 OPEN_NON_BLOCKER carried from v1.169.88) → C1 may address.
- EXT binaries missing (cognicode-mcp, chronos-mcp) → C2 will need them.
- AIW-S8 X08 (Jev corpus + baseline) DEFERRED per ROADMAP §A4.
- R11 (crate split), J7/J8/J9 → DEFERRED.
- SPEC-013..018 → already adopted in past sessions.

## §8 Two non-blocking notes for the operator

1. **MANIFEST.sha256 is partially stale** for the new C0 docs. `scripts/release.sh` step 4 regenerates it. No manual action needed.
2. **`$CARGO_TARGET_DIR` is `/var/home/rubentxu/cargo-targets/release/`, NOT `./target/release/`.** This is machine-specific. The release.sh script handles it internally.

## §9 Files referenced by this handoff

```
docs/roadmap/ROADMAP.md
docs/roadmap/CERTIFICATIONS.md
docs/roadmap/UAT-MATRIX.md
docs/roadmap/CURRENT.md
docs/roadmap/STATE.yaml
docs/roadmap/SESSION-JOURNAL.md
docs/roadmap/receipts/c0/e8964ac/{SCOPE-CONTRACT,UAT-EVIDENCE.yaml,C0-RECEIPT,C0-PRE-RELEASE-SMOKE,C1-RESEARCH-NOTES,C1-PREFLIGHT,C1-PREFLIGHT-2,C1-PREFLIGHT-3}.md

scripts/release.sh
githooks/pre-push
```

## §10 Status

| Item | Status |
|---|---|
| C0 cycle (reconciliation + baseline) | ✅ CLOSED on orchestrator side (pending operator release for full CLOSED) |
| C0 deliverable files (8) | ✅ All emitted and pushed |
| C1 preflight substance | ✅ Complete for H01..H06 |
| C1 SCOPE-CONTRACT | 🔲 Blocked on operator release |
| Operator release action | 🔲 Blocked on operator (`bash scripts/release.sh`) |
| T01 re-run post-release | 🔲 Blocked on operator release |
| C1 cycle start | 🔲 Blocked on T01 re-run + C0 CLOSED |
