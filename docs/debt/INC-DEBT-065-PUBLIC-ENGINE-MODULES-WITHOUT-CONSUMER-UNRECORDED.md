---
id: INC-DEBT-065
title: "24 modulos publicos de sddk-engine no los consume nada, ni de producto ni de pruebas, y no hay ningun registro que distinga cual de ellos es superficie para adopters y cual es trabajo a medio hacer"
status: open
severity: medium
priority: P2
fingerprint: "public_engine_modules_without_consumer_unrecorded"
fingerprint_aliases: []
cluster_id: CL-SPECULATIVE-GENERALITY
created: 2026-10-03
created_by: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
owner: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
detected_at: 2026-10-03
detected_in_session: session-69o
component: sddk-engine
surface: crates/sddk-engine/src/lib.rs (superficie publica)
related: [INC-DEBT-064, INC-DEBT-048, INC-AUDIT-S14-TEST-PORTS-UNCONSUMED]
references:
  - docs/roadmap/receipts/c3m1-knowledge-merkle-tree/SCOPE-CONTRACT.md
  - docs/roadmap/receipts/c3m1-knowledge-merkle-tree/medir-modulos-consumidor.py
  - crates/sddk-engine/src/lib.rs
  - crates/sddk-engine/src/reactive_verify.rs
---

## Qué es

De los **131** módulos declarados en `crates/sddk-engine/src/lib.rs`, **24 no los
consume ningún fichero de producto ni ningún fichero de pruebas**. Están
publicados —cualquier crate externo puede llamarlos— y nada dentro de este repo
lo hace.

| Categoría | Módulos |
|---|---|
| Con consumidor de producto | **97** |
| **Consumidos sólo por pruebas** (su código corre, ningún comando lo llama) | **10** |
| **Sin consumidor, ni de producto ni de pruebas** | **24** |

Denominador explícito porque es la parte del hallazgo: **131 declarados, 131
medidos, 0 sin clasificar**. Un instrumento anterior daba el mismo 24 pero
midiendo sólo 108, y el detalle está abajo porque explica por qué el
denominador se escribe.

Los 24, medidos:

```
active_graph_view              cas_object_store
converge_verification          decision_plane_gate
engineering_assurance_resolvers evidence_backed_promotion
experience_episodes            ga_publish
gate_evaluator                 human_resume_view
incident_pack                  lab_promotion
pack_agnosticity               production_hardening
projector_registry             reactive_verify
receipt_writers                secretary_l2_replan
security_upgrade_rollback      state_class_lint
structured_work                supply_chain_artifacts
uat_pack                       vault_boundary
```

**Ninguno es un módulo directorio.** Los 23 que son directorio con `mod.rs`
(`event_bus`, `architecture_receipt`, `context_compiler`, `verify_kernel`,
`observation`, `tasks`, `intelligence_loop`, …) están todos consumidos. La
superficie grande y estructural del engine está conectada; lo que no lo está son
24 módulos planos.

## El daño concreto

**No es que 24 módulos estén rotos.** Es que **nada sabe cuáles de los 24 son
superficie pública legítima para un adopter externo y cuáles son trabajo a medio
hacer que nadie está mirando**, y el coste de no saberlo ya se ha pagado una vez:

`reactive_verify` está en la lista, y no es un módulo cualquiera — es un
**contrato escrito sin implementación detrás**. `HostEvent`, la entrada de su
pipeline, **no lo produce nadie en todo el repo ni en ningún lenguaje**
(medido en el STOP 1 del ciclo `c3m1-knowledge-merkle-tree`, §3bis de su
`SCOPE-CONTRACT.md`: seis de los siete símbolos de la cadena no aparecen en un
solo fichero fuera de su propio módulo). El KMT que el roadmap quiere construir
encima no se construyó **precisamente por esto**, y sin este registro la
conclusión habría sido «el árbol está a medio hacer» en vez de «nadie alimenta
la entrada».

**Un módulo público sin consumidor y sin deuda registrada es trabajo a medio
hacer que nadie está mirando**, que es la misma forma que INC-DEBT-064 —código
completo, sin conectar, sin nadie vigilando— y la razón por la que este registro
es de inventario y no de un fichero suelto.

## Lo que esta medición NO dice

**«Sin consumidor» no es «código muerto».** Y hay un caso dentro de la propia
lista que lo demuestra, que por eso va escrito:

`gate_evaluator` no lo consume nada, y el comando `sddk cycle evaluate-gate`
**existe y funciona** —lo he ejecutado en esta misma sesión—. No es el mismo
código: el módulo evalúa dos gates declarados en el contrato de cycle-7b
(`debt-severity-assigned` y `debt-priority-assigned`) sobre `DebtReport`,
mientras el comando resuelve por `GateEvaluationInput` en otro camino.
**Dos cosas que comparten nombre sin relación.** Es el mismo patrón que llevó
`KMT` a ADR-0154, y la razón por la que un nombre compartido no es una
autoridad compartida.

Por eso **la severidad es `medium` y no `high`, y esa elección es deliberada**:
24 módulos sin consumidor no degradan por sí solos ninguna funcionalidad, y
declararlos `high` sería inflar el inventario con una cifra que no se sostiene.
El defecto registrado es **la ausencia del registro**, no los 24 módulos. La
excepción es `reactive_verify`, que sí es de otra gravedad, y **no se infla aquí
para no contaminar la medición**: tiene su propia vía abierta y su propio
bloqueo.

## Los 10 consumidos sólo por pruebas, registrados aparte

Porque reportarlos igual que «nadie los mira» sería el mismo error de medir mal:
**su código corre** y funciona, lo que no existe es un comando que lo alcance.

```
code_intelligence_port_fake   dynamic_expansion      ext_outcome
intelligence_advisory         orchestration_synthesis  revision_substrate
signed_gates                  task_executor          tasks
up_to_date
```

`dynamic_expansion` y `ext_outcome` son los dos que el roadmap nombra como
capacidades certificables (§C3n.2 lista *S4 dynamic expansion*): una capacidad
que se certifica desde pruebas y no desde un comando es una certificación que
nadie puede repetir por el camino que la usa. `intelligence_advisory` y `tasks`
son módulos directorio enteros.

## Cómo se mide, y por qué el instrumento falló cuatro veces

Instrumento: `docs/roadmap/receipts/c3m1-knowledge-merkle-tree/medir-modulos-consumidor.py`.
Se ejecuta desde el repo y deduce la raíz por su propia ubicación, así que el
número es reproducible en cualquier máquina:

```bash
python3 docs/roadmap/receipts/c3m1-knowledge-merkle-tree/medir-modulos-consumidor.py
```

**Se autocomprueba contra cuatro casos conocidos y no publica ningún número si
alguno falla** (sale con código 2). Los cuatro están verificados a mano, no por
el propio script:

| Control | Esperado | Por qué se sabe a mano |
|---|---|---|
| `cycle_pause` | sí consumido | `sddk-cli/src/cycle.rs:1967` llama `engine.cycle_pause(...)` |
| `reactive_verify` | **no** consumido | STOP 1 del ciclo C3m.1, §3bis |
| `event_bus` | sí consumido | `sddk-cli/src/cycle.rs:21`, llama `emit_outcome_event` en `:1443` |
| `architecture_receipt` | sí consumido | `sddk-cli/src/architecture_cmd.rs:28` |

Los dos últimos son **módulos directorio** y son el control del camino nuevo: sin
ellos la corrección siguiente no habría tenido con qué autocomprobarse.

**Cuatro números falsos antes de uno con base, y cada fallo es un modo distinto
de mentir:**

| Versión | Criterio | Dio | Por qué estaba mal |
|---|---|---|---|
| v1 | primer `pub struct` del módulo, en `sddk-engine/src` | 52 | **alcance**: la CLI vive en otro crate, así que todo lo que consume salía «sin entrada» |
| v2 | lo mismo, en todo `crates/` | 41 | **unidad**: medía *mención del símbolo*, no *consumo*; la CLI llama un **método** sin construir el struct de entrada |
| v3 | quién **usa el módulo**, `lib.rs` excluido | 38 | **vía**: la CLI consume por el camino corto (`use sddk_engine::algo`), sin que el nombre del módulo aparezca |
| v3 | + criterio de fichero base `src/<m>.rs` | 38 sobre **108** | **denominador**: se saltaba **23 de los 131** —los que son directorio con `mod.rs`— |
| v4 | v3 + método + reexportados + `mod.rs` + «solo tests» | **24 sobre 131** | — |

**El cuarto es el instructive, y por eso va escrito en la deuda y no solo en el
SCOPE:** entre los 23 que v3 se saltaba estaba **`architecture_receipt`, que es
el consumidor real de la salida de `reactive_verify`**. *Un instrumento que se
salta al consumidor de lo que investiga no puede después declarar «sin
consumidor»: se dice de lo que se mira, no de lo que no se miró.*

**Anotado como lo que es, porque parece una corrección y no lo es:** los 23
resultaron ser 21 con consumidor y 2 sólo desde tests, así que **el número de «sin
consumidor» no cambió — 24 antes, 24 después, los mismos 24**. El número de v3
estaba mal igualmente, y por un motivo que esta vez no movió la respuesta. Eso no
es suerte, y es exactamente por lo que el denominador se declara explícito en
lugar de dejar «131» a secas.

## Defecto encontrado y corregido en el instrumento, no en el producto

Al escribir la exclusión de subárbol para módulos directorio, la primera versión
la aplicaba también al caso plano, donde el directorio padre del fichero base es
`src/` y está en los ancestros de casi todo el engine: habría **excluido el
motor entero** y publicado «sin consumidor» para los 131. Un falso más grave que
el que se corrige. Detectado leyendo lo escrito, antes de ejecutarlo.

## Riesgos de este registro

1. **El reparto depende del criterio de «consumo» elegido**, y el criterio está
   escrito en el instrumento; otro criterio —por ejemplo, contar un `pub use`
   como consumo, que es lo que hace el `lib.rs`— daría otro reparto. El criterio
   está en el fichero precisamente para que sea discutible.
2. **Consumido por pruebas no es sano ni enfermo.** Los 10 son una categoría
   aparte y este registro no afirma nada sobre ellos más allá de eso.
3. **La lista es una fotografía de un SHA**, no una propiedad permanente: un
   módulo puede ganar su consumidor en el siguiente commit. El instrumento se
   re-ejecuta, no se reescribe.
4. **No se propone borrar nada.** Una entrada de inventario no es una lista de
   borrables, y borrar módulos públicos por no tener consumidor interno rompería a
   cualquier adopter externo sin saberlo.

## Por qué es un inventario y no 24 fichas

Porque **es la misma clase que una deuda que ya existe**:
`INC-AUDIT-S14-TEST-PORTS-UNCONSUMED` es «9 traits del SPI de SPEC-043,
implementados dentro del crate pero sin consumidor externo aún». La misma forma,
otra superficie. Abrir 24 fichas habría producido 24 filas que nadie puede
comparar entre sí, y una entrada con la medición como evidencia permite **ver
el conjunto, detectarlo cuando cambia, y agruparlo con lo que ya estaba
registrado**. Lo que **no** hace es fingir que los 24 comparten gravedad:
`reactive_verify` y `gate_evaluator` no son el mismo caso, y la entrada lo dice
en cada punto donde cambia.

## Cierre

Cierra cuando exista un registro —por módulo o por categoría— que distinga
superficie legítima de trabajo inconcluso, **o** cuando los 24 ganen consumidor.
No cierra por antigüedad ni por el paso del tiempo: cierra con un registro o
con un consumidor, y en ese momento la entrada dice algo que ya no es verdad.
