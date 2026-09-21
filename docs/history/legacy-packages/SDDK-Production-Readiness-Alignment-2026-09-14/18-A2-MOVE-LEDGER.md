# 18-A2-MOVE-LEDGER

Per-slice record: `from → to`, facade, consumers migrated, tests, removal state.
Rule: MOVE → VERIFY → CONSOLIDATE → DELETE.

## A2-S0 — Context ownership map (no move)

| field | value |
|---|---|
| from | — |
| to | `17-A2-CONTEXT-OWNERSHIP-MAP.md` |
| facade | none |
| consumers migrated | none |
| tests | n/a (inventory) |
| removal state | n/a |

## A2-S1 — Dependency/context fitness suite (no move)

| field | value |
|---|---|
| from | — |
| to | `crates/sddk-cli/tests/context_fitness.rs` |
| facade | none |
| consumers migrated | none |
| tests | 7 (see `19-A2-DEPENDENCY-FITNESS-RECEIPT.md`) |
| removal state | permanent ratchet |

## A2-S2 — `provider_router` → `completion_provider_router` (RENAME)

| field | value |
|---|---|
| from | `crates/sddk-engine/src/provider_router.rs` (`pub mod provider_router`) |
| to | `crates/sddk-engine/src/completion_provider_router.rs` (`pub mod completion_provider_router`) |
| facade | **none** — internal consumers updated directly; the public *type* re-exports (`ProviderRouter`, `RouteAttempt`, …) keep their names, so no external path breaks |
| consumers migrated | `sddk-engine/src/lib.rs` (`pub mod` + `pub use`), `agent_host.rs` (`crate::provider_router::RouteAttempt`), `circuit_breaker.rs` (doc link), `context_fitness` allowlist |
| tests | `provider_failover_tests` (10), `context_fitness` (7), `clippy -D warnings` |
| removal state | complete (no facade retained) |

Rationale: the module routes **completion/model** providers (ADR-027/SPEC-026) and
must not be confused with the future engineering-intelligence Provider SPI
(`CodeIntelligencePort`/`RuntimeIntelligencePort`). The rename is
behavior-preserving; the module docs now state the distinction explicitly.

## A2-S3 / A2-S4 — physical moves

No module was moved: the ownership map shows the existing crate/module tree
already expresses the target ownership (pure types in `sddk-domain`,
orchestration in `sddk-engine`, persistence in `sddk-storage`, adapters in
`sddk-gateway`/`sddk-vault`). Per the "no decorative directories / don't move for
symmetry" rule, no physical relocation is warranted. `.sddk`-ownership is
expressed by the map + fitness ratchets instead.
