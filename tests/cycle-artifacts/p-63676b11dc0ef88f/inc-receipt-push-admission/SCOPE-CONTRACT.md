# SCOPE-CONTRACT — INC-AIWS1-RECEIPT-PUSH-BLOCK: ciclo-artifacts documentales en el push admission

Cycle: `p-63676b11dc0ef88f/inc-receipt-push-admission`
Predecessor: INC-AIWS1-RECEIPT-PUSH-BLOCK (2026-09-19), tras el release v1.169.91.
Decisión del operador: corregir la regla de clasificación del hook, no encadenar releases, no bypass, no mover artifacts.

## Goal

Un push a `main` sin bump puede contener, además del allowlist documental
actual (`docs/**`, `.sddk/followups/**`), únicamente documentos de ciclo
reconocidos bajo `tests/cycle-artifacts/`:

    tests/cycle-artifacts/p-<project-id>/<cycle-id>/{SCOPE-CONTRACT.md,DISCOVERY.md,RECEIPT.md}

Nada más del árbol `tests/cycle-artifacts/**` queda admitido: logs,
salidas brutas, fixtures y volcados siguen fuera.

## Falsables (mínimo contractual del UAT)

1. ACCEPT: push cuyo rango contiene solo RECEIPT.md de ciclo (+ followup admitido).
2. REJECT: receipt de ciclo mezclado con cambios en crates/, scripts/, githooks/ sin bump.
3. REJECT: rename de archivo no permitido hacia una ruta permitida (sin bump) sigue ocultando el origen — el hook debe seguir rechazándolo.
4. REJECT: receipt con contenido que parezca un secreto (patrón de token alto-entropía / asignación `api_key=`, `AKIA…`, `ghp_…`) se bloquea antes de publicarse. Permitir la ruta no certifica el contenido.
5. REJECT: ruta de ciclo MAL-FORMADA (p.ej. `tests/cycle-artifacts/notas.md`, o un nombre de archivo no reconocido como SCOPE-CONTRACT/DISCOVERY/RECEIPT.md) sigue rechazada.

## Reconciliación con A5-1

Este ciclo NO reinterpreta A5-1 retroactivamente: amplia la cláusula (B)
con una tercera clase de ruta reconocida y mantiene intactos (A), el
rechazo de rangos vacíos (count>0), la no-autoridad del subject y el
chequeo de renombrados (usa `git diff --name-only --no-renames`, que
mantiene ambas rutas de un rename).

## Fuera de alcance

- Cualquier otro árbol (p.ej. `tests/fixtures/**`).
- Patrón de secreto exhaustivo: bloqueo heurístico de alta confianza
  (prefijos conocidos + asignación de claves), no un escáner completo.
- Modificar el flujo de release o el admission semántico de versión.
