---
id: ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT
status: proposed
supersedes_history: false
proposed_at: 2026-09-20
related_adrs:
  - ADR-0137-CODE-INTELLIGENCE-PORT-SEAM
  - ADR-0138-runtime-evidence-port
references:
  - arch-spec-021 IPB-004 (capability negotiation is runtime authority)
  - docs/architecture/a6/A6-COGNICODE-CC-S0-RECEIPT.md
  - tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness/SCOPE-CONTRACT.md
---

# ADR-0139: Coverage Contract for `STATIC_ENHANCED` Profile

## Contexto

`arch-spec-021` define cuatro perfiles negociables (`BASE`, `STATIC_ENHANCED`,
`RUNTIME_ENHANCED`, `FULLY_ENHANCED`) y exige en IPB-004 que un perfil solo
puede anunciarse mientras las capacidades negociadas requeridas estén
disponibles. IPB-002 además obliga a que el proveedor sea fuente de evidencia,
no autoridad de verdad, y IPB-005 clasifica la exigibilidad por consumidor
(`OPTIONAL` / `PREFERRED` / `REQUIRED`).

CC-S0 (A6-COGNICODE-CC-S0-RECEIPT, `53e03d0`) cerró el seam: el trait
`CodeIntelligencePort` y un fake determinista satisfacen IPB-001..010 con 6
tests de falsificación, pero **no declaran `STATIC_ENHANCED`**: el spike prueba
el transporte, no la completitud.

CC-S1 ejecuta la caracterización de qué información ofrece realmente el
proveedor estático (CogniCode v0.97.1) y traduce IPB-004 en una condición
operativa: un `CoverageContract` versionado, propiedad de SDDK, vinculado a un
consumidor concreto, evaluado contra una base reproducible, con un resultado
que conserva lo demostrado, lo incompleto y lo desconocido.

Este ADR cierra **CC-S1** y deja abierta la declaración del perfil global
`STATIC_ENHANCED` para un ciclo posterior con su propio contrato, ejecución y
ADR.

## Decisión

### 1. Tres dimensiones de cobertura

Toda observación estática bajo un `CoverageContract` declara, **independientemente**:

| Dimensión | Pregunta | Valores |
|-----------|----------|---------|
| **Inventario** | ¿qué archivos/unidades del alcance esperado fueron efectivamente analizados? | `Demonstrated` / `Partial` / `Incomplete` / `Unknown` |
| **Semántica** | ¿qué lenguajes, tipos de símbolo y clases de relación declara representar? | (mismo entre dimensión) |
| **Operativa** | ¿completó el análisis sin errores, cancelaciones, límites ni truncamientos? | (mismo entre dimensión) |

La evaluación global es `Satisfied` solo si **todas las dimensiones son
`Demonstrated`**. Cualquier dimensión `Partial`, `Incomplete` o `Unknown`
produce `Incomplete { gaps: [...] }` con los gaps exactos registrados.

### 2. Contrato versionado, no umbral global

Una capacidad estática enhanced se anuncia para un alcance `S` solo cuando
existe:

```text
CoverageContract {
    contract_id:    String,            // p.ej. "static-enhanced-workspace-v1"
    contract_version: SemVer,           // 1.0.0
    consumer:       String,            // p.ej. "verify-kernel::static_evidence"
    scope:          ScopeRef,          // repo + revisión + globs
    required_capabilities: Vec<RequiredCapability>,
}
```

- El contrato se aprueba **antes** de la ejecución.
- Su versión figura en la `CoverageBasis`.
- El runtime aplica la política aprobada; no elige otro umbral tras conocer
  los resultados.
- No existen constantes globales tipo `coverage_ratio >= 0.9`. Cada
  capacidad/claim exige su propio contrato.

### 3. Base reproducible, no inferida

Toda evaluación registra `CoverageBasis`:

```text
CoverageBasis {
    revision:           GitRev,        // pin a revisión Git
    inventory:          Inventory,     // archivos esperados, regenerable
    provider_strategy:  String,        // "lightweight" | "full" | ...
    provider_version:   String,        // "0.97.1"
    contract_revision:  SemVer,        // qu�� versión del contrato se aplicó
}
```

- El inventario se obtiene de una revisión Git fijada (`git rev-parse HEAD`
  al cierre del ciclo) y unas reglas explícitas de inclusión/exclusión
  (`include_globs`, `exclude_globs`, `exclude_paths`).
- El inventario es un artefacto durable, regenerable con un comando
  determinista (`just inventory-static-enhanced` o equivalente).
- **No** se calcula el inventario por `grep` heurístico sobre texto: contar
  declaraciones textuales no equivale a contar las unidades semánticas que
  CogniCode modela, y un `grep` puede omitir definiciones multilínea, macros
  y declaraciones con atributos.

### 4. Lo desconocido se conserva, no se rellena

Si el proveedor no informa de alguna dimensión, SDDK no la rellena. Un
inventario externo puede demostrar que faltan archivos, pero no siempre puede
demostrar que todos los archivos presentes se analizaron semánticamente. La
dimensión queda `Unknown` y la evaluación queda `Incomplete`. `PASS` no es
una opción para `Unknown`.

### 5. Estrategia negociada, no blanket-rejected

Una estrategia como `lightweight` **puede** satisfacer una claim acotada si
demuestra las capacidades y la cobertura exigidas por esa claim. **No** puede
demostrar completitud semántica global. La evaluación no rechaza `lightweight`
por defecto; lo rechaza para claims que exigen capacidades que `lightweight`
no declara.

### 6. Sin provider = sin enhanced

Si el proveedor está `UNAVAILABLE`/`DORMANT`/`STARTING`/`INCOMPATIBLE`/
`FAILED`/`BUSY`-timeout, el perfil `STATIC_ENHANCED` no se anuncia. Se devuelve
`EvidenceGap { reason: provider_unavailable }`. `STATIC_ENHANCED=true` por
defecto está prohibido.

## Consecuencias

### Positivas

- Una capacidad enhanced se anuncia solo cuando puede **demostrar** la
  cobertura que su consumidor necesita, no cuando parece cumplirla.
- El contrato y la base forman parte del recibo: una evaluación
  `Satisfied` es auditable y reproducible.
- Tres dimensiones independientes hacen explícitas las clases de fallo
  (F-α inventario omitido, F-β semántica no representable, F-γ operativa
  parcial); ninguna queda oculta tras un PASS agregado.
- Cero tipos CogniCode-específicos cruzan la frontera `sddk-domain` (cierre
  del lint `no_knowledge_to_provider_sdk`).

### Negativas

- El inventario durable requiere un comando determinista y un artefacto
  versionado en cada contrato. Coste de mantenimiento: bajo; coste de
  cambio: cualquier modificación del alcance exige nueva revisión + nuevo
  contrato (intencional: el contrato se aprueba antes).
- `lightweight` deja de ser una "opción rápida" para claims globales; para
  esas hace falta `full` o un subconjunto explícito. Intencional: la
  completitud semántica no es gratuita.
- Los tests EXT (contra CogniCode real) no cierran CC-S1 por sí solos: hace
  falta una ejecución fresca con `COGNICODE_MCP_BIN`, revisión y estrategia
  identificadas. Si el binario no está disponible en CI, el cierre no es
  posible hasta que lo esté.

### Neutras

- El perfil global `STATIC_ENHANCED` (para todo `sddk-framework`) requiere
  un ciclo posterior con su propio contrato, ADR y ejecución. CC-S1 no
  cierra ese perfil.
- La serie canónica de ADR mantiene numeración correlativa. ADR-022 está
  usado en paquetes históricos (`docs/SDDK-Context-First-...-2026-09-10/`,
  `docs/sddk-decision-kernel-architecture/`); la serie canónica llega hasta
  `ADR-0138-runtime-evidence-port.md`. La siguiente libre es `ADR-0139`.

## Aceptación

El ciclo de aceptación es `p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness`,
con SCOPE-CONTRACT, DISCOVERY (caracterización del proveedor real) y RECEIPT
con las falsificaciones F-α, F-β, F-γ y los gates
(`cargo test --workspace`, `clippy -D warnings`, `fmt --check`,
`test_push_prevention_hook.sh`).

## Referencias normativas

- `arch-spec-021 IPB-004` — capability negotiation is runtime authority.
- `arch-acceptance-coverage-001` — contrato de aceptación vinculado a este
  ADR.
- `arch-spec-021 IPB-002` — provider is evidence source, not truth authority.
- `arch-spec-021 IPB-005` — requirement semantics (OPTIONAL/PREFERRED/
  REQUIRED).