# PRE-FLIGHT — c3m1-knowledge-merkle-tree

**Cycle:** `p-63676b11dc0ef88f/c3m1-knowledge-merkle-tree`
**Date:** 2026-10-03
**Readiness:** **NOT_READY**
**Razón:** el STOP 1 del SCOPE-CONTRACT es una medición, y dio **negativo**.

---

## Qué se comprobó, y cómo

El STOP 1 pide responder, con evidencia, **quién va a alimentar el KMT** y **de
dónde salen los fingerprints**. Se respondió, y el resultado se escribe aunque
sea negativo —que es lo que el SCOPE exige.

| Comprobación | Método | Resultado |
|---|---|---|
| ¿Existe la estructura? | `grep -rn 'KmtNode\|KmtTree\|affected_units' crates/` | **cero coincidencias** |
| ¿Hay fingerprints de árbol? | `grep -ro 'fingerprint' crates/ --include=*.rs \| wc -l` | 118, **ninguno de árbol**: son idempotency keys de `dynamic_expansion` |
| ¿Quién produce `HostEvent`? | `grep -rn 'HostEvent'` en **todo el repo, todo lenguaje** | **nadie** |
| ¿Quién produce el `WorkspaceChangeSet`? | `grep -rn 'WorkspaceChangeSet' crates/` | **nadie** fuera de su módulo |
| ¿Quién construye el `KmtUnitIndex`? | `grep -rn 'KmtUnitIndex' crates/` | **nadie** fuera de su `fn kmt()` de test |
| ¿Quién llama `run_reactive_verify`? | `grep -rn 'run_reactive_verify' crates/` | **solo sus 4 tests** |
| ¿Lo define alguna spec? | `arch-spec-037:20` | **una línea**, sin estructura |

**Seis de los siete símbolos del pipeline no aparecen en un solo fichero fuera
de `reactive_verify.rs`.**

## Por qué esto bloquea la implementación

Construir `KmtNode`/`KmtTree` sin fuente de fingerprints produciría un árbol
determinista, verificable y **consultado por nadie** — que es exactamente la
forma de INC-DEBT-064: un mecanismo completo que no vigila nada porque no hay
nada que lo pregunte. El SCOPE lo dice en §3 y el STOP condition de §6 ordena
detenerse en `explore`, no seguir construyendo.

Y hay un dato que convierte el hallazgo en deuda y no en simple hueco de
C3m.1: **`reactive_verify` es superficie pública del engine** (`pub mod
reactive_verify;` en `lib.rs:112`) y **no hay ninguna deuda declarada** sobre
él — `grep -rln 'RHB-\|reactive_verify' docs/debt/` → cero. Un módulo
público sin entrada y sin deuda registrada es trabajo a medio hacer que nadie
está mirando, y eso pertenece al inventario de riesgos, no a un SCOPE de
construcción.

## Qué haría falta para que esto fuera READY

1. **Una superficie de producto que produzca `HostEvent`** — un adaptador que
   normalice un cambio del host. Hoy no existe, y qué debería hacerlo es una
   decisión de producto que no se deduce leyendo código.
2. **Una fuente declarada de fingerprints** — de dónde sale el contenido que
   hashea cada unidad.

Sin 1, la entrada no existe. Sin 2, el KMT no tiene nada que asignar a un nodo.

## Lo que este PRE-FLIGHT no hace

No declara el ciclo cerrado ni bloqueado. El ciclo queda **abierto en
`explore`**, con la medición escrita, que es el estado honesto: se midió, se
encontró un hueco que bloquea la construcción, y la decisión de qué lo llena es
del operador. La deuda del módulo sin entrada es un hallazgo de esta medición y
se propone como tal, **sin abrirla aquí**, porque declararla es una decisión
sobre el inventario de riesgos.
