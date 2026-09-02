# State-Driven CLI — Advisor & Context Inference (Research)

> **Status:** research package (input para Epic SD, candidatos cycle-52/53/54).
> **Seed:** requisito del maintainer (2026-09-02): "ir llevando un estado en los comandos tal como hace git… a lo mejor no hace falta declarar todos los comandos y el CLI puede inferir muchos datos a partir del estado actual, evitando el sobre-esfuerzo del LLM de adivinar dónde está y cómo pasarle los argumentos requeridos".
> **Refinamientos del maintainer (vinculantes):**
> 1. **Workflows dinámicos**: los steps cambiarán y serán auto-generados. El advisor NO puede hardcodear una secuencia canónica; debe derivarse de datos declarados.
> 2. **Inferencia > declaración**: el CLI debe inferir argumentos del estado actual en vez de exigir que el LLM los repita en cada comando.

---

## 1. Problema (evidencia reproducida en vivo)

Durante la investigación (2026-09-02) se reprodujo el dolor con comandos reales:

```console
$ sddk cycle status --root . --scope project --cycle p-63676b11dc0ef88f/kernel-cycle-51-supersede-first-class
error[STORAGE_NOT_FOUND]: cycle not found: p-63676b11dc0ef88f/kernel-cycle-51-supersede-first-class
  recovery: create the record or fix the reference
```

Tres falencias observables:

| # | Falencia | Coste para un agente LLM |
|---|----------|--------------------------|
| F1 | Args obligatorios que son **deducibles del estado** (`--root`, `--scope`, `--cycle`, `--project-id`) | El LLM los adivina, se equivoca, reintenta. ~500–2000 tokens y 1–3 iteraciones por incidente (F4 gotcha bare-slug es el caso documentado). |
| F2 | Errores con recovery **genérico** ("create the record or fix the reference") — no citan el comando exacto | El LLM recarga contexto (AGENTS.md §8/§9, mcw.md, orchestrator.md) para recordar el flujo: ~5–15k tokens por ciclo. |
| F3 | Ningún output de comando exitoso indica **el siguiente paso** | El orquestador reconstruye estado vía múltiples queries (load session, query events, leer envelopes) en cada fase: cientos de tokens por fase × 6 fases. |

**Nota importante:** el ciclo kernel-cycle-51 existe como expediente (`cycle-artifacts/`) pero el registro en el ledger runtime no es consultable aquí (cold-start risk aceptado: el runtime no serializa ciclos project-wide; `archive-manifest` es ground-truth). Esto refuerza F3: hoy el estado "dónde estoy" vive disperso entre ledger, artifacts y receipts, y solo el orquestador (con contexto humano) sabe reconstruirlo.

## 2. Cimientos que ya existen (descubrimiento clave)

El inventario del código muestra que **~80% del dominio necesario ya está implementado**:

| Pieza | Dónde | Qué da |
|-------|-------|--------|
| **Transiciones declaradas como datos** | `crates/sddk-engine/src/lib.rs:334-423` (valida `cycle.start`, `from: null`, targets) | El grafo de estados del ciclo es DECLARATIVO. Un advisor puede leer el grafo y computar la frontera de transiciones legales. **Compatible con workflows dinámicos por construcción.** |
| **WorkflowManifest** | `crates/sddk-domain/src/workflow.rs:50` (con `project_identity`) | Los workflows son objetos de dominio de primera clase, no strings. |
| **WorkflowRun (instancia de ejecución)** | `crates/sddk-domain/src/workflow_run.rs:149` (`project_id`, `run_id`, `node_id`, `attempt_seq`) | Ya existe el concepto de instancia de workflow con nodo actual e intentos. Es la base del futuro "workflow auto-generado": el advisor futuro lee el run graph, no un YAML estático. |
| **Lease en storage** | `crates/sddk-storage/src/lib.rs:929-984` (`cycle_leases` SQLite, con owner + fencing_token) | Si hay un lease activo, `--cycle` es 100% inferible sin que nadie lo pase. |
| **Resolución de identidad** | `resolve_project_identity(remote, scope, fallback_seed)` + `sddk project resolve` | `--project-id`/`--scope` inferibles desde git remote + cwd. `--root` inferible con walk-up buscando marcadores (.git, AGENTS.md, manifest). |
| **Rebuild** | `sddk cycle rebuild` | Reconstrucción de snapshot desde ledger events — fallback natural cuando no hay lease ni snapshot. |
| **Precedente de hint accionable** | GAP-UX-1 (cycle-51, v1.66.6): bare-slug → error tipado con forma canónica | Ya empezamos este camino; Epic SD lo generaliza. |

**Lo que falta es la capa UX que los ata**: ninguna infiere args; ningún error cita el comando exacto; ningún comando expone la frontera de transiciones legales.

## 3. Qué robarle a git (modelo de referencia)

Git resuelve exactamente el mismo problema con cuatro mecanismos:

1. **Descubrimiento de contexto por walk-up**: todo comando busca `.git` hacia arriba. Nadie pasa `--repo` nunca. → Nuestro equivalente: walk-up de marcadores de proyecto (manifest de `sddk init` / AGENTS.md / `.git`).
2. **Estado persistente entre comandos**: `HEAD` es un symbolic ref; el index persiste. `git status` **computa y sugiere**: `nothing added to commit but untracked files present (use "git add" to track)`. El output nunca deja al ejecutor sin el próximo paso.
3. **Hints accionables tras errores**: bloques `hint:` con el comando literal (`hint: Use 'git am --show-current-patch=diff' ...`).
4. **Superficie dual humano/máquina**: output porcelain (`--porcelain`, `--json`) para scripting/agentes, output rico para humanos. Exit codes estables.

El patrón unificador: **estado + siguiente acción, siempre emparejados**.

## 4. Restricciones de diseño (del maintainer, vinculantes)

- **D1 — Cero secuencias hardcodeadas en el advisor.** La frontera de "qué viene después" se computa leyendo el grafo de transiciones declarado (+ `WorkflowRun` cuando exista), nunca de una lista fija de fases. Si mañana un workflow dinámico declara otra topología, el advisor la sigue sin cambios de código.
- **D2 — Inferencia con degradación tipada.** Comandos con cero args cuando el estado es no-ambiguo. Cuando hay ambigüedad (p.ej. varios leases activos), error tipado que LISTA los candidatos con sus comandos completos — nunca un `STORAGE_NOT_FOUND` seco.
- **D3 — El ledger conserva autoridad.** La inferencia lee estado; nunca lo inventa. `dry-run` y `rebuild` preservan ledger authority (invariante ya presente en el roadmap del facade).
- **D4 — Superficie dual.** Todo output de advisor tiene forma humana (`hint:`) y forma `--json` para agentes (menos tokens de parseo).

## 5. Propuesta: Epic SD — tres ciclos

### cycle-52 — Context inference (args state-driven) → B-direct, tamaño S/M

- Walk-up de `--root` (marcadores: manifest de sddk-init, `.git`, AGENTS.md).
- `--project-id`/`--scope` desde `resolve_project_identity` (git remote + cwd).
- `--cycle` desde el lease activo del proyecto (único lease = inferencia no-ambigua; N leases = error tipado con lista).
- Flag `--no-infer` (opt-out explícito) y precedencia: arg explícito > inferido > error tipado con candidatos.
- **Aceptación:** `sddk cycle status` sin ningún arg funciona dentro de un proyecto adoptado con un lease activo; en ambigüedad lista candidatos con comandos completos.

### cycle-53 — Frontier advisor (`sddk cycle next`) → B-direct, tamaño M

- Nuevo comando (o modo de `cycle status`) que computa la **frontera de transiciones legales**: lee el grafo declarado del workflow del ciclo + eventos del ledger + presencia de artifacts/receipts.
- Output humano:
  ```text
  phase: verify  (spec ✓ apply ✓ verify ⬜ release ⬜ archive ⬜)
  next:  sddk cycle transition --cycle p-…/kernel-cycle-52 … --to verify.complete --evidence <verify-report.sha256>
  ```
- Output `--json` para agentes: `{"cycle":"…","node":"verify","frontier":[{"command":"sddk cycle transition …","transition":"verify.complete"}]}` (~100–200 tokens).
- **D1 es aquí donde se juega**: la frontera se deriva del grafo declarado. Proof-of-concept con un workflow YAML de topología distinta para demostrar que el advisor no asume A-min.
- **Aceptación:** dado cualquier estado del ciclo, `sddk cycle next --json` produce la transición legal correcta en ≤1 comando; con grafo alternativo (topología no-A-min) sigue correcto sin cambios de código.

### cycle-54 — Actionable hints + reconciliación YAML↔ledger → B-direct, tamaño M

- Generalizar GAP-UX-1: todo error de storage/engine con recovery **cita el comando exacto** (o el comando corregido si hay did-you-mean), no consejo genérico.
- Reconciliar los dos mundos que hoy divergen (el mismatch documentado en cycle-51): los YAML de `prompts/sddk/workflows/*.yaml` (mundo del orquestador de agentes) vs las transiciones declaradas del ledger (mundo del kernel). Objetivo: `sddk cycle next` se convierte en la **única fuente** que los prompts del orquestador consumen para saber "dónde estoy y qué sigue", sustituyendo la recarga de AGENTS.md §8/§9 + mcw.md en cada fase.
- **Aceptación:** grep de errores genéricos "recovery: create the record" = 0 en el CLI; el orchestrator prompt referencia `sddk cycle next --json` como fuente de estado.

### Convergencia posterior (Epic LF)

Cuando Epic LF entregue workflows dinámicos/`workflow_run` instanciado, el advisor pasa a leer el **run graph de la instancia** (`node_id`, `attempt_seq`) en vez del grafo declarado estático. Ninguna de las tres ciclos de Epic SD hace trabajo que haya que tirar: el contrato de frontera (D1/D4) es estable ante ese cambio de fuente.

## 6. Economía de tokens (motivación cuantificada)

| Flujo | Hoy | Con Epic SD |
|-------|-----|-------------|
| Fallo de comando por arg mal inferido | 1–3 iteraciones × (~500–2000 tokens) + posible recarga de docs | 0 (args inferidos) o 1 iteración (error tipado con comando exacto) |
| Orquestador reconstruyendo "dónde estoy" por fase | load session + query events + envelopes ≈ 300–800 tokens × 6 fases | `sddk cycle next --json` ≈ 150 tokens × 6 fases |
| Recarga de conocimiento de flujo (AGENTS §8/§9 + mcw.md) por ciclo | ~5–15k tokens | ~0 (el conocimiento vive en el punto de ejecución) |
| **Total estimado por ciclo** | ~10–25k tokens de burocracia | ~1–2k tokens |

## 7. Riesgos y edge cases

- **Múltiples leases activos** → ambigüedad resuelta con error tipado + lista (D2). No inferir en silencio.
- **Estado sin lease** (post-release, pre-archive) → inferir de artifacts + ledger events; `rebuild` como fallback explícito.
- **Proyectos no adoptados / sin manifest** → walk-up falla → error tipado apuntando a `sddk project resolve` / `sddk init` (no guess).
- **Frontera vacía** (ciclo terminal/superseded) → el advisor lo dice explícitamente; no sugiere nada.
- **Drift entre YAML del orquestador y transiciones declaradas** → cycle-54 lo reconcilia; hasta entonces, el mismatch documentado en cycle-51 sigue siendo el estado conocido.

## 8. Referencias

- Live repro §1 (2026-09-02, sesión cycle-51).
- GAP-UX-1 y Candidate BSG (precedentes de hint accionable): `BACKLOG.md`.
- F4 gotcha bare-slug: `prompts/sddk/orchestrator.md`.
- workflow-yaml-mismatch: memoria de ciclo-51 y §5-cycle-54 de este documento.
- Modelo git: walk-up + HEAD + hints + porcelain (§3).
