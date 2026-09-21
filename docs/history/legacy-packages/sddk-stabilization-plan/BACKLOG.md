# Backlog técnico — SDDK v3.6

**Estado auditado:** 2026-08-04 (v3.6 completo); 2026-08-07 añadida épica E11 (CP-2026-08, planificada); 2026-08-10 añadida épica E13 (UAT v3, planificada)
**Baseline:** `v0.14.0`
**Informe:** [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md)

> El estado mide criterios de aceptación demostrados en el repositorio actual. Código presente sin integración, gate automático o evidencia suficiente se marca como parcial. Nada de este paquete se considera entregado hasta quedar versionado y protegido por CI.

## Resumen de estado

| Estado | Historias | Significado |
| --- | ---: | --- |
| Completa | 49 | Todos los criterios de la historia tienen implementación y prueba directa. |
| Parcial | 0 | No queda ninguna historia parcial. |
| Desviada | 0 | No queda ninguna desviación contractual conocida. |
| No iniciada | 0 | El backlog completo (v3.6 + E11 + E12) está cerrado. |
| **Total** | **49** | v3.6 + CP-2026-08 + RS-2026-08 completos. |

## Matriz de aceptación

| Historia | Estado | Evidencia actual | Gap que impide cerrarla |
| --- | --- | --- | --- |
| SDDK-101 | Completa | `workflow/workflow.yaml`; `validate_workflow`; tests de dominio; `sddk validate schema` con dereferencia offline | Sin gap funcional demostrado. |
| SDDK-102 | Completa | `sddk generate docs`; SDDK009; tests deterministas; gate CI | Sin gap funcional demostrado. |
| SDDK-201 | Completa | SDDK001 y tests de referencias tipadas | Sin gap funcional de historia; falta automatizarla en CI a nivel roadmap. |
| SDDK-202 | Completa | SDDK002-SDDK004 y fixtures de shell ejecutable | Sin gap funcional de historia; el escaneo de fences es opt-in por diseño. |
| SDDK-203 | Completa | `sddk generate inventory`; SDDK010; README enlaza el inventario | Sin gap funcional demostrado. |
| SDDK-301 | Completa | Identidad remote/scope/UUID, receipt persistido y test contractual | Sin gap funcional demostrado. |
| SDDK-302 | Completa | Receipt v2, hash de configuración y rename atómico | Sin gap funcional demostrado. |
| SDDK-303 | Completa | `adopt repair`; tests ReceiptOnly/LedgerOnly/conflicto/corrupción | Sin gap funcional demostrado. |
| SDDK-401 | Completa | SQLite v1, WAL, foreign keys y migración transaccional | La evolución v2+ queda como riesgo del roadmap, no de este criterio inicial. |
| SDDK-402 | Completa | Cadena hash, triggers append-only, `sddk ledger verify` y test de corrupción | Sin gap funcional demostrado. |
| SDDK-403 | Completa | `frame_id` y `command_id` compartidos por comando y `sddk ledger events --frame` | Sin gap funcional demostrado. |
| SDDK-404 | Completa | `sddk cycle rebuild` restaura la base vacía desde eventos sin reescribir el ledger | Sin gap funcional demostrado. |
| SDDK-501 | Completa | Rechazo de transición, source, artifacts, gates y paths | Sin gap funcional de la API de engine; expuesto por CLI vía `cycle transition`. |
| SDDK-502 | Completa | Leases con owner, expiry, fencing token y `cycle lock acquire/release/status`; transición exige fence si hay lease | Sin gap funcional demostrado. |
| SDDK-601 | Completa | Runner tipado (argv separado, env allowlist, timeout y truncado) en `sddk-gateway` | Sin gap funcional demostrado. |
| SDDK-602 | Completa | `ScopedFs` con raíces restringidas, rechazo de escapes/symlinks y escritura atómica | Sin gap funcional demostrado. |
| SDDK-603 | Completa | `GitExecutor` tipado: inspect, create-branch, commit y tag con postcondiciones verificadas contra Git real | Sin gap funcional demostrado. |
| SDDK-604 | Completa | CAS `ArtifactStore` con SHA-256 obligatorio, deduplicación por contenido y verificación en lectura | Sin gap funcional demostrado. |
| SDDK-701 | Completa | Validación JSON Schema runtime con dereferencia local de `$ref` y `sddk validate agent-result` | Sin gap funcional demostrado. |
| SDDK-702 | Completa | Adaptador legacy (`convert_legacy_map`/`convert_legacy_text`) con warnings de campos no verificables y `sddk agent-result convert` | Sin gap funcional demostrado. |
| SDDK-703 | Completa | `permissions.yaml` + `PermissionPolicy` default-deny y gate en `capability apply --agent/--phase` + `sddk permission check` | Sin gap funcional demostrado. |
| SDDK-801 | Completa | Trait `Forge` neutral sin tipos de proveedor, `MockForge` y tests de contrato | Sin gap funcional demostrado. |
| SDDK-802 | Completa | Adaptador `GitHubForge` vía `gh` con runner tipado y tolerancia a ya-mergeado/ya-publicado | La integración contra GitHub real queda como prueba manual; el parseo y postcondiciones están testeados con runner inyectado. |
| SDDK-803 | Completa | `reconcile_pending` finaliza receipts `started` consultando la realidad del proveedor | Sin gap funcional demostrado. |
| SDDK-804 | Completa | `plan_release`/`apply_release` en Rust con secuencia canónica, idempotencia y convergencia tras interrupciones | Sin gap funcional demostrado. |
| SDDK-901 | Completa | Parser de vault: frontmatter, IDs, tipos, títulos, wikilinks y backlinks en `sddk-vault` | Sin gap funcional demostrado. |
| SDDK-902 | Completa | Índice FTS5 reconstruible e incremental por hash (tags, enlaces, backlinks, status) | Sin gap funcional demostrado. |
| SDDK-903 | Completa | Validación VAULT001-VAULT004: ids, títulos y wikilinks rotos | Sin gap funcional demostrado. |
| SDDK-904 | Completa | Grafo `petgraph`: ciclos, camino de muestra y orden topológico | Sin gap funcional demostrado. |
| SDDK-1001 | Completa | `sddk dev doctor|check|install|verify|uninstall` (equivalente a xtask) | Sin gap funcional demostrado. |
| SDDK-1002 | Completa | Receipt `sddk-install.json` con versión, commit, SHA-256, canal y timestamp; verificación y desinstalación atómicas | Sin gap funcional demostrado. |
| SDDK-1003 | Completa | `sddk release dist` genera binario, checksums.txt, sbom.json y attestation.json; `release verify` valida todo | Sin gap funcional demostrado. |
| SDDK-1005 | Completa | Packs declarativos (RF-012/ADR-0004): `manifest.toml`, validación PACK001-007, `sddk pack validate` y SDDK014 | Sin gap funcional demostrado. |
| SDDK-1006 | Completa | Indexación incremental del vault por hash de contenido (RNF-004) y profundidad FTS con tags/enlaces/backlinks (RF-009) | Sin gap funcional demostrado. |
| SDDK-1007 | Completa | Envolvente de error estructurada (RNF-006): código estable, causa y recuperación en errores del runtime | Sin gap funcional demostrado. |
| SDDK-1101 | Completa | Gaps de datos: costos/tokens estimados por modelo, teleological coherence, context quality real, verdict con receipt (RF-016) | Sin gap funcional demostrado. |
| SDDK-1102 | Completa | Store SQLite central `control-plane.sqlite` (projects/cycles/aggregates) reconstruible (RF-016/ADR-0009) | Sin gap funcional demostrado. |
| SDDK-1103 | Completa | `sddk telemetry ingest` cross-proyecto con upsert idempotente y derive desde ledger (RF-016) | Sin gap funcional demostrado. |
| SDDK-1104 | Completa | `sddk telemetry aggregate` cross-proyecto reusando `compute_aggregate` (RF-016) | Sin gap funcional demostrado. |
| SDDK-1105 | Completa | `sddk telemetry dashboard` HTML autocontenido sin CDN (RF-017/ADR-0010) | Sin gap funcional demostrado. |
| SDDK-1106 | Completa | Research packet cross-proyecto para agentes self-research (RF-016, sin MCP) | Sin gap funcional demostrado. |
| SDDK-1201 | Completa | Adopción no intrusiva: eliminar plantado de `workflow/workflow.yaml` en el repo (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1202 | Completa | Artefactos de ciclo en XDG (`cycle-artifacts/{cycle_id}/`) + prompts/skills actualizados (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1203 | Completa | `generate docs/inventory` → XDG por defecto con `--in-repo` explícito (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1204 | Completa | `lint` lee manifest embebido/bundle, no exige `workflow.yaml` en el repo (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1205 | Completa | Bundle runtime `$SDDK_DATA_DIR/framework/<v>/` multi-versión + `dev use` + link → `current` (ADR-0011/asdf) | Sin gap funcional demostrado. |
| SDDK-1206 | Completa | Migración: limpiar receipts duplicados, mover `sddk/` a XDG, re-linkear editores (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1207 | Completa | Resolución de versión por proyecto: `.sddk-versions` → `current` → `path:` (ADR-0011/asdf) | Sin gap funcional demostrado. |
| SDDK-1208 | Completa | Resolución multiplataforma con crate `dirs`: macOS `~/Library/...`, Windows `%APPDATA%` (ADR-0011) | Sin gap funcional demostrado. |

## ÉPICA E1 — Fuente canónica del workflow

### SDDK-101 Crear `workflow.yaml`

**Prioridad:** P0
**PR:** 1

**Criterios de aceptación:**

- Estados y fases aparecen una sola vez.
- Cada transición declara precondiciones, gates y artefactos.
- Se valida contra schema.

### SDDK-102 Generar documentación del workflow

**Prioridad:** P1
**PR:** 2

- Mermaid y tablas se generan automáticamente.
- CI falla si los generados están obsoletos.

## ÉPICA E2 — Consistencia del repositorio

### SDDK-201 Detectar referencias rotas

**Prioridad:** P0
**PR:** 2

- Detecta agentes, skills, plugins y rutas inexistentes.
- Código de error estable `SDDK001`.

### SDDK-202 Detectar placeholders literales

**Prioridad:** P0
**PR:** 2

- Detecta `{project}`, `~` no expandible en scripts y variables no definidas.

### SDDK-203 Inventario generado

**Prioridad:** P1
**PR:** 1

- El README no mantiene manualmente números de agentes o skills.
- CI falla con `SDDK010` si el inventario generado está obsoleto.

## ÉPICA E3 — Identidad y adopción

### SDDK-301 Resolver identidad lógica

**Prioridad:** P0
**PR:** 3

- Remote normalizado.
- Scope de monorepo.
- Fallback UUID sin remote.

### SDDK-302 Crear receipt de adopción

**Prioridad:** P0
**PR:** 3

- Escritura temporal y rename atómico.
- Incluye versión y hash de configuración.

### SDDK-303 Reparar adopción interrumpida

**Prioridad:** P1
**PR:** 3

- `sddk adopt repair` detecta y resuelve estados parciales.

## ÉPICA E4 — Ledger

### SDDK-401 Crear esquema SQLite

**Prioridad:** P0
**PR:** 4

- WAL y foreign keys activos.
- Migraciones integradas.

### SDDK-402 Implementar cadena hash

**Prioridad:** P0
**PR:** 4

- `sddk ledger verify` detecta alteración o huecos.

### SDDK-403 Implementar frames

**Prioridad:** P1
**PR:** 4

- Todos los eventos de un comando comparten `frame_id`.

### SDDK-404 Replay de estado

**Prioridad:** P0
**PR:** 4

- Reconstruye ciclos desde eventos en una base vacía.

## ÉPICA E5 — Máquina de estados

### SDDK-501 Validar transición

**Prioridad:** P0
**PR:** 4

- Rechaza transición no declarada.
- Explica gates ausentes.

### SDDK-502 Bloqueo y recuperación

**Prioridad:** P0
**PR:** 4

- Locks con owner y lease.
- Recuperación segura de locks huérfanos.

## ÉPICA E6 — Capacidades

### SDDK-601 Runner sin shell arbitrario

**Prioridad:** P0
**PR:** 5

- Programa y argumentos separados.
- Environment allowlist.
- Captura stdout/stderr.

### SDDK-602 Filesystem tipado

**Prioridad:** P0
**PR:** 5

- Escrituras atómicas.
- Paths restringidos al proyecto y vault.

### SDDK-603 Git local

**Prioridad:** P0
**PR:** 5

- Inspect, branch, commit y tag.
- Postcondiciones verificadas con Git real.

### SDDK-604 Almacén de artefactos

**Prioridad:** P1
**PR:** 5

- Deduplicación SHA-256.
- Metadata en SQLite.

## ÉPICA E7 — Agentes

### SDDK-701 Schema de resultados

**Prioridad:** P0
**PR:** 6

- Versionado.
- Artefactos y evidencia tipados.

### SDDK-702 Adaptador legacy

**Prioridad:** P1
**PR:** 6

- Convierte salidas actuales a resultado estructurado.
- Emite warnings y campos no verificables.

### SDDK-703 Permisos por fase

**Prioridad:** P0
**PR:** 6

- Cada agente declara fases y capacidades permitidas.

## ÉPICA E8 — Forge y release

### SDDK-801 Trait `Forge`

**Prioridad:** P0
**PR:** 7

- No contiene tipos específicos de GitHub.

### SDDK-802 Adaptador GitHub

**Prioridad:** P0
**PR:** 7

- Crear PR, leer checks, merge y release.

### SDDK-803 Reconciliación de efectos

**Prioridad:** P0
**PR:** 7

- Resuelve operaciones `unknown` consultando GitHub.

### SDDK-804 Corregir secuencia release

**Prioridad:** P0
**PR:** 1 y 7

- Nunca intenta fusionar después de esperar estado merged.

## ÉPICA E9 — Vault

### SDDK-901 Parser de frontmatter

**Prioridad:** P0
**PR:** 8

- IDs, tipos, relaciones y procedencia.

### SDDK-902 Backlinks e índice FTS

**Prioridad:** P1
**PR:** 8

- Reindexación incremental por hash.

### SDDK-903 Validación de relaciones

**Prioridad:** P0
**PR:** 8

- Relaciones rotas y tipos inválidos producen errores estables.

### SDDK-904 Proyección `petgraph`

**Prioridad:** P2
**PR:** 8

- Ciclos, caminos y orden topológico.

## ÉPICA E10 — Distribución

### SDDK-1001 `xtask install-dev`

**Prioridad:** P1
**PR:** 9

- fmt, clippy, tests, release, install y doctor.

### SDDK-1002 Receipts de instalación

**Prioridad:** P1
**PR:** 9

- Versión, commit, hash y canal.

### SDDK-1003 Publicación estable

**Prioridad:** P1
**PR:** 9

- Binarios, checksums, SBOM y attestations.

## ÉPICA E11 — Control plane local de telemetría (CP-2026-08)

### SDDK-1101 Cerrar gaps de datos de métricas

**Prioridad:** P0
**Milestone:** CP-2026-08

- Estimar `tokens_used` y `cost_estimate_usd` en la captura automática por modelo (`estimate_cost`).
- Persistir `costs` (L1-L6) cuando el ledger/manifiesto los exponga.
- Poblar `teleological_coherence_pct` desde artifacts del ciclo cuando existan.
- Leer `context_quality` real del `context.json` en lugar del default C2.
- Completar `verify_verdict` con el receipt de verify cuando exista.

**Criterio:** `sddk telemetry status` evidencia >0 ciclos con costos y coherence poblados (o gap documentado).

### SDDK-1102 Store SQLite central del control plane

**Prioridad:** P0
**Milestone:** CP-2026-08

- Schema v1 (`projects`, `cycles`, `aggregates`) en `~/.local/share/sddk/control-plane/control-plane.sqlite`.
- Upsert idempotente por `cycle_id`; proyección reconstruible desde JSONL locales.

**Criterio:** `rm control-plane.sqlite && sddk telemetry ingest` reconstruye el mismo estado; doble ingest no duplica.

### SDDK-1103 Ingest de telemetría cross-proyecto

**Prioridad:** P0
**Milestone:** CP-2026-08

- `sddk telemetry ingest` escanea `projects/*/` (adoption.json + metrics.jsonl + ledger.sqlite).
- Derivación de registros pobres desde eventos del ledger (reuso `derive_from_events`).
- `--dry-run` y `--format json|text`.

**Criterio:** ingest registra todos los proyectos adoptados del host y sus ciclos sin duplicados.

### SDDK-1104 Agregación cross-proyecto

**Prioridad:** P1
**Milestone:** CP-2026-08

- `sddk telemetry aggregate --window 7d|30d` reutilizando `compute_aggregate` sobre el store central.
- Persistencia en `aggregates` + `aggregate.json` + `tuning.md` del control plane.

**Criterio:** aggregate 30d con sample ≥ aggregate 7d cuando existen ciclos de más de 7 días.

### SDDK-1105 Dashboard HTML autocontenido

**Prioridad:** P1
**Milestone:** CP-2026-08

- `sddk telemetry dashboard --output` genera HTML estático sin CDN ni red (patrón `export_html`).
- KPIs, tendencias 7d/30d, distribuciones paths/verdicts, bottleneck por proyecto, señales F3.
- Datasets JSON embebidos; determinista.

**Criterio:** HTML sin URLs externas (grep `https?://` y `src=` externos → vacío), abrible vía `file://`, mismo hash para el mismo store.

### SDDK-1106 Research packet cross-proyecto

**Prioridad:** P1
**Milestone:** CP-2026-08

- `sddk analytics research` alimentado desde el store central cuando exista.
- Research packet con resumen por proyecto (`projects: [...]`).
- Agentes self-research actualizados para consumir el packet cross-proyecto (sin MCP).

**Criterio:** el research packet lista ciclos de todos los proyectos con agregados cross-proyecto.

## ÉPICA E12 — Separación de responsabilidades y cero intrusión (RS-2026-08)

### SDDK-1201 Adopción no intrusiva

**Prioridad:** P0
**Milestone:** RS-2026-08

- Eliminar `plant_workflow_manifest`: `adopt apply` no crea ficheros en el repo.
- El engine resuelve el workflow del manifest embebido o bundle runtime.

**Criterio:** `git status` de un proyecto adoptado queda limpio tras `adopt apply`.

### SDDK-1202 Artefactos de ciclo en XDG

**Prioridad:** P0
**Milestone:** RS-2026-08

- Artefactos de ciclo (proposal, spec, tasks, verify-report, release-report) en `~/.local/share/sddk/projects/<id>/cycle-artifacts/{cycle_id}/`.
- Actualizar `persistence-contract.md`, `knowledge-graph/SKILL.md` y `sddk-*.md` con los nuevos paths.

**Criterio:** un ciclo completo no deja ficheros bajo el working tree del proyecto.

### SDDK-1203 Generación de docs a XDG

**Prioridad:** P1
**Milestone:** RS-2026-08

- `sddk generate docs|inventory` escribe a `~/.local/share/sddk/projects/<id>/generated/` por defecto.
- Flag `--in-repo` explícito para el dogfooding del repo de desarrollo.

**Criterio:** `sddk generate` en un proyecto no modifica el working tree salvo con `--in-repo`.

### SDDK-1204 Lint sin dependencia del repo

**Prioridad:** P1
**Milestone:** RS-2026-08

- `sddk lint` lee el workflow del manifest embebido/bundle; no exige `workflow/workflow.yaml` en el repo.

**Criterio:** lint pasa en un proyecto sin `workflow/workflow.yaml` en el working tree.

### SDDK-1205 Bundle runtime y dev link

**Prioridad:** P0
**Milestone:** RS-2026-08

- Bundle runtime instalado en `$SDDK_DATA_DIR/framework/<version>/` (modo bundle de `dev update`, modelo asdf `installs/`).
- Múltiples versiones conviviendo; `sddk dev use <version>` actualiza el symlink `current`.
- `dev link`/`dev doctor` operan sobre `current`; los symlinks del editor apuntan ahí, no al repo de desarrollo.

**Criterio:** symlinks de opencode/zcode apuntan bajo `$SDDK_DATA_DIR/framework/current/`; instalar una versión nueva no altera los prompts activos hasta `dev use`.

### SDDK-1206 Migración del estado existente

**Prioridad:** P0
**Milestone:** RS-2026-08

- Eliminar los 2 receipts de adopción duplicados de `.sddk-shared`.
- Mover artefactos de `sddk/` del working tree a XDG.
- Re-linkear opencode/zcode contra el bundle runtime.

**Criterio:** un solo receipt por workspace; `sddk dev doctor` all_present; control plane ingiere identidades únicas.

### SDDK-1207 Resolución de versión por proyecto (modelo asdf)

**Prioridad:** P1
**Milestone:** RS-2026-08

- Resolución de versión: `.sddk-versions` (PWD → padres) → `current` global → `path:<dir>` para dogfooding.
- El framework nunca escribe `.sddk-versions`; lo gestiona el desarrollador (config declarativa, no estado).
- `SDDK_DATA_DIR` env override para todo el árbol de estado.

**Criterio:** un proyecto con `.sddk-versions` usa su versión pin; sin fichero, usa `current`; `path:` apunta al working tree del repo de desarrollo solo cuando se declara explícitamente.

### SDDK-1208 Resolución multiplataforma de paths

**Prioridad:** P1
**Milestone:** RS-2026-08

- Introducir crate `dirs` en `sddk-engine/src/paths.rs`: overrides `XDG_*`/`SDDK_DATA_DIR` primero, fallback `dirs::data_dir()/state_dir()/cache_dir()` por SO.
- macOS → `~/Library/Application Support/sddk`; Windows → `%APPDATA%\sddk`; Linux → XDG (actual).
- Tests de `paths.rs` con caso fallback `dirs`.

**Criterio:** `resolve_xdg_paths` no depende de `HOME` en SO donde no existe (Windows); tests pasan con y sin overrides; `cargo test` verde en linux + darwin.

## ÉPICA E13 — Human-Governed AI Quality Control Plane (UAT-2026-08-v3)

**Estado:** PLANIFICADA (2026-08-10) — ADR-014 propuesto; plan `docs/uat/PLAN-uat-v3-quality-control-plane.md` aprobado
**Objetivo:** reorientar el UAT de "framework con automatizaciones" a una plataforma de Human-Governed AI Quality / TestOps. Desacoplar executor/evidence/oracles/review policy, separar PASSED de ACCEPTED, review risk-based con sampling, Human Review Queue, disagreement dataset, event log inmutable, exploratory missions con Fara CUA.

### SDDK-1301 Schema v3 del plan (executor/evidence/oracles/review)

**Prioridad:** P0
**Milestone:** UAT-2026-08-v3

- `UatScenarioV3` con 4 ejes independientes: `executor` (cli/api/script/playwright/computer_use/human), `evidence` (bundle: screenshots, trace, console, network, accessibility, geometry, trajectory), `oracles[]` (exit_code/http/text/json_schema/dom/geometry/accessibility/visual_diff/visual_ai/llm_rubric/human), `review` (risk_based: require_human_when + sampling).
- Estados separados: `UatStatus` (execution result) + `UatAssessment` (SUPPORTED_PASS/FAIL/UNCERTAIN/CONFLICTING + confidence) + `UatHumanDecision` (PENDING/APPROVED/REJECTED/WAIVED) + `UatAcceptanceStatus` (ACCEPTED/REJECTED/CONDITIONAL/PENDING).
- Migrador `migrate_plan_v2_to_v3` (patrón v1→v2): `automation.status` → executor + review heurística.
- `LATEST_PLAN_SCHEMA_VERSION = 3`; validación y renderer aceptan v1/v2/v3.

**Criterio:** un plan v2 migra a v3 sin pérdida; `uat validate` acepta las 3 versiones; PASSED != ACCEPTED en el dominio (REQ-RF-023).

### SDDK-1302 PlaywrightExecutor + EvidenceCollector en gateway

**Prioridad:** P0
**Milestone:** UAT-2026-08-v3

- `PlaywrightExecutor`: wrapper sobre CLI `playwright` — navega, actúa, captura trace/screenshots/console/network/geometry/axe. Sensor + actuador, NUNCA juez.
- `EvidenceCollector`: normaliza salidas de cualquier executor a `UatEvidenceBundle` content-addressable (sha256) con `environment` (git_sha, app_version, browser, viewport, os) y `execution` (executor, model, model_hash, prompt_hash).
- `ComputerUseExecutor`: adaptador HTTP a Fara (llama.cpp `:8082`, patrón `cua-test-orchestrator`) — goal → trajectory → screenshots (F8).

**Criterio:** `uat run --executor playwright` produce un EvidenceBundle con trace + screenshots + console + a11y; cada artefacto con sha256 verificable.

### SDDK-1303 Oracles deterministas

**Prioridad:** P0
**Milestone:** UAT-2026-08-v3

- `oracles.rs` en gateway: exit_code, http, text, json_schema, dom, geometry, accessibility (axe), visual_diff (`toHaveScreenshot`) — sin IA, testables.
- Cada oracle emite `UatOracleAssessment {verdict, confidence, details}`.
- `uat assess --session FILE` corre oracles sobre la evidencia → machine assessments.

**Criterio:** cada oracle determinista tiene tests; `uat assess` evalúa un bundle sin red ni IA.

### SDDK-1304 Testability Agent (REQ-RF-021)

**Prioridad:** P1
**Milestone:** UAT-2026-08-v3

- Agente `uat-testability`: analiza cada scenario → `UatTestabilityReport` (deterministic, browser_automatable, agentic_automatable, requires_human_judgement, recommended_executor, recommended_oracles[], human_review.recommended, reasons).
- CLI `uat testability --plan FILE`. Recomendación advisory, nunca vinculante.

**Criterio:** para un scenario determinista recomienda cli/script sin review; para uno estético recomienda humano.

### SDDK-1305 Review policy + sampling + review queue (REQ-RF-022)

**Prioridad:** P1
**Milestone:** UAT-2026-08-v3

- `UatReviewPolicy` risk_based: `require_human_when` (business_criticality, ai_confidence, oracle_conflict, first_execution, visual_change, historical_failure_rate) + `sampling` (1-5%).
- CLI `uat review --queue`: pendientes con evidencia; `uat accept --scenario --decision approved|rejected|waived`.
- `UatDisagreement` capturado en cada machine/human conflict → dataset local.

**Criterio:** con sampling 0.02 y 1000 PASS confiados, ~20 aparecen en la queue; cada REJECT humano contra PASS máquina crea un disagreement.

### SDDK-1306 ValidationSession + event log inmutable (REQ-RF-023)

**Prioridad:** P1
**Milestone:** UAT-2026-08-v3

- `UatValidationSession` (Launch): release, commit, env, plan, n ejecuciones, n reviews.
- Event log: `uat.session`/`uat.verdict` existentes + nuevos eventos (ExecutionCompleted, OracleEvaluated, HumanReviewCompleted, AcceptanceGranted/Revoked, ReleaseGateEvaluated) como frames en el ledger (ADR-003).
- CLI `uat event log --release X` reconstruible: requirement → scenario → execution → evidence → assessment → decision → acceptance → gate.

**Criterio:** "¿por qué se aceptó este release?" es respondible con la cadena reconstruible desde el ledger.

### SDDK-1307 Exploratory missions con Fara CUA

**Prioridad:** P2
**Milestone:** UAT-2026-08-v3

- `UatMission`: goal, budget (actions/time), output (findings, screenshots, trajectory).
- `uat mission --plan FILE --goal "..."` ejecuta con ComputerUseExecutor (Fara).
- Hallazgos como findings, no como verdicts de scenario.

**Criterio:** una misión exploratoria con Fara produce findings + trajectory con screenshots sha256 (requiere server Fara).

### SDDK-1308 Dashboard: Human Review Queue + evidence viewer

**Prioridad:** P1
**Milestone:** UAT-2026-08-v3

- Nueva view `review-queue.html`: tarjetas con machine assessment + confidence + evidencia + Approve/Reject/Needs-work.
- Nueva view `evidence-viewer.html`: trace, screenshots, console/network/a11y por execution.
- `report.html` ampliado: verdict + confidence + disagreements.

**Criterio:** HTML sin URLs externas, determinista, abrible vía `file://` (ADR-010/013).

### SDDK-1309 Gates de release v3 + docs

**Prioridad:** P1
**Milestone:** UAT-2026-08-v3

- `release-uat-approved` exige acceptance != PENDING para scenarios con review requerida (REQ-RF-023).
- REQ-RF-019/020 ampliados; ADR-014 a accepted tras dogfood.
- Dogfooding: UAT v3 validando una release real del framework; release v1.7.0.

**Criterio:** el gate no se abre con machine PASS solo; `uat-skipped` sigue auditable (RNF-010).

### SDDK-1310 UAT Form DSL + Schema Validator + Wizard Compiler (REQ-RF-024/025)

**Prioridad:** P0
**Milestone:** UAT-2026-08-v3 (F12)

- `UatStep v3` (instruction/expected/observation/check/evidence/branch), `UatCheck`, `UatCheckpoint`, `UatCompletionPolicy` en el dominio.
- Vocabulario cerrado del DSL (inputs/evidence/oracles/informativos/flujo) validado por schema — specs fuera del vocabulario rechazadas por `uat validate`.
- `Wizard Compiler` + `UI Renderer` deterministas en el kit: los agentes generan YAML validado, el renderer produce componentes UI conocidos. **Los agentes NUNCA generan HTML/JS** (ADR-015).

**Criterio:** misma spec → mismo HTML (hash); spec fuera del DSL rechazada con error estable; ningún HTML del agente entra en el documento.

### SDDK-1311 Guided Runner UX — wizard, blind checks, ratings, checkpoints, diagnostics (REQ-RF-026/027)

**Prioridad:** P0
**Milestone:** UAT-2026-08-v3 (F13)

- Runner como app de primera clase: inbox "My Validations" (requires-attention / in-progress / blocked).
- Wizard por paso con branching dinámico (on pass/fail/blocked → goto), checks con identidad visual (determinista ✓ verde, IA ◉ azul con confidence, humano ○ pendiente).
- Blind checks (expected oculto) + observaciones guiadas + ratings 1-5 con `require_comment_below`.
- Evidence gates: `Continue` bloqueado sin evidencia requerida.
- AI diagnostics en FAIL: evidencia ya recolectada + causa probable + categoría + defecto sugerido.
- Human checkpoints: resumen máquina (checks, Fara assessment, anomalías) + approve/reject.
- Actual Result como concepto de dominio autocompletado por agentes (nunca pedir al humano re-introducir lo observable por máquina).

**Criterio:** un tester ejecuta un scenario completo sin ver HTML/YAML; blind check sin sesgo; FAIL produce defecto casi terminado.

### SDDK-1312 Modos Designer/Runner/Reviewer + sign-off inmutable + staleness (REQ-RF-028)

**Prioridad:** P1
**Milestone:** UAT-2026-08-v3 (F13)

- Tres modos: Designer (requirements/scenarios/coverage/form editor/testability), Runner (wizard/evidence/observation), Reviewer (evidence/AI assessment/disagreements/defects/RELEASE ACCEPTANCE).
- Sign-off inmutable: `UatAcceptanceRecord` con decision, actor, timestamp, `plan_version` sha256, `evidence_snapshot` sha256, `outstanding_findings[]`, justification — válido aunque cambien los tests.
- Staleness: diff de textos/roles de oracles DOM/ARIA marca UAT afectados al cambiar la UI.

**Criterio:** el Reviewer firma un release con snapshot inmutable; el cambio posterior de un label marca los UAT afectados como stale.

### SDDK-1313 Pipeline de agentes: UX Form + Form Quality + Test Discovery

**Prioridad:** P2
**Milestone:** UAT-2026-08-v3 (F14)

- UX Form Agent: transforma criterio semántico → interacción óptima (blind observation + machine check + human confirmation).
- Form Quality Agent: anti-patrones de test smells (arXiv:2308.01386): instrucción ambigua, expected ausente, pregunta de lo observable por máquina, leading question, check duplicado, criterio subjetivo sin escala, failure sin evidencia, step demasiado grande, sin recovery path, prerequisito oculto.
- Test Discovery Agent: Fara + Playwright exploran la app real → Actual Application Model → Guided UAT generado del flujo real (no inventado).

**Criterio:** un requisito + app corriendo produce un wizard UAT descubierto de la UI real, con procedencia (agente/modelo/based_on/confidence/human_reviewed).
