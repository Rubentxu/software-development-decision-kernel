# SCOPE-CONTRACT — a8-s1-conformance-workbooks (AC12, arch-spec-040)

## Goal

UAT del modelo de Conformance Workbooks + time-travel (AC12) sobre
superficies existentes: `ConformanceVector` (7 dimensiones canónicas,
REQ-AC4-023/024), `Revision`/`RefStore` del revision substrate.

| Req | Test | Estado |
|---|---|---|
| AC-040-001/004 workbook 7 dimensiones, sin score universal | t_ac040_001_004 | ✅ |
| AC-040-002 cada fila enlaza base de evidencia revisionada | t_ac040_002 | ✅ |
| AC-040-005 time-travel: diff determinista entre revisiones | t_ac040_005 | ✅ |
| AC-040-005 CAS append-only en RefStore (Stale no mueve ref) | t_ac040_005 | ✅ |

## Notas

- El workbook es proyección read-only (AC-040-003): el test construye
  el tipo sin path de escritura; sin API de score.
- Semántica descubierta: `RefStore::cas` devuelve `RefUpdate::Stale`
  (no Err) cuando el expected no matchea — el test pinea esa
  semántica y que el ref NO se mueve.
- Oid = contenido del payload (provenance no participa): dos
  revisiones con payloads idénticos comparten oid; el test usa
  payloads distintos para diferenciar bases.
- AC13 (counterfactual) y AC14 (proof-carrying/ratchets) → A8-S2/S3.

## Evidence

- `cargo test -p sddk-engine --test a8_s1_conformance_workbooks` → 4 PASS / 0 FAIL.
- clippy -D warnings = 0. Test-only; sin cambios en src.
