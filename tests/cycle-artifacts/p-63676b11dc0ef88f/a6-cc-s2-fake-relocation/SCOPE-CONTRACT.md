# SCOPE-CONTRACT — a6-cc-s2-fake-relocation

Cycle id: `p-63676b11dc0ef88f/a6-cc-s2-fake-relocation`
Baseline (released): `v1.169.98` → `d9511047ff9bcdc0fb37a190838e23f20a852a7f`
Cycle lead: orchestrator (this session, auto-run)
Mode: hybrid (code + docs; library gating + Cargo feature + dev-dep self-reference)

## 1. Tensión que cierra

CC-S0 (`docs/history/legacy-packages/architecture-a5-a6/architecture-a6/A6-COGNICODE-CC-S0-RECEIPT.md` §3 finding 1)
dejó pendiente:

> The fake provider module is not really "test-only". The fake is `pub`
> and visible to downstream consumers unconditionally. **CC-S1+ will move
> the fake under a `dev-dependencies` / test-feature gate.**

CC-S1 (RECEIPT §4.1) documentó honestamente que la reubicación NO se hizo
en ese ciclo, proponiéndola como alcance de CC-S2 con auditoría previa de
qué crates consumen el fake. Este ciclo cierra esa auditoría y aplica la
reubicación.

## 2. Auditoría de consumidores (pre-implementación)

Búsqueda exhaustiva (`grep -rn "code_intelligence_port_fake\|FakeCodeIntelligenceProvider\|NullCodeIntelligenceProvider"`):

- **Ningún consumidor `src/`** fuera del propio módulo.
- **Ningún consumidor en otros crates** del workspace.
- **4 consumidores `tests/`** dentro de `crates/sddk-engine/tests/`:
  - `a6_cognicode_protocol_spike.rs` (CC-S0 — 6 tests)
  - `a6_cc_s1_static_graph_completeness.rs` (CC-S1 — 12 tests + 1 EXT ignored)
  - `a6_s1_uat_coverage_fake.rs` (A6 macro-cycle — 12 tests)
  - `aiw_s1_cognicode_real.rs` (AIW-S1 — 2 tests + 3 EXT ignored)

**Conclusión**: la reubicación bajo `#[cfg(any(test, feature = "test-support"))]`
es segura. Los 4 archivos de tests se compilan como targets separados donde
`cfg(test)` no se propaga al library crate; la solución estándar es
auto-referenciar `sddk-engine` con `features = ["test-support"]` en
`[dev-dependencies]`.

## 3. Decisión adoptada

- **Gating**: `#[cfg(any(test, feature = "test-support"))] pub mod code_intelligence_port_fake;`
  en `crates/sddk-engine/src/lib.rs`. Cubre tanto el caso in-crate `cargo test`
  (donde `cfg(test)` se propaga al library) como el caso downstream consumer que
  quiera la fake habilitando la feature.
- **Feature explícita**: `[features] test-support = []` en `crates/sddk-engine/Cargo.toml`.
  No se activa por defecto; solo la activan los dev-deps de `sddk-engine` mismo
  (auto-referencia) y los consumers externos que la pidan.
- **Self-reference dev-dep**: `sddk-engine = { path = ".", features = ["test-support"] }`
  en `[dev-dependencies]` del propio crate, con comentario WHY que justifica el
  patrón canónico de Cargo para APIs test-only `pub`.
- **N3** (no modificar arch-spec-021) sigue preservado: ADR-0137 no se enmienda;
  la reubicación se desarrolla dentro del ciclo sin tocar el spec.
- **M9** del SCOPE de CC-S1 cerrado: el fake deja de ser visible a production
  builds. El lint `no_knowledge_to_provider_sdk` queda reforzado (el fake era
  el único provider-tipo que vivía en production namespace).

## 4. MUST

- **M1.** `code_intelligence_port_fake` NO está compilado en production build
  (`cargo build -p sddk-engine` sin features). Verificable con
  `cargo build -p sddk-engine --verbose | grep code_intelligence_port_fake`
  → no output.
- **M2.** Los 4 archivos de tests que importan la fake siguen compilando y
  pasando (CC-S0 6/6, CC-S1 12/12, A6 macro 12/12, AIW-S1 2/2).
- **M3.** El feature `test-support` está declarada en `crates/sddk-engine/Cargo.toml`
  con doc-comment WHY justificando su purpose.
- **M4.** El self-reference dev-dep está declarado con comentario WHY explicando
  por qué Cargo requiere el patrón (integration tests en `tests/` son un target
  separado sin `cfg(test)` propagado al library).
- **M5.** `cargo clippy --workspace --all-targets -- -D warnings` exit 0.
- **M6.** `cargo fmt --check` exit 0.
- **M7.** `cargo test --workspace --offline` → 0 failed.

## 5. MUST NOT

- **N1.** No se introduce una nueva dependencia.
- **N2.** No se modifica el trait `CodeIntelligencePort` ni la implementación
  del `FakeCodeIntelligenceProvider`/`NullCodeIntelligenceProvider` (cambio
  puramente de gating, no de comportamiento).
- **N3.** No se modifica `arch-spec-021` ni el ADR-0137.
- **N4.** No se añade `test-support` al `[features] default = [...]` — debe
  ser opt-in para no exponer la fake en producción.
- **N5.** No se elimina el self-reference dev-dep; es la única forma estándar
  de hacer visible la fake a los integration tests sin propagar `cfg(test)`.

## 6. Diseño aplicado

```rust
// crates/sddk-engine/src/lib.rs
pub mod code_intelligence_port;
/// CC-S2: gated bajo cfg(test) para in-crate cargo test, y bajo la
/// feature `test-support` para downstream consumers que la pidan
/// explícitamente.
#[cfg(any(test, feature = "test-support"))]
pub mod code_intelligence_port_fake;
pub mod code_intelligence_port_mcp;
```

```toml
# crates/sddk-engine/Cargo.toml
[features]
default = []
std = []
# CC-S2: gates code_intelligence_port_fake...
test-support = []

[dev-dependencies]
# ... otros dev-deps ...
# CC-S2: re-importar sddk-engine con la feature habilita la fake
# en integration tests (que se compilan como target separado).
sddk-engine = { path = ".", features = ["test-support"] }
```

## 7. Net delta esperado

| File | Change |
|------|--------|
| `crates/sddk-engine/src/lib.rs` | +10/-1 líneas: gating + doc-comment |
| `crates/sddk-engine/Cargo.toml` | +6/-0 líneas: feature `test-support` + dev-dep self-reference con WHY |

## 8. Falsification battery (gates antes de close)

1. `cargo build -p sddk-engine --offline` exit 0, fake excluded.
2. `cargo build -p sddk-engine --offline --features test-support` exit 0, fake included.
3. `cargo build --release -p sddk-engine --offline` exit 0, fake excluded (no warnings).
4. `cargo test -p sddk-engine --offline --test a6_cognicode_protocol_spike` → 6/6 PASS.
5. `cargo test -p sddk-engine --offline --test a6_cc_s1_static_graph_completeness` → 12/12 + 1 ignored EXT.
6. `cargo test -p sddk-engine --offline --test a6_s1_uat_coverage_fake` → 12/12 PASS.
7. `cargo test -p sddk-engine --offline --test aiw_s1_cognicode_real` → 2/2 + 3 ignored EXT.
8. `cargo test --workspace --offline` → 0 failed.
9. `cargo clippy --workspace --all-targets --offline -- -D warnings` exit 0.
10. `cargo fmt --check` exit 0.

## 9. Out of scope

- ADR-0137 / arch-spec-021 (no se modifican).
- Cualquier cambio en `FakeCodeIntelligenceProvider` o `NullCodeIntelligenceProvider`
  (cambio es puramente de gating).
- Mover la fake a un crate separado (p.ej. `sddk-engine-fakes`) — sería un
  refactor mayor fuera del scope CC-S2; queda como posible follow-up.
- Renombrar el módulo (la reubicación conserva el nombre).

## 10. Limitación a registrar honestamente

CC-S2 cierra el finding 1 de CC-S0 en su forma funcional: la fake ya no
está en el namespace de producción. La cobertura de los 6 tests de CC-S0
sigue verde y los 4 archivos de tests que la usan compilan y pasan con el
patrón canónico de auto-referencia dev-dep + feature gate.

No se cierra ningún ciclo que requiera `cognicode-mcp` real (CC-S3) — esa
queda bloqueada por la ausencia del binario en este entorno, y se activa
en el release flow canónico (`scripts/release.sh`).