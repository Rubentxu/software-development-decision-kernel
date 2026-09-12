# Handoff — 2026-09-12 — Cierre roadmap sweep m9–m18 + Release v1.168.60

## Goal

Cerrar el barrido de specs del vault (`p-63676b11dc0ef88f/sddk-framework`) y publicar
el release canónico real que sincronizara binario local + bundle con `main`.
Modo auto continuado a petición del usuario ("continúa revisando lo que podamos
tener pendiente del roadmap usando sddk en modo auto").

## Estado final (verificado)

- **HEAD = origin/main = tag v1.168.60** = `e411a01`
- GH Release **v1.168.60** publicado (`2026-09-12T22:48:09Z`), assets completos
  (binario, bundle tarball, BUNDLE.toml v2, CHECKSUMS, sbom.json CycloneDX)
- Install desde URL real OK, `sddk dev doctor` → `all_present: true`,
  prune `--keep 1` OK
- Estado local: **binary 1.168.60 · bundle 1.168.60 · current 1.168.60**
  (antes de hoy el binario local estaba en 1.168.47; todos los bumps m13–m58
  eran ceremoniales sin release real)

## Trabajo realizado en la sesión (rondas m13–m18 + release)

| Ronda | Versión | Commit | Contenido |
|---|---|---|---|
| m13 | 1.168.53 | `11f6a5b` | INC-DEBT-020; 25 specs REQ-DC* (pln4-projections) → implemented; REQ-DW-RUNTIME-002 Graph-Revision-Binding → implemented, Engine-Deferral → superseded |
| m14 | 1.168.54 | `058dcf3` | 7 specs flip (workflow-compiler, validator 7 gates, budgets-consume, context-capsule-inline-validator, expansion-permission-split, graph-store-port, vault Writer XDG fail-closed); 6 duplicados legacy → superseded |
| m15 | 1.168.55 | `336cc25` | 24 specs `engine/` → implemented (pack/GA 13 módulos, substrates, cockpit/decision, InstallPerfection3) |
| m16 | 1.168.56 | `35c2c7e` | `preserve_repair_receipts` en cycle_supersede.rs (anotación `waiver: superseded` idempotente, write atómico) + `preserved_receipts_count` en supersede-receipt.json |
| m17 | 1.168.57 | `c125e64` | Auditoría de incidencias (INC-2026-08-27-cycle-44 → closed retroactive, INC-005720 accepted_risk, INC-028 nota obsoleta); uat RF-021/RF-023 → implemented |
| m18 | 1.168.58 | `7225d5b`+`a53ca70` | Learning loop RF-022: `sddk uat disagreement` (dataset append-only XDG), `UatDisagreementMetrics` + aggregator + validator en domain, `uat report` embebe `disagreement_metrics`. E2E verificado |
| Release | **1.168.60** | `e298646`→`e411a01` | help-uat snapshot regenerado; Cargo.lock sincronizado; `release.sh` 14/14 OK |

## Estado del roadmap tras el sweep

- **Todas las specs del vault con contrato implementable → `implemented`**
  (flip solo con evidencia verificada: tests green ejecutados en sesión o
  lectura de código fuente)
- **Cero incidencias abiertas sin justificación**
- Solo quedan SPECs marco doc-only (SPEC-028 reactive-graph, SPEC-023/037,
  SPEC-PAUSE-001, secretary-runtime Stage 0): diseño futuro, no deuda
- Archive manifests: `.sddk/cycles/p-63676b11dc0ef88f-roadmap-sweep-mN-*/archive-manifest.md`
  (m9…m18)

## Incidencias/problemas resueltos durante el release

1. **`uat_help_matches_snapshot` fallaba** en `cargo test --workspace`: el snapshot
   `crates/sddk-cli/tests/fixtures/cli/help-uat.txt` no incluía `disagreement`.
   Fix: regenerar con `sddk uat --help` (commit `e298646`).
2. **Pre-push hook exige commit ceremonial en CADA push a main**: subject
   `chore(release): bump version` o bump de `[workspace.package] version`.
   Committeo Cargo.lock sin bump → rechazo. Regla práctica: todo push a main
   lleva su `chore(release): bump version`.
3. **`cargo build` regenera Cargo.lock dirty** (sincroniza versiones del
   workspace). Hay que committearlo (con bump ceremonial) ANTES de lanzar
   `release.sh`, que rechaza tree dirty en preflight.
4. **Timeout de bg task 10 min mata release.sh** (~7-10 min solo el gate de
   workspace). Usar timeout ≥ 2h para tasks de release.

## Gotchas / lecciones

- Lecciones m13–m18 ya archivadas en sus respectivos archive manifests y
  memoria de proyecto (incl. el incidente `git checkout crates/sddk-domain/src/uat.rs`
  en m18 y su reaplicación vía script Python).
- El snapshot de help de la CLI es un contrato de compatibilidad: cualquier
  subcomando nuevo requiere regenerar su fixture (`fixtures/cli/help-*.txt`).

## Next steps

- Nada pendiente del roadmap. Próximo trabajo natural (cuando toque):
  - Implementar los SPECs marco doc-only si se decide priorizarlos
    (SPEC-028 reactive-graph es el más sustancioso)
  - RF-022: alimentar el dataset de disagreements con sesiones UAT reales
- El árbol está limpio y sincronizado; cualquier ciclo nuevo arranca con el
  pre-flight normal (`sddk adopt status`, `git fetch && pull`).
