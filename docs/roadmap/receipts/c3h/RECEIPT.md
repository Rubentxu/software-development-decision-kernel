# C3h RECEIPT — Supply-chain audit + remediation

**Cycle:** C3h (Supply-chain audit, ROADMAP §C3 "secret-screen y dependencias/supply chain")
**Type:** Maintenance / Security
**Status:** CLOSED — PASS

## Timeline (UTC)

- 2026-09-22T10:14:00 — baseline audit (2 vulns found)
- 2026-09-22T10:15:00 — SCOPE-CONTRACT created
- 2026-09-22T10:15:32 — `time` pin bumped 0.3.36 → 0.3.47 in Cargo.toml
- 2026-09-22T10:16:54 — T2 build clean (1m 21s)
- 2026-09-22T10:18:51 — T3 tests pass (2931)
- 2026-09-22T10:20:20 — T4 clippy clean
- 2026-09-22T10:20:32 — T5 `cargo update -p quinn-proto --precise 0.11.15` (stale lock entry cleaned)
- 2026-09-22T10:20:51 — T6 post-fix audit: 0 vulns, exit 0
- 2026-09-22T10:21:00 — T7 build post-update clean
- 2026-09-22T10:21:17 — fix commit `9560de1`
- 2026-09-22T10:24:49 — bump commit `d906809` (workspace 1.169.152)

## Commits

| SHA | Type | Description |
|---|---|---|
| 9560de1 | fix(deps) | time 0.3.36 → 0.3.47 (RUSTSEC-2026-0009) + quinn-proto stale entry |
| d906809 | chore(release) | bump 1.169.151 → 1.169.152 |

## Results

| Metric | Pre | Post |
|---|---|---|
| cargo-audit vulns | 2 | 0 |
| workspace tests | 780 (lib only) | 2931 (workspace --lib) |
| clippy warnings | 0 | 0 |
| build clean | yes | yes |

## Findings resolved

- **F1** (quinn-proto 0.11.14, RUSTSEC-2026-0185): stale Cargo.lock entry, not in build graph; targeted update to 0.11.15 eliminated advisory and stale entry.
- **F2** (time 0.3.36, RUSTSEC-2026-0009): direct workspace dep; pin bump to 0.3.47 fixes stack-exhaustion DoS.

## Out-of-scope (deferred)

- `cargo-deny.toml` configuration: would need license allowlist and CI integration. Defer to a future cycle if operator requests.
- CI integration of `cargo-audit`: requires workflow change (authority).
- Upgrades of unflagged dependencies.

## Acceptance check

- [x] Pre-fix audit shows exactly 2 vulns (reproducibility).
- [x] `time` upgraded to 0.3.47 and workspace compiles.
- [x] `cargo test --workspace --lib` → existing tests pass (2931/0).
- [x] `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- [x] `quinn-proto` updated to 0.11.15; post-fix audit clean for it.
- [x] Post-fix audit shows 0 vulns.
- [x] No production code change.
- [x] Workspace version bumped (1.169.151 → 1.169.152).

## Operator-visible artifacts

- `docs/roadmap/receipts/c3h/SCOPE-CONTRACT.md` — scope and exit criteria
- `docs/roadmap/receipts/c3h/AUDIT-RESULTS.yaml` — full pre/post audit trail
- `docs/roadmap/receipts/c3h/UAT-EVIDENCE.yaml` — 7 evidence items, 5 UAT decisions
- `docs/roadmap/receipts/c3h/RECEIPT.md` — this file

## Next

- HEAD: `d906809` (workspace 1.169.152, 20 commits ahead of origin/main)
- C3h closed. Continue with next post-C3 WorkItem or push for release.
