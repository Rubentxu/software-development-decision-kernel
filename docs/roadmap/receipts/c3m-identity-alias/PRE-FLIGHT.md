# C3m-identity-alias — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/identity-alias` (C3m, ROADMAP §C3m)
**Date:** 2026-10-02T10:45:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto v2.5.2)
**HEAD:** `e7864c0b` (`docs(adr): ADR-0152 …`), `HEAD == origin/main`, árbol limpio
**Authority:** ADR-0152 `status: proposed`; SCOPE-CONTRACT en
`docs/roadmap/receipts/c3m-identity-alias/SCOPE-CONTRACT.md`

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | ADR escrito con decisión, costos y criterios falsables | OK — ADR-0152 |
| 2 | SCOPE-CONTRACT con objetivo falsable, no-objetivos y STOP conditions | OK |
| 3 | Superficie de código mapeada y leída, no inferida | OK — se cita línea por línea abajo |
| 4 | Superficie mínima para el lote actual | 1 fichero del dominio, sin I/O |
| 5 | Tests que fijan la propiedad, con falsificador cada uno | declarados abajo |
| 6 | Riesgo de datos | **cero**: este lote no abre ni escribe ningún ledger |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `IdentityError` | `crates/sddk-domain/src/identity.rs:14` | donde va el error de ciclo |
| `ProjectId::new` | `identity.rs:47` | valida el formato; un alias no puede inventar ids |
| `resolve_project_identity` | `identity.rs:364` | resuelve desde remote o seed; **no** toca aliases |
| `resolve_identity_honoring_pin` | `crates/sddk-cli/src/lib.rs:1516` | el sitio **único**; el alias se encadena aquí |
| `plan_adoption` | `crates/sddk-engine/src/adoption.rs:197` | el pin gana sobre la derivación (W2c) |
| `compute_content_hash` | `crates/sddk-domain/src/event_envelope.rs:162` | por qué la migración es imposible |
| trigger append-only | `crates/sddk-storage/tests/cross_ledger_consistency.rs:151` | el invariante que este trabajo no puede romper |

## Lote de este apply: SOLO el dominio

`crates/sddk-domain/src/identity.rs`, sin I/O, sin schema, sin CLI.

**Motivo de acotar:** la lógica de resolución transitiva y la detección de
ciclo son donde está el riesgo, y son puras. Añadir persistencia y CLI en el
mismo lote haría que un fallo de lógica y uno de cableado fueran indistinguibles
al leer el resultado. Los lotes 2 (persistencia) y 3 (CLI) van después, con
tests propios.

## Lote de verificación (scoped, no el perfil completo)

```text
cargo test -p sddk-domain --lib identity
```

Perfil completo sólo en `verify`/release, según AGENTS.md §2.3.

## STOP conditions

1. Si `cargo test -p sddk-domain` falla por algo **fuera** de `identity`, parar:
   el baseline estaba verde y ese fallo no es mío para arreglarlo en este lote.
2. Si el lote toca un fichero fuera de `crates/sddk-domain/src/identity.rs`,
   parar: el lote se broadened solo y eso es un defecto de disciplina, no de
   diseño.
3. Si aparece la necesidad de **escribir** en algún ledger para que la
   resolución funcione, parar. La propiedad es «resolver sin escribir»; un
   diseño que necesita escribir para leer no es este diseño.
4. Si el selfcheck de normalización golden (`test_migrate_project_identity_mirror.py`)
   falla, parar: he tocado el normalizador y ese guard existe por algo.

## Riesgos declarados

| riesgo | mitigación en este lote |
|---|---|
| Que el alias se lea como «reescribir la identidad» | el tipo se llama `ProjectAlias` y su doc dice que **no** emite ids nuevos |
| Detección de ciclo que no termina | el límite de pasos es explícito y el ciclo se nombra en el error, no se trunca en silencio |
| Un `A -> A` que se cuelgue | criterio falsable 1: resuelve a `A` y termina |

## Lo que este pre-flight NO autoriza

- No autoriza publicar una release (sigue bloqueada por la clave del KMS).
- No autoriza tocar el storage real de la máquina: los 15 aliases se aplicarán
  en un lote posterior, con su propio criterio de recuento de filas.
