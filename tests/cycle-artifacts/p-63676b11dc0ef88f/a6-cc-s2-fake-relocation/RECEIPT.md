# RECEIPT — a6-cc-s2-fake-relocation

Cycle id: `p-63676b11dc0ef88f/a6-cc-s2-fake-relocation`
Baseline (released): `v1.169.98` → `d9511047ff9bcdc0fb37a190838e23f20a852a7f`
Closing commit (this cycle): `<filled at commit>` (feat engine; bump 1.169.99 en commit chore(release) separado por condición A del `githooks/pre-push`).
Cycle lead: orchestrator (this session, auto-run).
Status: **CLOSED (locally)** — push pendiente de release publication.

## §1 Scope adherence

| MUST | Status | Evidence |
|---|---|---|
| **M1** Fake excluida de production build | ✅ | `cargo build -p sddk-engine --offline --verbose` no compila `code_intelligence_port_fake.rs`. Sin la feature `test-support`, el módulo está configurado fuera. |
| **M2** 4 archivos de tests verde | ✅ | CC-S0 6/6 PASS, CC-S1 12/12 + 1 EXT ignored, A6-macro 12/12 PASS, AIW-S1 2/2 + 3 EXT ignored. Total: 32/32 PASS + 4 EXT ignored. |
| **M3** Feature `test-support` declarada | ✅ | `[features] test-support = []` en `crates/sddk-engine/Cargo.toml` con doc-comment WHY de 5 líneas justificando el purpose. |
| **M4** Self-reference dev-dep con WHY | ✅ | `sddk-engine = { path = ".", features = ["test-support"] }` en `[dev-dependencies]` con comentario explicando el patrón canónico Cargo para `pub` test-only APIs. |
| **M5** `cargo clippy --workspace --all-targets --offline -- -D warnings` exit 0 | ✅ | Verificado. |
| **M6** `cargo fmt --check` exit 0 | ✅ | Verificado. |
| **M7** `cargo test --workspace --offline` 0 failed | ✅ | Verificado. |

| MUST NOT | Status | Evidence |
|---|---|---|
| **N1** Sin nuevas dependencias | ✅ | No se añadió ningún crate nuevo; el self-reference es un patrón, no una nueva dep. |
| **N2** Sin modificar `CodeIntelligencePort` ni los impls de fake/null | ✅ | Cambios puramente en `lib.rs` y `Cargo.toml`. `code_intelligence_port.rs` y `code_intelligence_port_fake.rs` sin cambios. |
| **N3** Sin modificar `arch-spec-021` ni ADR-0137 | ✅ | Spec/ADR sin tocar. |
| **N4** `test-support` NO en default features | ✅ | `default = []` se mantiene; solo dev-deps y consumers externos activan `test-support`. |
| **N5** Self-reference dev-dep preservado | ✅ | Presente en Cargo.toml. |

## §2 Evidence

### 2.1 Production build excludes fake

```text
$ cargo build -p sddk-engine --offline --verbose 2>&1 | grep code_intelligence_port_fake
(no output — el módulo está cfg-out)
```

### 2.2 Test build includes fake via self-reference

```text
$ cargo test -p sddk-engine --offline --test a6_cognicode_protocol_spike
test t1_negotiate_returns_static_enhanced_snapshot_when_advertised ... ok
test t2_analyze_delta_is_deterministic_across_runs ... ok
test t3_protocol_major_mismatch_yields_incompatible ... ok
test t4_cancellation_yields_typed_refusal_no_false_evidence ... ok
test t5_restart_mid_request_yields_replay_or_partial_marker ... ok
test t6_base_mode_first_class_with_provider_unavailable ... ok

test result: ok. 6 passed; 0 failed; 0 ignored
```

### 2.3 All 4 fake-using test files PASS

```text
a6_cognicode_protocol_spike        : 6 passed; 0 failed; 0 ignored
a6_cc_s1_static_graph_completeness : 12 passed; 0 failed; 1 ignored (COGNICODE_MCP_BIN EXT)
a6_s1_uat_coverage_fake            : 12 passed; 0 failed; 0 ignored
aiw_s1_cognicode_real              : 2 passed; 0 failed; 3 ignored (cognicode-mcp EXT)
```

### 2.4 Workspace test (no regression)

```text
$ cargo test --workspace --offline
(... all crates ...)
test result: ok. (varios cientos) passed; 0 failed; (varios) ignored
```

### 2.5 fmt + clippy

- `cargo fmt --check` → exit 0.
- `cargo clippy --workspace --all-targets --offline -- -D warnings` → exit 0.

### 2.6 Pre-push secret screen

`bash tests/test_push_prevention_hook.sh` → matrix result PASS=35 FAIL=0 (sin regresión del fix del hook en cycle `inc-push-hook-canary-purref`).

## §3 Files changed

| Path | Δ | Role |
|---|---|---|
| `crates/sddk-engine/src/lib.rs` | +10/-1 | `#[cfg(any(test, feature = "test-support"))] pub mod code_intelligence_port_fake;` con doc-comment que justifica el patrón. |
| `crates/sddk-engine/Cargo.toml` | +13/-0 | `[features] test-support = []` con WHY; dev-dep self-reference `sddk-engine = { path = ".", features = ["test-support"] }` con WHY. |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cc-s2-fake-relocation/SCOPE-CONTRACT.md` | nuevo | Este contrato. |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cc-s2-fake-relocation/RECEIPT.md` | nuevo | Este receipt. |

## §4 Decisions and deviations

### 4.1 Patrón canónico Cargo para APIs `pub` test-only

El gating bajo `#[cfg(test)]` solo afecta al library crate cuando es compilado
como parte de un `cargo test` in-crate. Para los integration tests en `tests/`
que son un **target separado**, `cfg(test)` no se propaga al library y la fake
queda invisible. La solución estándar de Cargo es auto-referenciar el crate
con `features = ["test-support"]` en `[dev-dependencies]`. Este es el patrón
canónico; cualquier intento de evitarlo (hack con `#[cfg(all(test, ...))]`
más complejo, mover la fake a un crate externo, etc.) sería over-engineering
para esta necesidad.

### 4.2 No se mueve la fake a un crate separado (`sddk-engine-fakes`)

Considerado y descartado: implicaría un refactor mayor (cambio de path,
actualizar todos los imports, ciclo de releases separado) fuera del scope
del finding de CC-S0. La gating bajo Cargo feature es la solución
mínima-viable que satisface el MUST M1 sin sobrecargar el ciclo.

### 4.3 `test-support` no se activa por defecto

Crítico: si se añadiera a `default = [...]`, la fake volvería al namespace
de producción. La separación dev-deps vs production-features es lo que
hace que esta sea una solución real, no cosmética.

## §5 Open follow-ups

1. **CC-S3**: ejecutar la EXT `t_ar_6_ext_real_cognicode_run` con
   `COGNICODE_MCP_BIN` disponible. Sigue bloqueada en este entorno.
2. **AIW-S6 / CC-S4**: definir un `CoverageContract` para un consumer
   real distinto de `verify-kernel` (p.ej. `release-pipeline::static_evidence`).
3. **CC-S5 (regen)**: regenerar `inventory_v1.json` en cada bump; sigue
   manual mientras no exista `just inventory-static-enhanced`.

## §6 References

- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cc-s2-fake-relocation/SCOPE-CONTRACT.md`
- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness/RECEIPT.md` (§4.1 propone CC-S2)
- `docs/architecture/a6/A6-COGNICODE-CC-S0-RECEIPT.md` (§3 finding 1)
- `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md`
- `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`
- `githooks/pre-push` (push admission contract — condition A real bump)