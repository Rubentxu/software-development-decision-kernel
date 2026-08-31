---
status: accepted
date: 2026-08-15
deciders: [orchestrator, rubentxu]
linked-cycles: [p-52b95ef55999f9de/gate-receipt-seq-race]
---

# ADR-0002 — Asignación atómica de seq en gate_receipts

## Contexto

El flujo anterior para persistir un `GateReceipt` requería **dos llamadas** desde el motor:

1. `Storage::allocate_gate_receipt_seq(gate, plan_hash)` → `seq` dentro de una transacción `IMMEDIATE`
2. `Storage::insert_gate_receipt(GateReceiptInput { seq, receipt_id: format!("gate-{g}-{h[7..23]}-{seq}") })`

Entre (1) y (2) otro thread o proceso podía allocatear el mismo `seq`, causando:
- Violación del constraint `UNIQUE(gate, plan_hash, seq)` si ambos completaban el `INSERT`
- O receipt_ids con seq duplicados si el segundo `allocate` retornaba el mismo valor

Este race condition afectaba a cualquier evaluación concurrente sobre el mismo `(gate, plan_hash)` group.

## Decisión

Un solo método `Storage::insert_gate_receipt_next_seq` que:

1. Abre una transacción `IMMEDIATE`
2. Computa `SELECT COALESCE(MAX(seq)+1, 1) WHERE gate=? AND plan_hash=?`
3. Construye `receipt_id = format!("gate-{gate}-{plan_hash[7..23]}-{seq}")` **dentro de la misma transacción**
4. Ejecuta `INSERT INTO gate_receipts(...)`
5. Commit

El lock de escritura de SQLite (`RESERVED` vía `IMMEDIATE`) serializa las asignaciones concurrentes. El constraint `UNIQUE(gate, plan_hash, seq)` actúa como belt-and-braces.

### API afectada

| Método | Cambio |
|--------|--------|
| `Storage::allocate_gate_receipt_seq` | **ELIMINADO** (ponytail: INC-DEBT-007) |
| `Storage::insert_gate_receipt_next_seq` | **NUEVO** — atomic seq allocation + rid build + insert |
| `Storage::insert_gate_receipt` | **MANTENIDO** — para tests de storage y uso interno |

## Consecuencias

Positivas:
- `seq` y `receipt_id` son producidos por un solo punto; no puede haber split entre allocate e insert
- La transacción `IMMEDIATE` garantiza que SQLite serialice los competidores
- El `receipt_id` mantiene el formato byte-compatible con v1.9.17: `gate-{gate}-{16hex}-{seq}`

Negativas / costes:
- El método内幕 una transacción de escritura completa; overhead marginal vs. dos llamadas
- `Storage::insert_gate_receipt` existente queda huérfana para uso directo (pero la usa 1 test de storage como seed del bootstrap)

## Alternativas consideradas

- **Caller suministra `seq`** — el race sobrevive si dos callers llaman con el mismo valor
- **Closure allocator** — divide la sección crítica entre compute y insert
- **Advisory lock row** — overkill; SQLite ya tiene write lock con `IMMEDIATE`

## Decisiones relacionadas

- ADR-0001: validación E2E sandbox — usa este fix en los tests de concurrencia
- `gate-receipt-seq-race` (v1.9.18): este fix
