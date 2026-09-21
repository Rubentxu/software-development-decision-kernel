# Cycle-29 Handoff: `sddk dev reconcile`

**Subject**: `sddk dev reconcile` — Authoritative IDE reconciliation
**Cycle ID**: `p-52b95ef55999f9de/kernel-cycle-29-cli-authoritative-reconciliation`
**Status**: build phase complete ✅
**Tag**: `v1.45.0` (next minor)
**Branch**: `feat/kernel-cycle-29-cli-authoritative-reconciliation`

---

## Summary

Implemented `sddk dev reconcile`, a new CLI command that detects and fixes drift between:
- `assets/agent-models.yaml` (model mapping)
- `agents/*.md` (bundle agent sources)
- Per-IDE configs (opencode.json, zcode.json, claude agents, codex agents)

Key behaviors:
- **Dry-run by default** — shows drift without mutating files
- **`--apply`** — actually mutates IDE configs
- **`--check`** — exit 1 if drift detected, exit 0 if clean
- **Ownership rule** — user agents (not in bundle) are never reconciled or pruned
- **Field preservation** — unknown fields in IDE configs are preserved
- **Atomic writes** — all mutations use `atomic_write`

## Commits (5 total)

| # | SHA | Message |
|---|-----|---------|
| 1 | `aa8a093` | test(cli): RED tests for sddk dev reconcile (cycle-29) |
| 2 | `14e83db` | feat(cli): sddk dev reconcile GREEN plumbing (cycle-29) |
| 3 | `460e5b6` | docs(reconcile): agent-reconciliation.md + agent-models-registration update (cycle-29) |
| 4 | (this commit) | docs(handoff): cycle-29 handoff (cycle-29) |
| 5 | (fmt) | style(fmt): fix N new fmt violations from cycle-29 GREEN commit (cycle-29 WU-amend) |

## Files Modified

| File | Change |
|------|--------|
| `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | NEW — types + trait |
| `crates/sddk-cli/src/dev/editor_adapters/json.rs` | Added ReconcileAdapter impl |
| `crates/sddk-cli/src/dev/editor_adapters/claude.rs` | Added ReconcileAdapter impl |
| `crates/sddk-cli/src/dev/editor_adapters/codex.rs` | Added ReconcileAdapter impl |
| `crates/sddk-cli/src/dev/editor_adapters/mod.rs` | Added `pub(super) mod reconcile;` |
| `crates/sddk-cli/src/dev/reconcile.rs` | NEW — CLI command |
| `crates/sddk-cli/src/dev/mod.rs` | Added Reconcile variant to DevCommand |
| `crates/sddk-cli/src/dev/tests/reconcile_tests.rs` | NEW — 23 tests |
| `docs/agent-reconciliation.md` | NEW — user guide |
| `docs/agent-models-registration.md` | Added reconcile section |

## Test Results

```
running 23 tests
test dev::reconcile::reconcile_tests::apply_preserves_unknown_json_keys ... ok
test dev::reconcile::reconcile_tests::apply_preserves_unknown_toml_keys ... ok
test dev::reconcile::reconcile_tests::apply_preserves_unknown_yaml_frontmatter_keys ... ok
test dev::reconcile::reconcile_tests::check_exit_code_one_with_drift ... ok
test dev::reconcile::reconcile_tests::check_exit_code_zero_without_drift ... ok
test dev::reconcile::reconcile_tests::editor_capabilities_claude ... ok
test dev::reconcile::reconcile_tests::editor_capabilities_codex ... ok
test dev::reconcile::reconcile_tests::editor_capabilities_opencode ... ok
test dev::reconcile::reconcile_tests::editor_capabilities_zcode ... ok
... (all 23 pass)
```

## Next Cycle: Cycle-30

**Deferred from cycle-29**: Map source-context isolation + cross-tick replay

This was scoped out of cycle-29 to deliver reconcile on time. The source-context work will enable:
- Per-agent prompt template variables
- Cross-tick state preservation for long-running agents
- See ROADMAP.md §Cycle-30

## Debt Findings

- Pre-existing fmt drift in `crates/sddk-cli/src/{cycle.rs, inventory_cycle.rs}` (22 files) — tolerated per cycle-28 precedent
- `model_validator: Option<fn(&str) -> bool>` produces function pointer comparison warnings — not actionable without context

## Operational Notes

- Same environment setup as cycle-28
- Local CI gate: `cargo build && cargo test -p sddk-cli && cargo clippy -p sddk-cli`
- Workspace fmt drift tolerance: 22 pre-existing files out of scope

## Verification Checklist

- [ ] `cargo build --release -p sddk-cli` — green
- [ ] `cargo test -p sddk-cli --lib -- reconcile` — 23 tests pass
- [ ] `cargo fmt -p sddk-cli -- --check` — clean (or fmt commit fired)
- [ ] Branch pushed to origin (release phase)
- [ ] Tag `v1.45.0` created (release phase)
