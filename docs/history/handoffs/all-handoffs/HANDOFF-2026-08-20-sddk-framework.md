# HANDOFF — sddk-framework — 2026-08-20

> **Cycle:** `kernel-cycle-4-internal-debt` (SDDK 2.0)
> **Released as:** v1.32.0
> **HEAD:** `484c3cd` (pre-cycle base) → `?????` (cycle-close, v1.32.0 tagged)
> **Tag:** v1.32.0

## Drift carry-over (not resolved in this cycle)

| Drift | Location | Status |
|-------|----------|--------|
| `uat_common::time::now_rfc3339` (37 LOC Hinnant orphan) | `crates/sddk-pack-uat/src/conformance.rs` (or equivalent) | Pending — pre-existing, has 10+ callers. Cycle 5 candidate. |
| `docs/old/*` (3 subdirs, 712K) | `docs/old/{responsibility-separation,sddk-2.0-architecture-consolidation,sddk-stabilization-plan}` | Pending — decision human (archive vs delete). Cycle 5. |
| `AGENTS.md` LOC budget: 229 vs ≤150 | `AGENTS.md` | Pending — accumulated docs beyond target budget. Cycle 5 candidate. |
| WV-0027 `expires_at` structured field | `docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml:92-107` | Pending — W-001 from cycle 3. Cosmetic. Cycle 5. |
| `civil_from_days` audit exhaustivo | repo-wide | Pending — verify no orphan Hinnants remain after cycle 4. Cycle 5. |

## Last closed cycle

`kernel-cycle-4-internal-debt` (v1.32.0) — closed 4 SUGGESTION/MINOR items from cycle-3 debt-verify (S-001..S-004 partial) without introducing domain features.

## Current state (cargo test / clippy)

```
cargo test --workspace   ✓ green
cargo clippy --workspace ✓ 0 errors
```

## Recovery cheat sheet

```bash
# Verify workspace hygiene
cargo fmt --all -- --check && cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings

# Check AGENTS.md LOC budget (deferred to cycle 5)
wc -l AGENTS.md                                         # 229 (target ≤150)

# Confirm v1.32.0 musl-static binaries
file dist-amd64/sddk-linux-x86_64-musl                  # ELF ... statically linked, stripped

# Rollback this cycle
git reset --hard <pre-cycle-SHA> && git tag -d v1.32.0
```

## What changed (4 commits)

1. `refactor(domain): replace orphan Hinnant in projections.rs with sddk_domain::format::now_rfc3339_utc wrapper` (REQ-K4-004)
2. `feat(domain): add assert_variant_count_eq! macro + apply to 5 trimmed enums` (REQ-K4-001)
3. `refactor(storage): mark proj_store_conn_mut as test-only with doc(hidden)` (REQ-K4-002)
4. `chore(repo): gitignore proptest deterministic-replay cache` (REQ-K4-003, partial)
5. `chore(release): bump to v1.32.0 (kernel-cycle-4-internal-debt)` (REQ-K4-005)

## Next cycle (suggested)

`kernel-cycle-5-cross-cutting-debt` — close 5-cycle backlog items: `uat_common::time::now_rfc3339` (live Hinnant, 10+ callers), `AGENTS.md` LOC budget, `docs/old/*` decision, WV-0027 `expires_at` structured field, post-cycle-4 `civil_from_days` audit.
