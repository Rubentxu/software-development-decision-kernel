---
id: INC-DEBT-053-CHAIN-AND-DEBT-GATES-VERIFY-NOTHING
title: "verify-chain, backfill-chain y debt report responden sin examinar nada"
severity: high
priority: P1
status: resolved
opened: session-62
resolved: session-62
resolution: Ruta 1 (dejar de fabricar el veredicto, no bajar el gate)
component: sddk-cli
surface: crates/sddk-cli/src/ledger.rs, crates/sddk-cli/src/debt.rs
---

# INC-DEBT-053: `verify-chain`, `backfill-chain` y `debt report` responden sin examinar nada

> **RESUELTA (session-62).** Los tres subcomandos dejaban de fabricar su entrada.
> `verify-chain` y `backfill-chain` resuelven por defecto el conjunto real de
> streams del ledger del proyecto y se niegan a emitir `PASS` sobre cero eventos
> verificados; `debt report` y `debt gates` fallan cerrado nombrando que la
> detección de deuda no está implementada. Ver «Consecuencia aceptada».

## Criterio verificable

Medido sobre el ledger de `p-d55b20670622127c` (172 eventos, todos con
`chain_hash` no nulo), **antes** del cambio:

```
$ sddk ledger verify
event_count: 171

$ sddk ledger verify-chain
stream: project:p-d55b20670622127c
event_count: 0
head_chain_hash: null
status: PASS

$ sddk ledger backfill-chain
stream: project:p-d55b20670622127c
updated: 0
status: SUCCESS
```

`verify` ve 171 eventos. `verify-chain` ve 0 y contesta `PASS`; `backfill-chain`
documenta «Defaults to all streams», backfillea 0 y contesta `SUCCESS`.

## Causa raíz

`crates/sddk-cli/src/ledger.rs` resolvía el stream por defecto como
`format!("project:{}", project_id)`. Ningún evento ha tenido nunca ese nombre.
Recuento sobre los **326** ledgers de la máquina:

| forma de `stream_id` | streams |
|---|---|
| `cycle:<project_id>/<cycle>` | 409 |
| `<project_id>/<algo>` | ~60 |
| `authority-<substream>` | 50 |
| **`project:<id>`** | **0** |

El literal no era un valor por defecto desafortunado: era un identificador que
no había existido nunca en ningún ledger escrito jamás. Seleccionaba cero
eventos, y `verify_chain_integrity` contesta `Ok(())` sobre cero eventos — lo cual
es cierto de cero eventos y falso como afirmación sobre un ledger. El comando
convertía esa verdad menor en un veredicto.

`run_backfill_chain` repetía el mismo literal de forma independiente. La
duplicación es lo que permitió que el mismo defecto apareciera dos veces sin que
ninguna copia delatar a la otra.

## El defecto de `debt` es peor de lo registrado

`INC-DEBT-REPORT-FOREIGN-CYCLE` (owner `sddk-framework`, observado desde
`p-f4d8f28cd78d443e`) dice que `debt report` «resuelve un ciclo activo de otro
proyecto». La causa real es más simple: **no resolvía nada**. El identificador
estaba escrito a mano.

```rust
// crates/sddk-cli/src/debt.rs, antes del cambio
fn cmd_report(output: &PathBuf) -> CommandOutput {
    let report = empty_report_for_cycle("p-52b95ef55999f9de/kernel-cycle-8");
```

`cmd_report` no recibía el entorno, así que no podía conocer el proyecto.
`cmd_gates` no recibía ni siquiera `env`:

```rust
fn cmd_gates(gate_name: &str) -> CommandOutput {
    let report = empty_report_for_cycle("p-52b95ef55999f9de/kernel-cycle-8");
    let outcome = evaluate_named_gate(gate_name, &report);
```

`evaluate_named_gate` reúne los hallazgos que **incumplen** el predicado; un
informe vacío no incumple nada, luego `Passed`. Informe fabricado → siempre
vacío → gate siempre `PASS`.

**Consecuencia no registrada hasta ahora:** los gates `debt-severity-assigned` y
`debt-priority-assigned` que la fase `verify` exige en **todo** proyecto han
sido autorizados por una constante. No eran gates ciegos: eran constantes.

## El defecto estaba además fijado por tests

`test_report_empty_findings` afirmaba `status == 0` y `output.exists()`. La
prueba **requería** que la fabricación funcionase. Un defecto con test no es un
defecto sin cubrir: es un defecto ratificado. Ambas pruebas se reescribieron para
fijar el comportamiento honesto.

## Dónde NO está el defecto

Ni `sddk-storage` ni `sddk-engine` mienten. `Ok(())` sobre cero eventos es
cierto; «ningún hallazgo incumple el predicado» sobre un informe vacío también.
El falsehood está entero en el adaptador `sddk-cli`, que fabrica la entrada.
Cambiar la semántica de esas primitivas habría sido tapar una falsedad del
adaptador rompiendo dos primitivas correctas.

## Dientes: cuatro inyecciones, y dos que no mordieron

1. El stream vacío vuelve a declararse intacto → `un_stream_vacio_no_se_declara_cadena_intacta` falla.
2. El default vuelve al stream inventado → `el_por_defecto_verifica_los_streams_que_existen` falla por la etiqueta.
3. El informe y el `PASS` fabricados vuelven → los dos tests de `debt` fallan.
4. Un stream nombrado vuelve a etiquetarse como el conjunto entero →
   `un_stream_nombrado_se_responde_con_su_nombre` falla.

La inyección 2 **no mordió la primera vez**, y el motivo importa: el test
llamaba directamente a `verify_every_stream`, así que la línea que elegía el
stream por defecto quedaba fuera de su alcance. La prueba pasaba mientras el
defecto seguía presente. Se extrajo `resolve_streams` como decisión única —
también elimina la duplicación que había dejado aparecer el defecto dos veces— y
el test pasó a ejercitar la decisión.

Durante esa corrección el refactor **introdujo una regresión** que el test
atrapó: la vacuidad se juzgaba por el número de nombres del conjunto y no por los
eventos examinados, de modo que un nombre que no resuelve a nada volvía a
declararse `PASS`. El criterio correcto es `event_count == 0`.

La inyección 4 **tampoco podía morder**, y por la razón opuesta a la de la 2: los
dos tests del refactor pasaban por `resolve_streams(None, ..)`, luego ninguno
alcanzaba la ruta del stream explícito. Véase «Regresión de etiqueta».

## Regresión de etiqueta (encontrada al evaluar `ddfd2b51`, no por sus tests)

Al integrar `ddfd2b51` se encontró un defecto que el commit no cubría.
`run_backfill_chain` conservaba la etiqueta que devuelve `resolve_streams`;
`run_verify_chain` la descartaba (`let (_, streams) = …`) y `verify_streams` la
reconstruía con `resolve_streams(None, streams, project_id)`. El efecto:

```
$ sddk ledger verify-chain --stream cycle:p-demo/one
stream: all streams of p-demo      # lo pedido: cycle:p-demo/one
```

El veredicto era correcto —se verificaba el stream pedido— pero la salida
nombraba otra cosa. Es el mismo género de falta que este INC registra, una
escala más pequeña: el informe describía algo distinto de lo examinado, y el
doc-comment del propio struct prometía lo contrario («The stream verified, or the
label describing the set verified when the caller named none»).

RED medido antes de corregir:

```
left:  "all streams of p-demo"
right: "cycle:p-demo/one"
```

El arreglo hace que la etiqueta viaje desde quien la decide hasta quien la
publica: `verify_streams` la recibe en lugar de recalcularla. Solo el resolver
sabe si quien llamó nombró un stream o tomó el default, así que duplicar esa
decisión dentro de la verificación es lo que permite el fallo. Con el cambio,
`run_backfill_chain` y `run_verify_chain` hacen exactamente lo mismo con el
resultado de `resolve_streams`.

**Consecuencia sobre el propio INC-DEBT-053:** sigue `resolved`, pero la
resolución era incompleta en el momento del commit. El aviso «la fase `verify`
deja de poder autorizarse» permanece íntegro y no le afecta.

## Consecuencia aceptada

`debt gates` ya no puede devolver `PASS` mientras la detección de deuda no exista.
**La fase `verify` de todo proyecto deja de poder autorizarse** con esos dos
gates.

Es el precio de dejar de fabricar el veredicto, y es el aviso que el gate ciego
llevaba años reprimiendo. La salida correcta si esa indisponibilidad resulta
inaceptable no es restaurar el `PASS` constante, sino implementar la detección;
cabe en su propio ciclo.

## Ceguera estructural que este arreglo NO corrige

`verify_chain_integrity` sigue contestando `Ok(())` sobre cero eventos, porque
sobre cero eventos es cierto. Lo que se corrigió es el comando que la publicaba
como veredicto. Un store que devuelve la verdad sobre su entrada y un comando que
no convierte una ausencia en un `PASS` es la división correcta; invertirla
—hacer que el store falle— cambiaría la semántica de una primitiva correcta para
compensar a un adaptador que mentía.
