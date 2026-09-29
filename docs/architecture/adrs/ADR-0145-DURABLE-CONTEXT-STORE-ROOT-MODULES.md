---
id: ADR-0145-DURABLE-CONTEXT-STORE-ROOT-MODULES
status: accepted
supersedes_history: false
proposed_at: 2026-09-29
accepted_at: 2026-09-29
accepted_by_cycle: c3j
references:
  - docs/sddk-hypermedia-workflow-platform-evolution-2026-09-29/03-specs/SPEC-005-DURABLE-CONTEXT-SESSION-HANDOFF.md
  - docs/roadmap/ROADMAP.md#c3j
  - docs/architecture/adrs/ADR-0140-agentic-integration-root-modules.md
  - crates/sddk-engine/src/durable_capsule_store.rs
  - crates/sddk-engine/src/durable_session_binding.rs
---

# ADR-0145: Módulos raíz de contexto durable (CapsuleStore y SessionBindingStore)

- **Estado**: Accepted
- **Fecha**: 2026-09-29
- **Decisión**: Aceptar `durable_capsule_store.rs` y `durable_session_binding.rs` como
  módulos raíz nuevos del engine, porque cada uno implementa **un seam existente**
  (no crea un modelo nuevo) y cumple el guard
  `no_new_root_level_context_module_without_adr` de
  `crates/sddk-cli/tests/context_fitness.rs`.

## Contexto

C3j slice 1 (commit `f35c5e82`) añadió dos módulos raíz al engine para
materializar CTX-001 (CapsuleStore durable) y CTX-002 (SessionBindingStore
durable). El guard arquitectónico `no_new_root_level_context_module_without_adr`
acepta un módulo raíz nuevo **iff su nombre aparece en una ADR**: los módulos
root-level de contexto son superficie de arquitectura, no detalle de
implementación.

Sin ADR, el guard quedó RED desde ese commit. Es deuda real y no cosmética: un
módulo raíz de contexto sin ADR es exactamente el segundo-acto del problema que
el guard existe para evitar (autoridad duplicada en la capa de contexto).

## Opciones

### (a) Mover los módulos dentro de un módulo de contexto existente

Rechazada. `context_capsule.rs` es el **compilador** (inputs → capsule); meter
allí la persistencia mezcla dos responsabilidades con perfiles de error
distintos (`CapsuleError` vs `DurableStoreError`) y con ciclos de dependencia
entre el compilador y su store.

### (b) Un solo módulo `durable_context.rs` con ambos stores

Rechazada. Son seams independientes con contratos independientes:
`CapsuleStore` (cold start, rehydration) y el binding store (sesión host↔SDDK).
Fusionarlos crea un módulo que hay que abrir para leer cualquiera de los dos.

### (c) Dos módulos raíz, uno por seam — **aceptada**

Cada módulo implementa un trait ya existente, vive en su propio fichero y es
testeable en aislamiento. El coste es la superficie raíz, y por eso lleva ADR.

## Decisión

Se aceptan `crates/sddk-engine/src/durable_capsule_store.rs` y
`crates/sddk-engine/src/durable_session_binding.rs` como módulos raíz del engine.

Condiciones que hacen aceptable la decisión:

1. **Ninguna segunda autoridad.** `FilesystemCapsuleStore` implementa el
   `CapsuleStore` canónico; no define un store paralelo. El binding reusa el
   modelo `AgenticBinding` / `BindingTarget` / `ContextBasis` de
   `agentic_session_binding.rs`; no inventa un tipo de binding.
2. **Resolución de rutas por el resolver compartido.** Ambos toman como raíz el
   `AdoptionPaths` que produce `resolve_xdg_paths`; ningún módulo calcula
   rutas XDG por su cuenta.
3. **Contrato de fallo explícito.** Corrupción en disco devuelve error tipado; no
   se entrega capsule ni binding parcialmente válido.
4. **Claves colon-free.** La clave de capsule no puede contener `:` porque el
   store indexa por el primer componente de `<workflow_run>:<node>:<attempt>`.
   Documentado en `context_cmd::cycle_key`.

## Consecuencias

- El guard arquitectónico vuelve a GREEN sin baseline ni excepción.
- La superficie raíz del engine crece en 2 ficheros; el coste está pagado por
  esta ADR, que es lo que el guard exige.
- Si en el futuro aparece un tercer store durable de contexto, la decisión por
  defecto es fusionarlo con el más cercano por seam, no crear un tercer módulo
  raíz.

## Alternativas rechazadas

- Bajar el módulo a `crates/sddk-engine/src/context/` sin ADR: el guard lee
  `crates/sddk-engine/src` y aceptaría el movimiento sin ADR. **Rechazado**
  explícitamente: esquivar el guard moviendo el fichero no es cumplirlo. La
  decisión de arquitectura se registra, no se esconde.
- Añadir los módulos a `BASELINE_ROOT_MODULES`: eso convierte deuda en norma.

## Evidencia

- `cargo test -p sddk-cli --test context_fitness` → 7/7 (guard GREEN).
- `cargo test -p sddk-engine --lib` → 1342 passed, 0 failed, 1 ignored.
- Tests de filesystem: 5 (binding) + 4 (capsule) + 5 e2e
  (`tests/durable_context_e2e.rs`).
