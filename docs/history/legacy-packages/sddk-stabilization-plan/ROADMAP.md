# Roadmap de entrega — SDDK v3.6

**Estado auditado:** 2026-08-04
**Baseline:** `v0.14.0`
**Informe:** [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md)

> Los números PR 1-9 representan unidades funcionales del plan, no números literales de pull request. El plan PR1-PR9 se entregó entre `v0.1.0` y `v0.10.0`; v3.6 se declaró estable en `v0.11.0` y se endureció en `v0.12.0`-`v0.14.0`.

## Panel de entrega

| PR | Estado actual | Gate | Bloqueo principal |
| --- | --- | --- | --- |
| PR 1 | Completo | CI + SDDK001-SDDK010 | Contrato único e inventario generado demostrados. |
| PR 2 | Completo | Required quality gates | Workspace, linter, generadores y testkit tienen pruebas y CI. |
| PR 3 | Completo | Tests Rust + adopción | UUID persistido, XDG y reparación están alineados con el workflow. |
| PR 4 | Completo | Tests Rust + CLI end-to-end | Ciclo, fases, ledger, leases/fencing y rebuild expuestos por CLI y probados. |
| PR 5 | Completo | Gateway + Git + CAS probados | Capability gateway default-deny, runner tipado, filesystem scoped, Git local con postcondiciones y CAS SHA-256. |
| PR 6 | Completo | Tests Rust + CLI | Schema validation runtime, adaptador legacy y permisos por fase con default-deny. |
| PR 7 | Completo | Tests Rust + MockForge | Forge trait, adaptador GitHub, release plan/apply idempotente y reconciliación contra el proveedor. |
| PR 8 | Completo | Tests Rust + CLI | Parser de vault, índice FTS5 reconstruible, validación y grafo petgraph. |
| PR 9 | Completo | Tests Rust + CLI | `dev doctor/check/install/verify/uninstall`, dist con checksums/SBOM/attestations y verificación atómica. |

## Próximo corte recomendado

El roadmap está COMPLETO y cerrado. Post-estabilidad: hardening, integración dogfood, registro de agentes, packs declarativos, índice incremental y envolvente de error (`v0.12.0`-`v0.18.0`). Con RNF-006, todos los requisitos del PRD (RF-001 a RF-015, RNF-001 a RNF-006) quedan cubiertos.

## PR 1 — Estabilización semántica

**Estado actual:** Completo; canon único e inventario protegidos por CI.

### Alcance

- Corregir adopción, release y ramas.
- Unificar paths.
- Resolver debt verification.
- Eliminar referencias rotas.
- Añadir inventario generado.

### Gate

No existen dos definiciones incompatibles de una misma regla operativa.

## PR 2 — Workspace Rust y linter

**Estado actual:** Completo; workspace, testkit, linter y generación protegidos por CI.

### Entregables

- `sddk-domain`.
- `sddk-engine` mínimo.
- `sddk-storage` mínimo.
- `sddk-cli`.
- `sddk-testkit`.
- `sddk lint`.
- `sddk generate docs`.
- `sddk generate inventory`.

### Gate

CI detecta referencias rotas, placeholders y documentación generada desactualizada.

## PR 3 — Identidad, paths y adopción

**Estado actual:** Completo; implementado, probado y alineado con el contrato canónico.

### Entregables

- Resolución de proyecto y workspace.
- Paths XDG.
- Registro de adopción atómico.
- Comandos `adopt plan/apply/status/repair`.

### Gate

Dos repositorios con igual nombre no colisionan y una adopción interrumpida es reparable.

## PR 4 — Ledger y máquina de estados

**Estado actual:** Completo; autoridad local expuesta por CLI y probada extremo a extremo.

### Entregables

- SQLite.
- Migraciones.
- Frames y cadena hash.
- Ciclos y fases.
- Replay.
- CLI `cycle start|status|transition|rebuild`, `cycle lock`, `ledger verify|events`.

### Gate

Replay reconstruye el mismo estado lógico y las transiciones inválidas se rechazan. El CLI recorre un ciclo completo (adopt → start → transition → verify → rebuild) sin red ni reloj real y las mutaciones exigen fencing cuando el ciclo está leaseado.

## PR 5 — Gateway de capacidades locales

**Estado actual:** Completo; gateway, Git local y CAS implementados y probados.

### Entregables

- Filesystem tipado.
- Process runner.
- Git local.
- Testing.
- Artefactos por hash.

### Gate

Toda acción local relevante queda registrada y es idempotente. El gateway aplica policy default-deny, approvals R3/R4 y receipts `started → succeeded|failed` con redacción; las operaciones Git verifican postcondiciones y el CAS exige y re-verifica SHA-256.

## PR 6 — Protocolo de agentes

**Estado actual:** Completo; schema validation, adaptador legacy y permisos por fase probados.

### Entregables

- Schemas completos.
- Adaptador legacy.
- Permisos por fase.
- Registro de procedencia.

### Gate

Un agente no puede cambiar de fase mediante texto libre. La validación JSON Schema runtime rechaza resultados inválidos, el adaptador emite warnings de campos no verificables y `PermissionPolicy` niega por defecto agentes/fases/capacidades no declaradas.

## PR 7 — Forge y release

**Estado actual:** Completo; trait Forge, adaptador GitHub, release plan/apply y reconciliación probados.

### Entregables

- Trait `Forge`.
- Adaptador GitHub.
- Release plan/apply/reconcile.

### Gate

Un fallo durante merge o publicación se reconcilia sin duplicar efectos. `apply_release` re-chequea el proveedor antes de cada paso, omite efectos ya presentes y `reconcile_pending` finaliza receipts interrumpidos consultando la realidad.

## PR 8 — Vault, índices e Inspector mínimo

**Estado actual:** Completo; parser, validación, FTS5 reconstruible, grafo e inspector HTML probados.

### Entregables

- Parser Markdown/frontmatter.
- Backlinks.
- FTS5.
- Grafo `petgraph`.
- HTML autocontenido.

### Gate

El índice puede borrarse y reconstruirse desde el vault (`vault index` re-crea la tabla FTS5 desde los nodos).

## PR 9 — Distribución

**Estado actual:** Completo; doctor, gates, instalación con receipt y dist verificable probados.

### Entregables

- `sddk dev doctor|check|install|verify|uninstall` (equivalente a xtask).
- `sddk release dist|verify`.
- Checksums.
- SBOM.
- Attestations.
- Instalación side-by-side.

### Gate

Una versión puede instalarse, verificarse, promoverse y revertirse de forma atómica (`dev install/verify/uninstall` con receipt SHA-256; `release dist/verify` con checksums, SBOM y attestation).

## Orden recomendado

No ejecutar PR 7 antes de que PR 4, PR 5 y PR 6 estén consolidados. No introducir LadybugDB dentro de v3.6.

La consolidación exige: cambios versionados, CI obligatoria, criterios del backlog demostrados y ausencia de gaps P0 abiertos en [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md).

---

## Milestone E2E-2026-08 — Validación E2E ampliada (post-v1.3.0)

**Estado:** IMPLEMENTADO (2026-08-06) — 7/7 suites PASS (PR #91)
**Objetivo:** probar instalación real, despliegue, multi-lenguaje y render de diagramas.

| Work item | Tipo | Depende de | Estado |
|-----------|------|-----------|--------|
| scripts/e2e-install.sh (N1) | feature | ADR-0001 | **done** |
| scripts/e2e-render.sh (N2) | feature | ADR-0001 | **done** |
| validate-project.sh --lang (matrix 5 lenguajes) | feature | ADR-0001 | **done** |
| scripts/e2e-all.sh (orquestador) | feature | N1+N2+matrix | **done** |
| docs/validation/e2e-report.md + evidencia | docs | e2e-all | **done** |
| Checklist N3 (editor real) | docs | dev link | **done** |

**Criterios de salida:**
- 5/5 lenguajes validados (adopt + cycle + tests baseline)
- Instalador probado en sandbox sin git (variantes a-d)
- Diagramas renderizados y verificados visualmente (SVG + screenshots)
- Report E2E publicado con evidencia embebida

## Modo de operación: LOCAL-FIRST con act (2026-08-06)

**GitHub Actions cloud DESACTIVADO** (`actions/permissions.enabled = false`). El CI corre **localmente con `act`** (nektos/act v0.2.89) + podman + imágenes `catthehacker/ubuntu:*` (config en `~/.config/act/actrc`).
- Branch protection de main: sin required status checks (los merges no dependen de checks remotos)
- CI validado en local: `act -j required` (fmt + clippy + 39 tests en container) — verde
- auto-merge/release-automation cloud: inactivos (dependían de events de Actions cloud)
- Flujo operativo: validación con `act`, merges vía `gh pr merge`, releases con scripts locales

**Consecuencias:**
- Cada cambio se valida localmente con act antes de mergear (sin consumir minutos del plan)
- El milestone E2E-2026-08 se ejecuta 100% local (podman + mmdc)
- Nota: act no ejecuta jobs de `macos-*` ni `ubuntu-24.04-arm` (solo ubuntu-latest mapeado); el release multi-target sigue siendo un flujo local separado

---

## Milestone CP-2026-08 — Control plane local de telemetría (post-E2E)

**Estado:** IMPLEMENTADO (2026-08-07) — ADRs 0009/0010 aceptados, G1-G6 completos
**Objetivo:** agregar la telemetría de todos los proyectos adoptados en un SQLite central local, cerrar los gaps de datos y presentarla en un dashboard HTML autocontenido. Sin componente MCP.

| Work item | Tipo | Depende de | Estado |
|-----------|------|-----------|--------|
| G1 — Gaps de datos (costos, coherence, context quality, verdict) | feature | — | **done** |
| G2 — Store SQLite central + `telemetry ingest` (schema v1, upsert, derive) | feature | G1 | **done** |
| G3 — `telemetry aggregate` cross-proyecto (reuso `compute_aggregate`) | feature | G2 | **done** |
| G4 — `telemetry dashboard` HTML autocontenido (patrón `export_html`) | feature | G2+G3 | **done** |
| G5 — Research packet cross-proyecto + agentes self-research (sin MCP) | feature | G3 | **done** |
| G6 — Docs (README control plane), tests y CI | docs | G1-G5 | **done** |

**Criterios de salida:**
- Ingest idempotente y reconstruible (delete + ingest == mismo estado) ✅
- Aggregate cross-proyecto con sample = suma de ciclos de todos los proyectos ✅
- Dashboard HTML sin URLs externas, determinista, abrible vía `file://` ✅
- Gaps cerrados evidenciables con `sddk telemetry status` ✅
- 0 regresiones (`cargo test` + `act -j required`) ✅ (44 CLI + engine tests)
- Requisitos PRD RF-016, RF-017 y RNF-007 cubiertos ✅

---

## Milestone RS-2026-08 — Separación de responsabilidades y cero intrusión (ADR-0011)

**Estado:** IMPLEMENTADO (2026-08-07) — ADR-0011 aceptado; R1-R7 completos
**Objetivo:** separar repo de desarrollo / bundle runtime / workspace de uso, y garantizar que el framework no escribe nada dentro de los repos git de los proyectos (todo en XDG).

| Work item | Tipo | Depende de | Estado |
|-----------|------|-----------|--------|
| R1 — Eliminar `plant_workflow_manifest` (adopt no crea ficheros en el repo) | feature | — | **done** |
| R2 — Artefactos de ciclo a XDG (`cycle-artifacts/{cycle_id}/`) + prompts/skills actualizados | feature | R1 | **done** |
| R3 — `generate docs/inventory` → XDG por defecto con `--in-repo` explícito | feature | R1 | **done** |
| R4 — `lint` lee manifest embebido/bundle, no exige workflow.yaml en repo | feature | R1 | **done** |
| R5 — Bundle runtime multi-versión `$SDDK_DATA_DIR/framework/<v>/` + `dev use` (modelo asdf) | feature | R2 | **done** |
| R6 — `dev link` → `current` del bundle + migración (receipts duplicados, `sddk/`, re-link) | ops | R5 | **done** |
| R7 — Dogfooding en clon de trabajo separado (nunca en repo de desarrollo) | ops | R5 | **done** |
| R8 — Resolución de versión por proyecto: `.sddk-versions` → `current` → `path:` (asdf) | feature | R5 | **done** |
| R9 — Multiplataforma: crate `dirs`, fallback macOS/Windows en `resolve_xdg_paths` | feature | R5 | **done** |

**Criterios de salida:**
- `git status` de un proyecto adoptado idéntico antes/después de un ciclo completo ✅ (test `adopt_apply_is_non_intrusive`, `cycle_start_falls_back`)
- Ningún comando `sddk` crea ficheros bajo el working tree del proyecto (salvo `.sddk-versions` declarado por el desarrollador) ✅
- Symlinks de opencode/zcode apuntan bajo `$SDDK_DATA_DIR/framework/current/` ✅ (v1.3.0 instalado y linkeado)
- Múltiples versiones de bundle conviviendo; `dev use` cambia la activa sin tocar el editor ✅ (test `dev_use_switches_bundle_version_and_path`)
- Un solo receipt por workspace; control plane ingiere identidades únicas ✅ (receipts duplicados eliminados)
- `resolve_xdg_paths` resuelve sin `HOME` (Windows) vía `dirs`; `cargo test` verde en linux + darwin ✅ (test `falls_back_to_platform_dirs_without_home`)
- 0 regresiones (`cargo test` + `act -j required`) ✅

---

## Milestone UAT-2026-08-v3 — Human-Governed AI Quality Control Plane (post-v1.6.1)

**Estado:** PLANIFICADO (2026-08-10) — ADR-014 propuesto; plan `docs/uat/PLAN-uat-v3-quality-control-plane.md` aprobado por el usuario
**Objetivo:** reorientar el UAT de "framework UAT con automatizaciones" a una plataforma de Human-Governed AI Quality / TestOps: executor/evidence/oracles/review separados, PASSED != ACCEPTED, review risk-based con sampling, Human Review Queue, disagreement dataset, event log inmutable y exploratory missions con Fara CUA.

| Work item | Tipo | Depende de | Estado |
|-----------|------|-----------|--------|
| F0 — ADR-014 + diseño schema v3 | docs | — | **planned** (ADR escrito, status proposed) |
| F1 — Domain: schema v3 + migrador v2→v3 | feature | F0 | pending |
| F2 — Gateway: PlaywrightExecutor + EvidenceCollector | feature | F1 | pending |
| F3 — Oracles deterministas (exit_code/http/text/json_schema/dom/geometry/a11y/visual_diff) | feature | F2 | pending |
| F4 — CLI: `uat assess` + `uat run --executor playwright` | feature | F2+F3 | pending |
| F5 — Testability agent + CLI `uat testability` (REQ-RF-021) | feature | F1 | pending |
| F6 — Review policy engine + sampling + CLI `uat review` (REQ-RF-022) | feature | F3+F5 | pending |
| F7 — Disagreement dataset + ValidationSession + event log (REQ-RF-023) | feature | F6 | pending |
| F8 — ComputerUseExecutor (Fara CUA) + exploratory `uat mission` | feature | F2+F7 | pending |
| F9 — Dashboard: review-queue + evidence-viewer + report ampliado | feature | F6+F7 | pending |
| F10 — Workflow gates + REQ/ADR updates + migración dogfood | feature | F7+F9 | pending |
| F11 — Release v1.7.0 + dogfood del ciclo completo | ops | F10 | pending |
| F12 — Guided Runner: UAT Form DSL + renderer determinista (ADR-015, REQ-RF-024/025) | feature | F1+F9 | pending |
| F13 — Runner UX (inbox/wizard/blind checks/ratings/diagnostics/checkpoints) + Designer/Reviewer + sign-off inmutable (REQ-RF-026/027/028) | feature | F12+F6+F7 | pending |
| F14 — UX Form Agent + Form Quality Agent + Test Discovery Agent | feature | F5+F8+F12 | pending |

**Criterios de salida:**
- `automation.status` eliminado del schema canónico; migrador v2→v3 automático; renderer acepta v1/v2/v3
- `uat run --executor playwright|cli|script|computer_use` produce EvidenceBundle content-addressable
- Oracles deterministas evaluables por CLI sin IA (`uat assess`)
- `uat testability` recomienda executor/oracles por scenario
- Review policy risk_based + sampling funciona; `uat review` muestra queue con evidencia
- PASSED != ACCEPTED en el dominio; gate exige acceptance (REQ-RF-023)
- Human Review Queue renderiza en el dashboard; disagreement dataset capturado
- Event log reconstruible ("¿por qué se aceptó este release?")
- **Guided UAT Runner**: agentes generan spec declarativa validada, renderer determinista produce wizard (REQ-RF-024); Form DSL con vocabulario cerrado (REQ-RF-025); blind checks + ratings anti-Next-Next-Next (REQ-RF-026); checkpoints + AI diagnostics en FAIL + Actual Result de dominio (REQ-RF-027); tres modos Designer/Runner/Reviewer + sign-off inmutable (REQ-RF-028)
- 250+ tests workspace verde, clippy -D warnings, lint 0/0, 4 suites shell verdes
- Dogfooding: UAT v3 validando una release real del framework

**REQs nuevos:** [[REQ-RF-021]] (testability), [[REQ-RF-022]] (sampling + disagreement), [[REQ-RF-023]] (PASSED != ACCEPTED), [[REQ-RF-024]] (Guided Runner declarativo), [[REQ-RF-025]] (Form DSL), [[REQ-RF-026]] (blind checks + ratings), [[REQ-RF-027]] (checkpoints + diagnostics), [[REQ-RF-028]] (3 modos + sign-off) — propuestos en el vault.

