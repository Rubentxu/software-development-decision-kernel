# A6-COGNICODE-CC-S0 RECEIPT — Code Intelligence Port Seam

Cycle: `p-63676b11dc0ef88f/a6-cognicode-protocol-spike`
Baseline (released): `v1.169.88` → `add896d94274a7515254b8c195e2e78c3669108f`
Development head at start: `aa6855e7cea1bf99992c55c9771e2bc51ed7894a`
Final development head: `53e03d0`
Scope: `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cognicode-protocol-spike/SCOPE-CONTRACT.md`
ADR: `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md`
Spec: `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`
Handoff: `docs/SDDK-Production-Readiness-Alignment-2026-09-14/04-COGNICODE-HANDOFF.md`

## Identity

| Field | Value |
|---|---|
| `project_id` | `p-63676b11dc0ef88f` |
| `workspace_id` | `w-2e7853aadc28217a6649e309` |
| `cycle_id` | `a6-cognicode-protocol-spike` |
| `release HEAD` | post-cycle bump (target v1.169.90) |
| `cycle commits` | `1fb8332` (scope + ADR), `53e03d0` (feat + tests) |
| `incidents opened` | 0 |
| `incidents closed` | 0 |
| `findings` | 1 — see §3 |

## §0 Falsification first (RED → GREEN)

| # | Defect | Pre-fix | Post-fix |
|---|--------|---------|----------|
| 1 | `CodeIntelligencePort` trait does not exist in `crates/sddk-engine/src/`. AC10 (static evidence through the port) cannot be met without it. The contract exists in `arch-spec-021` as prose; no Rust code exists for it. | `git grep CodeIntelligencePort crates/sddk-engine/src/` returns nothing. The closest reference is a single informative comment in `crates/sddk-engine/src/completion_provider_router.rs:3` (a different responsibility, not the static-intelligence port). | New module `crates/sddk-engine/src/code_intelligence_port.rs` with the trait, the four mandatory operations (`analyze_delta`, `analyze_scope`, `analyze_impact`, `capabilities`), the lifecycle state enum (7 states), the capability profile enum (BASE / STATIC_ENHANCED / RUNTIME_ENHANCED / FULLY_ENHANCED), the capability snapshot, the analysis basis, the observation set, and the result / digest types. Pinned by 6 falsification tests in `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs`. |

Falsification matrix:

| T | Pre-fix (compile) | Post-fix (run) |
|---|-------------------|----------------|
| T1 negotiate → StaticEnhanced | compile error (module not found) | ok |
| T2 analyze_delta deterministic | compile error | ok |
| T3 protocol-major mismatch → Incompatible | compile error | ok |
| T4 cancellation → typed Cancelled | compile error | ok |
| T5 restart mid-request → Partial marker | compile error | ok |
| T6 Base-mode first-class with UNAVAILABLE | compile error | ok |

## §1 What landed

| Surface | Change | Rationale |
|---|---|---|
| `crates/sddk-engine/src/code_intelligence_port.rs` | new, 411 lines | The trait + types per `arch-spec-021` |
| `crates/sddk-engine/src/code_intelligence_port_fake.rs` | new, 345 lines | The deterministic in-process fake + Null provider |
| `crates/sddk-engine/src/lib.rs` | +1 line | `pub mod code_intelligence_port; pub mod code_intelligence_port_fake;` |
| `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs` | new, 218 lines | 6 falsification tests |
| `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md` | new, 144 lines | The ADR that registers both new modules for the `no_new_root_level_context_module_without_adr` lint |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cognicode-protocol-spike/SCOPE-CONTRACT.md` | new, 167 lines | The scope contract pinned before the implementation |
| `docs/proposals/2026-09-19-a6-cognicode-cc-s0-protocol-spike-PROPOSAL.md` | new, 227 lines | The proposal that justified this cycle (operator authorisation was implicit) |

Total: ~1500 lines net new. Zero lines modified in any existing SDDK
engine surface.

## §2 Gate evidence

| Gate | Evidence | Class |
|---|---|---|
| **FENCE matrix T1..T6** | `cargo test -p sddk-engine --test a6_cognicode_protocol_spike` → 6 passed; 0 failed | OBSERVED |
| **`no_new_root_level_context_module_without_adr`** | `cargo test -p sddk-cli --test context_fitness` → 7 passed; 0 failed. ADR-0137 names `code_intelligence_port` and `code_intelligence_port_fake`. | OBSERVED |
| **`no_knowledge_to_provider_sdk`** | `cargo test -p sddk-cli --test context_fitness` → green. No `cognicode`/`chronos`/`prost`/`tonic` token in `semantic_*.rs` / `evidence_ref.rs` / `vault_boundary.rs` / `why_queries.rs`. | OBSERVED |
| **`no_domain_manifest_provider_or_rpc_crate_dependency`** | context_fitness → green | OBSERVED |
| **`no_jcode_type_in_generic_agentic_contract`** | context_fitness → green | OBSERVED |
| **`no_workbook_canonical_write`** | context_fitness → green | OBSERVED |
| **`no_alignment_to_governance_authority_or_instruction_compiler`** | context_fitness → green | OBSERVED |
| **`no_domain_to_rpc_or_provider_or_host_sdk`** | context_fitness → green | OBSERVED |
| **Full sddk-engine regression** | `cargo test -p sddk-engine --offline` → all test binaries pass (no FAILED entries) | OBSERVED |
| **Profile clean** | `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean | OBSERVED |

## §3 Findings discovered during the cycle

1. **The fake provider module is not really "test-only".** The
   falsification battery lives in `crates/sddk-engine/tests/`,
   which compiles `sddk-engine` without `#[cfg(test)]`. The
   fake is therefore `pub` and visible to downstream consumers
   who depend on `sddk-engine`. **Honesty marker**: the fake is
   public by necessity, not by design intent. CC-S1+ will move
   the fake under a `dev-dependencies` / test-feature gate.
   **Cluster**: `CL-SPIKE-BOUNDARY`. **Disposition**: deferred
   to CC-S1.

## §4 Honest limits (carried forward, expanded)

1. **Spec remains `proposed`.** `arch-spec-021` is implemented
   for IPB-001, IPB-002, IPB-003, IPB-004, IPB-008, IPB-010;
   IPB-005 (requirement semantics), IPB-006 (lifecycle
   integration with engine event log), IPB-007 (full
   reproducible basis), IPB-009 (contradiction preservation),
   IPB-011 (completion / model routing distinction), IPB-012
   (no-LLM intermediary) remain for CC-S1+.
2. **Fake is not CogniCode.** CC-S0 proves the SDDK-side seam;
   production ingestion is CC-S1.
3. **No production adapter.** No `sddk-extension-platform`
   crate, no `cognicode-protocol`, no wire DTOs.
4. **No static evidence into Verify.** AC10 is CC-S1.
5. **Trait lives in `sddk-engine`.** A future decision may move
   it to `sddk-domain` if IPB semantics tighten.
6. **No `serde` / `prost` / `tonic` dependency added.** Trait
   uses SDDK-internal types only; the fake returns digests.

## §5 Test run (scope = spike + regressions)

```text
$ cargo test -p sddk-engine --test a6_cognicode_protocol_spike --offline
… 6 passed; 0 failed …

$ cargo test -p sddk-cli --test context_fitness --offline
… 7 passed; 0 failed …  (all architectural lints green)

$ cargo test -p sddk-engine --offline
… all test binaries pass (no FAILED entries) …

$ cargo fmt --check
… (no output) …

$ cargo clippy --workspace --all-targets -- -D warnings --offline
… Finished `dev` profile [unoptimized + debuginfo] target(s) …
```

## §6 What this cycle does NOT do

- Does not modify `arch-spec-021`.
- Does not introduce a real CogniCode adapter.
- Does not consume static evidence into Verify / Knowledge /
  Alignment (AC10).
- Does not modify any existing SDDK engine surface.
- Does not add a new dependency.
- Does not push to `origin/main`. The implementation commit
  contains `.rs` files outside the `docs/**` allowlist; the
  release script (`scripts/release.sh`) is the only path that
  accepts such a range (per the pre-push hook discipline).

## §7 Linkage

- `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md` — implemented, not amended
- `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md` — accepted in-cycle
- `docs/proposals/2026-09-19-a6-cognicode-cc-s0-protocol-spike-PROPOSAL.md` — the proposal that authorised this cycle
- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cognicode-protocol-spike/SCOPE-CONTRACT.md` — the scope pinned before the implementation
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/04-COGNICODE-HANDOFF.md` — CC-S0..CC-S5 internal milestones

## §8 Next step

CC-S1 — stable provider boundary. The next cycle consumes
`arch-spec-021` IPB-005 / IPB-006 / IPB-007 / IPB-009 /
IPB-011 / IPB-012, plus the production CogniCode adapter
scaffold (wire protocol, on-demand service, basis
reproducibility). Scope and falsification will be written
in a fresh SCOPE-CONTRACT pinned before any code lands.

Out of scope (named explicitly, deferred):

- A7 Chronos RUNTIME_ENHANCED — separate track.
- J2 JCode anti-corruption adapter — separate track
  (`docs/proposals/2026-09-19-jcode-anti-corruption-adapter-PROPOSAL.md`).
- SEC-WORKSPACE-FLAKE — separate, narrow cycle. The release
  of v1.169.89 (SEC-1) is gated by the operator decision
  on whether to wait for the flake fix or authorise a
  bounded `--skip-tests` exception.
