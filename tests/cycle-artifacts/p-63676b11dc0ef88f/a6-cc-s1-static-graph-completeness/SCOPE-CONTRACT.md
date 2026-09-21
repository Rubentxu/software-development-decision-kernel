# SCOPE-CONTRACT — a6-cc-s1-static-graph-completeness

Cycle id: `p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness`
Baseline (released): `v1.169.93` → `99abe1a9cac99f07cd8af62bb0e1eae42af2362a`
Cycle lead: orchestrator (this session, auto-run)
Mode: hybrid (code + docs; SDK-side changes + ADR-0139 + acceptance contract)

## 1. Tensión que cierra

CC-S0 (`p-.../a6-cognicode-protocol-spike`, receipt `docs/history/legacy-packages/architecture-a5-a6/architecture-a6/A6-COGNICODE-CC-S0-RECEIPT.md`) cerró el seam del proveedor estático: el trait `CodeIntelligencePort` y un fake determinista satisfacen IPB-001..010 con 6 tests de falsificación. Pero CC-S0 **no declara `STATIC_ENHANCED`**: la matriz documenta que el spike prueba el seam, no la capacidad.

`arch-spec-021 IPB-004` dice textualmente:

> A profile may only be advertised while its required negotiated capabilities are available.

Esto es un requisito normativo que ya está vigente. Lo que falta es **concretar IPB-004 para el perfil `STATIC_ENHANCED`**: qué contrato de cobertura satisface una observación estática "completa" para un alcance dado, cómo se acredita y qué ocurre cuando se desconoce.

CC-S1 cierra esa concreción. **No** introduce un porcentaje global pinado en código ni un booleano universal. Introduce un **contrato de cobertura versionado, propiedad de SDDK, vinculado a un consumidor concreto**, evaluado contra una **base reproducible** (revisión Git fijada + reglas de alcance explícitas), con un resultado que **conserva lo demostrado, lo incompleto y lo desconocido** (no se admite).

CC-S1 también cierra el **finding 1 del CC-S0 receipt**:

> The fake provider module is not really "test-only". [...] The fake is `pub` and visible to downstream consumers. [...] CC-S1+ will move the fake under a `dev-dependencies` / test-feature gate.

Esta reubicación **no es requisito funcional de cobertura**: es tarea secundaria de frontera de producción, conservando los 6 tests de CC-S0 verdes. Si la reubicación rompe los 6 tests, se reabre y filtra la causa; si no, queda hecha y registrada.

## 2. Decisión adoptada (resumen)

- **ADR-0139** (no `ADR-022`: ese identificador ya está usado por `docs/history/legacy-packages/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/02-BOUNDED-CONTEXTS/knowledge/adrs/ADR-022-KNOWLEDGE-MERKLE-TREE-AND-SEMANTIC-OVERLAY.md` y `docs/history/legacy-packages/sddk-decision-kernel-architecture/03-adrs/ADR-022-ACTIVE-GRAPH-PROJECTION.md`, fuera de la serie canónica). Verificado en HEAD local: la serie canónica `docs/architecture/adrs/` llega hasta `ADR-0138-runtime-evidence-port.md`. `ADR-0139` está libre.

- **Contrato de aceptación normativo** `arch-acceptance-coverage-001` (nuevo) vinculado a `arch-spec-021 IPB-004`. La condición obligatoria para anunciar `STATIC_ENHANCED` no queda escondida en código ni en el ADR: queda reflejada en la especificación.

- **No** se introduce `coverage_claim` escalar en `CodeIntelligencePort`. Se introduce `CoverageContract`, `CoverageBasis`, `CoverageEvaluation` como ADTsSDDK-own.

- **No** se introduce un orquestador paralelo. **No** se introduce un tipo CogniCode-específico en `sddk-domain`. **No** se modifica `arch-spec-021` (la cobertura se desarrolla vía ADR-0130 + acceptance contract, sin enmendar el feature).

## 3. MUST

- **M1 — Verdad de suelo inventariada, no inferida.** Existe un inventario independiente de los archivos del alcance, obtenido de una revisión Git fijada (`git rev-parse HEAD` al cierre del ciclo) y unas reglas explícitas de inclusión/exclusión (`include_globs`, `exclude_globs`, `exclude_paths`). El inventario es un artefacto durable (`crates/sddk-engine/tests/fixtures/static_enhanced/inventory_v1.json`), regenerable con un comando determinista (`just inventory-static-enhanced` o equivalente). **No** se calcula el inventario por `grep` heurístico sobre texto.

- **M2 — Contrato de cobertura versionado, no umbral global.** Una capacidad estática enhanced se anuncia para un alcance `S` solo cuando existe un `CoverageContract { contract_id, contract_version, consumer, scope, required_capabilities[] }` aprobado antes de la ejecución, y la evaluación contra ese contrato es `Satisfied`. El contrato y su versión forman parte de la `CoverageBasis`. El runtime aplica la política aprobada; no elige otro umbral después de conocer los resultados.

- **M3 — Sin proveedor, no enhanced.** Si el proveedor está `UNAVAILABLE`/`DORMANT`/`STARTING`/`INCOMPATIBLE`/`FAILED`/`BUSY` con timeout, el perfil `STATIC_ENHANCED` no se anuncia. Se devuelve `EvidenceGap { reason: provider_unavailable }`. Nunca `PASS` silencioso, nunca `STATIC_ENHANCED=true` por defecto.

- **M4 — Estrategia negociada contra capacidad semántica exigida.** Una estrategia como `lightweight` **puede** satisfacer una claim acotada si demuestra las capacidades y la cobertura exigidas por esa claim. **No** puede demostrar completitud semántica global. La evaluación no rechaza `lightweight` por defecto; lo rechaza para claims que exigen capacidades que `lightweight` no declara. La estrategia figura en `CoverageBasis`.

- **M5 — Tres dimensiones de cobertura conservadas.** Toda `CoverageEvaluation` registra tres dimensiones independientemente:
  - **Inventario:** archivos/unidades del alcance esperado que el proveedor efectivamente analizó.
  - **Semántica:** lenguajes, tipos de símbolo y clases de relación que el proveedor declara representar.
  - **Operativa:** ¿completó el análisis previsto sin errores, cancelaciones, límites alcanzados ni resultados truncados?

  Cada dimension tiene uno de cuatro valores: `Demonstrated`, `Partial`, `Incomplete`, `Unknown`. Se permite `Partial` con un campo `gaps[]` que enumera exactamente qué falta.

- **M6 — Sin inventario externo de símbolos no presente, se declara `Unknown`, no se inventa.** Si el proveedor no informa de alguna dimensión, SDDK no la rellena. Un inventario externo puede demostrar que faltan archivos, pero no siempre puede demostrar que todos los archivos presentes se analizaron semánticamente. La dimension queda `Unknown` y la evaluación queda `Incomplete`.

- **M7 — Tests EXT contra CogniCode real, no cierran CC-S1 por sí solos.** El cierre de CC-S1 requiere una ejecución fresca contra CogniCode real con la revisión, estrategia y alcance identificados. La variable de entorno es `COGNICODE_MCP_BIN` (corregida del M7 inicial que decía `CHRONOS_PROVIDER_BIN`). Si la variable no está seteada, los EXT se marcan `#[ignore = "requires COGNICODE_MCP_BIN"]` y `cargo test --workspace` los lista explícitamente. No cierro CC-S1 sin evidencia real observada en la corrida.

- **M8 — Falsificaciones que importan "clase de fallo" sin falsear PASS.** La batería incluye tres falsificaciones críticas, todas deben pasar:
  - **F-α:** un archivo exigido por el contrato que el proveedor omite → evaluación `Incomplete` con `gaps=[<path>]`. No `Satisfied`.
  - **F-β:** una clase de relación requerida por la claim que la estrategia `lightweight` no puede detectar → evaluación `Incomplete` con `gaps=[<relation_class>]`. No `Satisfied`.
  - **F-γ:** un resultado con todas las rutas presentes pero análisis parcial (errores en algunos archivos) → evaluación `Incomplete` con `gaps=[<error_kind>]`. No `Satisfied`.

- **M9 — Reubicación del fake bajo gating de test (secundario).** El módulo `code_intelligence_port_fake` se mueve bajo `dev-dependencies` con `#[cfg(any(test, feature = "test-support"))]` y un `Cargo.toml` feature explícito. Los 6 tests de CC-S0 siguen verdes con `cargo test -p sddk-engine --offline`. Si los tests requieren cambios, el cambio se justifica línea por línea en el RECEIPT. Si el cambio no es viable sin perder cobertura, se reabre y se documenta.

## 4. MUST NOT

- **N1.** No se introduce un orquestador paralelo ni un nuevo adapter CogniCode-específico.

- **N2.** No se introduce un type CogniCode-específico en `sddk-domain` (cierre del lint `no_knowledge_to_provider_sdk`).

- **N3.** No se modifica `arch-spec-021`. El ADR-0139 y el `arch-acceptance-coverage-001` desarrollan IPB-004 sin enmendar la especificación. El contrato de aceptación está versionado independientemente y vinculado por `arch-spec-021 reference`.

- **N4.** No se hace `STATIC_ENHANCED = true` por defecto en ausencia del provider o en presencia de un provider que no cumple el contrato. La ausencia nunca es PASS.

- **N5.** No se publica `coverage_ratio` como un escalar único. Cualquier número publicado debe estar cualificado por `CoverageContract.contract_id` y `CoverageBasis.revision`.

- **N6.** No se reescribe contador `limit: 0.9` o `threshold: 0.95` como constante global. Cada capacidad/claim exige su propio `CoverageContract`. No se permiten constantes globales.

- **N7.** No se relaja `coverage_ratio` durante la corrida para alcanzar PASS. El contrato se aprueba antes, el runtime lo aplica, no negocia después.

- **N8.** No se reescribe `Cargo.toml` `[workspace.package]` version. El bump ceremonial, si lo hay, es del release posterior. CC-S1 cierra con código y docs, no con bump.

## 5. Diseño aplicado (plan)

```rust
// crates/sddk-engine/src/code_intelligence_port.rs (extensión)
// ADTs SDDK-own; sin tipos CogniCode.

pub struct CoverageContract {
    pub contract_id: String,            // p.ej. "static-enhanced-workspace-v1"
    pub contract_version: SemVer,        // 1.0.0
    pub consumer: String,                // p.ej. "verify-kernel::static_evidence"
    pub scope: ScopeRef,                 // repo + revisión + globs
    pub required_capabilities: Vec<RequiredCapability>,  // qué debe declarar el provider
}

pub struct CoverageBasis {
    pub revision: GitRev,                // pin a revisión Git
    pub inventory: Inventory,            // archivos esperados (M1)
    pub provider_strategy: String,       // "lightweight" | "full" | ...
    pub provider_version: String,        // "0.97.1"
    pub contract_revision: SemVer,        // qué versión de CoverageContract se aplicó
}

pub struct CoverageEvaluation {
    pub inventory: DimensionEvaluation,   // M5 inventario
    pub semantics: DimensionEvaluation,   // M5 semántica
    pub operational: DimensionEvaluation, // M5 operativa
    pub gaps: Vec<CoverageGap>,          // gaps[] registrados
}

pub enum DimensionValue { Demonstrated, Partial, Incomplete, Unknown }

pub enum CoverageVerdict {
    Satisfied,                           // todos Demonstrated
    Incomplete { gaps: Vec<CoverageGap> }, // al menos uno < Demonstrated
}

// Uso:
fn evaluate(contract: &CoverageContract, basis: &CoverageBasis,
            observations: &ProviderObservations) -> CoverageEvaluation;
```

Tres traits/métodos en el port se **extienden** (no se reescriben):

- `CodeIntelligencePort::coverage_evaluation(contract, basis)` — nuevo método, devuelve `Result<CoverageEvaluation, CoverageError>`.
- `CodeIntelligencePort::capabilities()` — existente; devuelve `CapabilitySnapshot` que ahora incluye `available_strategies[]` y `semantic_classes[]` (lo que CogniCode declara como representable).
- `analyze_delta` / `analyze_scope` / `analyze_impact` — sin cambio de signature; su salida adjunta ahora `basis: CoverageBasis` cuando se ejecuta bajo un contrato.

## 6. ADR y contrato de aceptación

- **ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.md** (nuevo, ~140 líneas): qué significa "completo" para `STATIC_ENHANCED`, cómo se acredita, qué ocurre cuando se desconoce, qué consumidor necesita esa garantía.
- **arch-acceptance-coverage-001.md** (nuevo, ~80 líneas): contrato de aceptación normativo vinculado a `arch-spec-021 IPB-004`. Define los tres casos `Satisfied`/`Incomplete`/`Unknown` y las obligaciones del runtime.

## 7. Net delta esperado

| File | Change |
|------|--------|
| `docs/architecture/adrs/ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.md` | nuevo, decisión arquitectónica |
| `docs/architecture/specs/arch-acceptance-coverage-001.md` | nuevo, contrato normativo |
| `crates/sddk-engine/src/code_intelligence_port.rs` | +120/-10 líneas: `CoverageContract`, `CoverageBasis`, `CoverageEvaluation`, `evaluate()`; extiende `CapabilitySnapshot` con `available_strategies[]` y `semantic_classes[]` |
| `crates/sddk-engine/src/code_intelligence_port_fake.rs` | mover a `dev-dependencies` con gating `#[cfg(any(test, feature = "test-support"))]` (M9) |
| `crates/sddk-engine/Cargo.toml` | nuevo `feature = "test-support"`; `code_intelligence_port_fake` movido a `[dev-dependencies]` |
| `crates/sddk-engine/tests/a6_cc_s1_static_graph_completeness.rs` | nuevo, ≥80 líneas: tests deterministas con fixtures (M8 F-α/β/γ) + tests EXT env-gated |
| `crates/sddk-engine/tests/fixtures/static_enhanced/inventory_v1.json` | nuevo, inventario independiente del alcance declarado (M1) |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness/{SCOPE-CONTRACT,DISCOVERY,RECEIPT}.md` | este contrato, su discovery, su receipt |

## 8. Falsification battery (gates antes de close)

1. `cargo test -p sddk-engine --offline --test a6_cc_s1_static_graph_completeness` → 0 FAILED (tests deterministas, sin EXT).
2. `cargo test -p sddk-engine --offline` → 0 FAILED, 6 tests CC-S0 verdes (M9).
3. `cargo test -p sddk-engine --offline --test a6_cc_s1_static_graph_completeness -- --include-ignored` con `COGNICODE_MCP_BIN` seteado → EXT verdes contra CogniCode real.
4. `cargo fmt --check` clean.
5. `cargo clippy --workspace --all-targets -- -D warnings` clean.
6. `cargo test --workspace --offline` → 0 FAILED.
7. `bash tests/test_push_prevention_hook.sh` → 35 PASS / 0 FAIL (no regresión del fix de hook).

## 9. Out of scope

- A6 CC-S2..CC-S3 (persistencia, anti-corruption adapter con tipos de provider).
- AIW-S2..S4 (handoff durable, ampliación por evidencia, captura estructurada).
- AIW-S1b (evolución de Evidence model por contradicción real con `Denies`).
- Cerrar `STATIC_ENHANCED` para todo `sddk-framework` (este ciclo prueba el contrato, no declara el perfil global).
- Modificar `arch-spec-021` (solo se desarrolla por ADR-0139 + acceptance contract).
- Renombrar `code_intelligence_port_fake` (la reubicación conserva el nombre).

## 10. Limitación a registrar honestamente

CC-S1 prueba el **contrato de cobertura**, no declara `STATIC_ENHANCED` para todo el workspace. Una claim global "workspace completo bajo `STATIC_ENHANCED`" requiere:
1. un `CoverageContract` específico para esa claim (no en este ciclo),
2. una ejecución fresca con `COGNICODE_MCP_BIN` y la revisión fijada,
3. una ADR posterior que conecte esa claim al perfil global.

No fabrico cobertura global en CC-S1.