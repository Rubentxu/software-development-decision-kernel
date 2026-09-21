# ADR-0003 — SQLite para ledger, estado operativo e índice local

**Estado:** aceptada
**Fecha:** 2026-08-03

## Contexto

El workflow necesita transacciones, locks, idempotencia, recuperación, consultas rápidas y soporte para múltiples invocaciones cortas del CLI.

## Decisión

Usar SQLite por proyecto para:

- Ledger de eventos.
- Estado de ciclos y fases.
- Locks y leases.
- Operaciones externas y reconciliación.
- Aprobaciones.
- Índice del vault.
- Búsqueda FTS5.
- Offsets de proyectores.

Activar WAL, foreign keys y migraciones versionadas.

## Ledger

Cada evento incluirá un enlace hash al evento anterior para detectar manipulaciones o pérdida de secuencia.

## Consecuencias positivas

- Distribución sencilla.
- Buen soporte transaccional.
- Excelente compatibilidad local.
- Sin necesidad de daemon obligatorio.

## Consecuencias negativas

- Los recorridos complejos de grafos requieren CTE o carga en `petgraph`.
- Un único escritor concurrente.

## Alternativas rechazadas

- Ficheros JSON como única persistencia operativa.
- LadybugDB como ledger principal.
- Base de datos cliente-servidor obligatoria.
