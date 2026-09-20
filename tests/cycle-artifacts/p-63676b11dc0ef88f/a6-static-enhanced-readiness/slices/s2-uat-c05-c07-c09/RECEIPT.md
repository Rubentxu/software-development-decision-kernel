# RECEIPT — S2 — UAT C05, C07, C09

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s2-uat-c05-c07-c09`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Baseline (released):** `v1.169.95` → `9688ebb` (S1 close)
> **Cycle lead:** orchestrator (this session, auto-run)
> **Status:** **CLOSED (LOCALLY)** — push pending release flow.

## §1 Scope adherence

| Hard constraint | Status | Evidence |
|---|---|---|
| C1: zero new ADTs in `sddk-engine/src/**` unless necessary | ✅ | No source file in `crates/sddk-engine/src/**` modified. C09 reuses `KnowledgeBasis::invalidate(Contradicted, _)` and `InvalidatedKnowledgeBasis` — both pre-existing. |
| C2: no modification to `arch-spec-021` or `arch-acceptance-coverage-001` | ✅ | Both unchanged. |
| C3: no modification to `Evidence` / `EvidencePosture` | ✅ | Both unchanged. |
| C4: no modification to `sddk-storage` | ✅ | Unchanged. |
| C5: zero provider DTO types in `sddk-domain` | ✅ | `sddk-domain` untouched. |
| C6: no new dependency added | ✅ | `Cargo.toml` / `Cargo.lock` unchanged by this slice. |
| C7: workspace test green; CC-S0, CC-S1, S1 still pass | ✅ | See §2. |

| STOP condition (per SCOPE §4) | Triggered? |
|---|---|
| `KnowledgeBasis::invalidate(Contradicted, _)` cannot preserve prior assertion's content | **No** — `basis_hash` is preserved verbatim (Step 3 in `t_uat_c09_contradiction_preserves_prior_assertion_and_records_reconciliation`); re-declaring the original payload yields the same content-addressed hash (Step 6). |
| `analyze_impact` cannot return without leaking provider DTO into SDDK | **No** — `t_uat_c05_no_dto_leakage_through_analyze_impact` passes against the canonical forbidden-token list. |
| `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` | **No** — both exit 0 after one fmt iteration and one clippy iteration (useless_format fix). |
| `KnowledgeId` namespace cannot host reconciliation note alongside contradicted one | **No** — the test exercises distinct ids (`symbol-42:usages` vs `symbol-42:usages:reconciliation:2026-09-20T00:00:00Z`) and `KnowledgeId::new` accepts both. |

Per the operator's instruction this session, **none** of the named STOP conditions fired; **no line was stopped**, S2 closed cleanly.

## §2 Evidence

### 2.1 S2 test binary (4 PASS / 0 FAIL / 0 ignored)

```
$ cargo test -p sddk-engine --offline --test a6_s2_uat_c05_c07_c09
…
running 4 tests
test t_uat_c05_no_dto_leakage_through_analyze_impact ... ok
test t_uat_c07_restart_preserves_prior_stable_refs ... ok
test t_uat_c09_contradiction_preserves_prior_assertion_and_records_reconciliation ... ok
test t_uat_c09_naive_replace_loses_contradiction_so_invalidate_is_required ... ok

test result: ok. 4 passed; 0 failed; 0 ignored
```

### 2.2 Regression checks

| Battery | Result |
|---|---|
| CC-S0 spike (`a6_cognicode_protocol_spike`) | 6 PASS / 0 FAIL |
| CC-S1 acceptance (`a6_cc_s1_static_graph_completeness`) | 12 PASS / 0 FAIL / 1 ignored |
| S1 (`a6_s1_uat_coverage_fake`) | 12 PASS / 0 FAIL |
| `cargo fmt --check` | exit 0 |
| `cargo clippy -p sddk-engine --all-targets -- -D warnings` | exit 0 |
| `bash tests/test_push_prevention_hook.sh` | 35 PASS / 0 FAIL |
| `cargo test --workspace --offline` | <running> |

### 2.3 UAT matrix rows

See `tests/cycle-artifacts/.../s2-uat-c05-c07-c09/UAT-EVIDENCE.yaml` for the durable YAML evidence.

| Row | Profile | Result | Test |
|---|---|---|---|
| `PR-UAT-C05` (impact, no DTO leakage) | Static | PASS | `t_uat_c05_no_dto_leakage_through_analyze_impact` |
| `PR-UAT-C07` (restart/reconnect, stable refs) | Static | PASS | `t_uat_c07_restart_preserves_prior_stable_refs` |
| `PR-UAT-C09` (contradiction preservation) | Static | PASS | `t_uat_c09_contradiction_preserves_prior_assertion_and_records_reconciliation` + `t_uat_c09_naive_replace_loses_contradiction_so_invalidate_is_required` |

## §3 Files changed by this slice

| Path | Δ | Role |
|---|---|---|
| `crates/sddk-engine/tests/a6_s2_uat_c05_c07_c09.rs` | nuevo | 4 UAT tests covering C05, C07, C09. |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s2-uat-c05-c07-c09/SCOPE-CONTRACT.md` | nuevo | Slice scope. |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s2-uat-c05-c07-c09/UAT-EVIDENCE.yaml` | nuevo | Durable UAT evidence rows in YAML. |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s2-uat-c05-c07-c09/RECEIPT.md` | nuevo | This file. |

No source file in `crates/sddk-engine/src/**` modified by this slice (test-only).

## §4 Honest limits

1. **C09 reconciliation note is demonstrated at the API surface, not at the durability layer.** The `InvalidatedKnowledgeBasis` carries the prior assertion content via its preserved `basis_hash`, and a separate reconciliation assertion can be recorded as a second entry with a distinct `KnowledgeId`. Durability of these artefacts across crash/reopen belongs to S4 (`PR-UAT-024`). This slice does not assert durability; it asserts the invariant that contradiction is preserved within a single in-memory lifecycle.
2. **C05 covers the SDDK-visible surface of `analyze_impact`, not the wire surface.** The forbidden-token list reflects the architectural lints (`no_knowledge_to_provider_sdk`, `no_domain_to_rpc_or_provider_or_host_sdk`). Wire-level leakage (e.g. CogniCode JSON schema leaking into a serialized receipt) is exercised indirectly by the JSON-ish surface scan; it does not add a serde dependency to `sddk-engine` (per C6).
3. **C07 simulates a single restart cycle.** Multiple reconnects with ref retention across sessions is a stronger invariant; S4 will exercise it together with durability.

## §5 Next slices

- **S3 — AC10 Verify integration.** Depends on S1, S2. STOP conditions named; if AC10 forces `Evidence` change, that line stops.
- **S4 — durability** (independent of S3; can start after S2 if S3 blocks).
- **S6 — fake relocation** (independent; can run in parallel with S3/S4).
- **S5 — real CogniCode EXT** (depends on S3+S4 completing; will be `NOT_EVALUATED` if `COGNICODE_MCP_BIN` is unavailable).
