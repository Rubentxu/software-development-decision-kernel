---
id: INC-DEBT-067
title: "La vista del operador dice 'Cycle completed' y 'Nada por ahora' para TODOS los ciclos, incluidos los abiertos y los que esperan aprobacion humana, porque las dos frases son constantes y no derivaciones"
status: resolved
severity: high
priority: P1
resolved: 2026-10-04
resolved_in_session: session-78
resolved_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
fingerprint: "cycle_narrative_operator_view_asserts_completed_and_no_action_regardless_of_state"
fingerprint_aliases: []
cluster_id: CL-VERIFICATION
created: 2026-10-04
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-04
detected_in_session: session-68
component: sddk-cli (run_cycle_narrative) / sddk-engine (cycle_narrative)
surface: crates/sddk-cli/src/cycle.rs:1618 y crates/sddk-engine/src/cycle_narrative.rs:321,492,500
related: [INC-DEBT-065, INC-DEBT-066]
references:
  - crates/sddk-cli/src/cycle.rs
  - crates/sddk-engine/src/cycle_narrative.rs
---

## Qué es

`sddk cycle narrative` se describe a sí mismo como la **operator view** (su propio
`--help`: «Render a deterministic markdown narrative for a cycle (operator view)»,
`--audience maintainer` por defecto). Es, por tanto, **la forma en que una persona lee
el estado del ciclo**. Y sus dos afirmaciones centrales **no derivan de nada**:

1. **`"Cycle completed."`** es el valor por defecto de `what_was_done` cuando el
   llamante no pasa `--what-was-done` (`crates/sddk-cli/src/cycle.rs:1618`).
   **No consulta el `status` del ciclo.** Es una cadena literal.
2. **`"Nada por ahora."`** es lo que el pie `Necesito de ti` imprime cuando
   `n.human_action_required` es `None` (`cycle_narrative.rs:500`) — y ese campo
   **no tiene productor en todo el workspace**: se declara en `:286`, se inicializa a
   `None` en el constructor (`:321`) y **no se le asigna ningún `Some(..)` en ninguna
   parte**. Las únicas cuatro apariciones del símbolo en el repo son la declaración del
   campo, ese `None` de construcción, la rama que lo renderiza y un fixture de test
   (`:565`). **El pie no puede decir otra cosa que "Nada por ahora", ni por construcción.**

## OBSERVED — el falsificador, y por qué no es un default inofensivo

Si el defecto fuera «el estado no se propaga», bastaría un ciclo. Se compararon **tres,
elegidos para que la respuesta correcta difiera en los tres ejes**:

| Ciclo | `status` | `phase` | `runtime_state` | Lo que dice el narrative |
|---|---|---|---|---|
| `cycle-45-build-remediate-archive` | **CLOSED** | archive | — | `Cycle completed.` / `Nada por ahora.` |
| `cycle-46-install-coherence` | **OPEN** | build | — | `Cycle completed.` / `Nada por ahora.` |
| `c3n-production-boundary-certification` | **OPEN** | design | **approval-waiting** | `Cycle completed.` / `Nada por ahora.` |

**Las tres filas renderizan idénticas.** Un `CLOSED` y dos `OPEN` en fases distintas
producen byte a byte la misma afirmación sobre su estado. Eso descarta la lectura
cómoda —«el default es aceptable porque el caller puede sobrescribirlo»—: el problema
no es el default, es que **el valor por defecto es la respuesta para todos los casos**,
incluido el que más la necesita.

## Consecuencia viva en este repo, no teórica

El ciclo `c3n-production-boundary-certification` lleva **desde `2026-10-03` en
`approval-waiting`**, con una solicitud de aprobación registrada
(`surface.cycle_state#cycle_supersede`, `request_hash sha256:c128d51c…`) que **solo un
operador humano puede conceder** —-y que este agente no se concedería a sí mismo,
porque aprobar una superficie gobernada desde quien pide la mutación anula el gate—.
Mientras tanto, **la vista del operador afirma que el ciclo está completado y que no
hace falta nada de él.** Quien se guíe por ella no concede la aprobación, y el ciclo no
avanza: el bloqueo es invisible justo en la vista diseñada para hacerlo visible.

## Por qué `high` y no `critical`

No hay pérdida de datos ni corrupción: **`sddk cycle status` —la vista de máquina— es
correcta** (`status: OPEN`, `phase: design`, `runtime_state: approval-waiting`, lease con
su `fencing_token`), y la frente a ella es la que miente. El daño está acotado a la
proyección legible por una persona. Se sube a `critical` si la proyección llega a
escribir estado —es decir, si algún día `narrative` alimenta un artefacto o una
transición en vez de solo imprimir—.

## Misma clase que ya está registrada

Es la forma de **INC-DEBT-066** (un `PASS` que certifica algo que la fila no exige) y la
de **INC-DEBT-065** (`reactive_verify`, un contrato escrito sin nadie detrás): **una
superficie que existe, se renderiza y no tiene productor.** Aquí la diferencia es que la
superficie *sí* tiene productor —el render— y lo que no tiene productor es el **dato**,
y el render **presenta la ausencia de dato como una afirmación positiva**.

## Cómo refutarla (o por qué es `open` y no `resolved`)

El defecto se puede comprobar con un ciclo `OPEN` en `approval-waiting` cuyo narrative
diga algo distinto de `Cycle completed.` y cuyo pie `Necesito de ti` nombre la
aprobación pendiente. **Hoy ninguno de los dos ocurre**, y no hay test que lo exija:
`grep -rn human_action_required` sobre `crates/` devuelve 4 líneas y ninguna es una
asignación.

Cierre = (1) `what_was_done` **derivado del `status`/`phase`/`runtime_state`** leídos
del ciclo, no una constante; (2) `human_action_required` **poblado** desde el estado real
—aprobaciones pendientes, `blockers`, `runtime_state != proceeding`— con el
`None` **visto** como «no se pudo determinar» y no como «no hace falta nada»; (3) un test que
afirme que **un ciclo `OPEN` en `approval-waiting` no renderiza `Cycle completed.`**,
y su falsificación por mutación de los tres estados.

---

## RESOLVED (session-78) — y la deuda era PEOR de lo que este documento dice

Lo de arriba queda como se detectó. Esto es lo que se midió al arreglarla.

### Lo que este documento NO recogía

El documento afirma que el defecto son «dos constantes». Es cierto, y es **lo
menor**. Al medir el camino completo antes de tocar nada:

1. **`run_cycle_narrative` no abría el store.** Su parámetro `environment`
   estaba literalmente **sin usar** (verificado: cero referencias en el cuerpo de
   la función). No consultaba el ciclo, no consultaba la lease, no consultaba el
   resumen de runtime. No era una derivación ausente: era **la ausencia de
   lectura**.
2. **Consecuencia medida:** `sddk cycle narrative --cycle no-existe-este-ciclo-xyz`
   imprimía `Cycle completed.` y `Nada por ahora.` con **exit 0**. Sobre un
   objeto que no existe. Una vista que fabrica una afirmación de completitud
   sobre un ciclo inexistente no es una vista débil — es una vista **falsa**, y
   falla abierta **en la dirección que esconde trabajo**, que es la peor.

El documento ya decía que la proyección era lo dañado y que `cycle status` era
correcto. Eso sigue siendo cierto y por eso la severidad no sube: lo que cambia
es que la proyección **no solo era incomplete, sino que affirmaba lo contrario
de lo que no sabía**.

### Qué se hizo

- **El productor vive en el engine, no en la CLI.** `derive_claims()` en
  `crates/sddk-engine/src/cycle_narrative.rs` recibe `NarrativeFacts` (status,
  phase, runtime_state, approval_waiting_on, lease, now_ms) y devuelve
  `NarrativeClaims`. La razón es la misma que mueve `remote_urls_equivalent` al
  dominio: dos llamantes con copias privadas divergen en cuanto se toca una y no
  la otra, y esta es una regla de verdad, no de formato.
- **`now_ms` lo suministra el llamante.** La narrativa es un artefacto
  determinista: dos renders de los mismos hechos tienen que ser byte-idénticos,
  así que la funcion de "ahora" no se lee del reloj dentro del motor.
- **Precedencia total y determinista** para `human_action_required`:
  **aprobación > UAT > remediación > lease**, alineada con la que ya usa
  `derive_cycle_summary`. Si el pie y `cycle status` pueden discrepar, uno de los
  dos miente, y no hay forma de saber cuál.
- **Falla cerrado ante ciclo inexistente** con el mismo patrón tipado que
  estableció `INC-DEBT-043` para el envelope de contexto: `resolve_cycle_context`
  → `RuntimeContext::open` → `get_cycle`, y el error se propaga en vez de
  convertirse en un default.
- **El override del llamante se conserva, pero gana sobre un default
  *derivado*, no sobre una constante** — que es exactamente lo que este
  documento rechazaba como arreglo.

### Comportamiento verificado sobre el estado real del repo

| Entrada | Antes | Ahora |
|---|---|---|
| ciclo inexistente | `Cycle completed.` / `Nada por ahora.` / **exit 0** | `error: cycle not found: <id>` / **exit 1**, sin narrativa |
| `cycle-45-build-remediate-archive` (`CLOSED`) | `Cycle completed.` | `The cycle closed at phase "archive".` / `Nada por ahora.` |
| `c3n-production-boundary-certification` (`OPEN`, `approval-waiting`, lease caducada) | `Cycle completed.` / `Nada por ahora.` | `The cycle is open at phase "design" (runtime: approval-waiting).` / **`Decide 1 pending approval request: surface.cycle_state#cycle_supersede.`** |

La tercera fila es la que motivó la deuda: **el bloqueo que la vista escondía
ahora se lee en la vista**.

### Dos defectos MÍOS que los tests propios encontraron

1. `unreachable!("guarded by lease_is_live")` **era alcanzable**: una lease
   etiquetada `Live` con la caducidad ya pasada cumple el patrón pero no la
   invariante. La función ahora es **total** — sin panic — y el caso está
   cubierto por un test que pasa la caducidad.
2. `apply_claims` **sobrescribía el override del llamante en silencio**: el
   llamante aplicaba su override, y `apply_claims` lo pisaba después. El orden es
   ahora explícito — claims primero, override después — y vive en una función
   con nombre (`override_what_was_done`) para que el orden se lea.

El segundo es **del mismo género que la deuda original**: una superficie que
ignora lo que se le pide y además no lo dice.

### El hallazgo que la suite completa convirtió en rojo

Al integrar, `cargo test --workspace` cayo con **5 tests rojos de la propia CLI**:
`s_narrative_cli_minimal_renders_to_stdout`, `..._respects_audience_tone_variants`,
`..._uses_default_title_when_omitted`, `..._writes_to_file_when_output_set` y
`..._falls_back_when_cycle_id_absent`. Los cinco usaban
`CliEnvironment::default()` — **sin ledger** — con un id que nunca existió, y
afirmaban `status == 0`.

**Esos tests no eran flojos: eran el defecto escrito como contrato.** Verde
significaba «funciona», y lo que funcionaba era narrar un ciclo inexistente
como completado. El arreglo los dejó rojos, que es exactamente lo que debía
pasar. Lo que **no** era admisible era behavioralizarlos para volver a ponerlos
en verde: habría convertido el arreglo en un giro de guion, y un suite que
exige el defecto no puede sobrevivir a que se corrija.

Ahora los cuatro que sí tienen objeto siembran un ciclo **real** y verifican lo
que pretendían verificar (render a stdout, variantes de `audience`/`tone`,
título por defecto, escritura a fichero). El quinto, `..._falls_back_when_cycle_id_absent`,
**ya no afirma ningún fallback**: `falls_back` era el defecto — inventar una
identidad para un objeto que no existe — y ahora afirma que **falla cerrado y lo
dice**. Se renombró a `s_narrative_cli_fails_closed_when_cycle_id_absent`.

**Medido, no supuesto, al sembrar el fixture:** tres cosas se creían razonables
y eran falsas, y las tres fallaron con el mismo mensaje (`cycle not found`) que
no identifica la causa real. (a) El ledger guarda `manifest.cycle_id` **desnudo**;
calificarlo con el `project_id` parece equivalente y no lo es, porque la
búsqueda es por clave primaria. (b) `RuntimeArgs::default()` **no** resuelve el
proyecto: la capa de identidad pregunta por el root **antes** de tocar el store,
y con un root por defecto mira a otro sitio. (c) `fallback_seed` tiene que ser
**un UUID v4**, aunque `stable_fallback_project_id` hashee cualquier string; un
seed no-UUID se rechaza durante la resolución, mucho antes del store, y el
fallo **se presenta como ciclo ausente**. Un mismo síntoma, tres causas, y
ninguna era la que el mensaje señalaba.

### Un ratchet arquitectónico que el arreglo casi violaba

La suite completa detecto un guard que no es mio:
`conf09b_no_new_runtime_status_references` (**WU-C3 / DELTA-CONF-004**), que
prohibe referenciar las variantes de status derivadas de runtime
(`Remediating`, `Recovering`, `UatWaiting`, `ApprovalPending`) fuera de un
conjunto **decode-only** cerrado. Mi test las enumeraba para comprobar que
ninguna se narraba como completada.

**El guard tenía razón y el arreglo estaba mal.** Desde el cutover esas
variantes son decode-only: la verdad de espera/remediación/recuperación vive en
hechos de Run/Authority y **la etiqueta se deriva** (`derive_cycle_summary`), no
se persiste. Enumerarlas en un test afirmaba contra un status que el dominio ya
no trata como verdad canónica — y el propio mensaje del guard dice cuál es la
vía correcta, que es exactamente la que ya usaba el resto del arreglo.

Reescrito el barrido para recorrer **los statuses persistidos por cada etiqueta
que `derive_cycle_summary` puede derivar** (`""`, `approval-waiting`,
`uat-waiting`, `remediating`, `recovering`), sin nombrar ninguna variante. Es una
afirmación **más fuerte**, no más débil: la narrativa se mantiene honesta para
toda combinación que la autoridad puede producir, incluidas las que una lista
escrita a mano se saltaría. Y evita extender el allowlist, que habría sido
convertir una prohibición en una excepción sin justificación.

### La autofalsación, y por qué su primera versión no valía

`tests/test_cycle_narrative_operator_view_mutation.sh`, cableado como paso
**3j** de `scripts/release.sh`. Cuatro mutaciones, cada una sobre UN punto de
enforcement, cada una exigiendo que caiga el test **nombrado** que lo vigila
(needle por nombre de test, no por el texto de una aserción: tres veces esta
sección que un needle ambiguo invirtió el veredicto). Medido: `PASS=4 FAIL=0
SKIP=0`, con restauración verificada por ausencia de marcadores.

**M2 es la historia de por qué el falsador importa.** Su primera versión rompía
`resolve_cycle_context` para que aceptara el id crudo, y **el test siguió en
verde**. No porque el store dejara de fallar cerrado, sino porque el enlace
siguiente —`get_cycle(cycle_id)?`— seguía intacto y seguía rechazando el id
inexistente. El test pasaba, luego la mutación no había roto la garantía: había
roto otra cosa. Se reescribió para mutar el enlace que **decide** —la lectura del
store, que ante un `Err` fabricaba una narrativa en vez de propagar— y ahí sí
cae.

Un guard que se queda verde cuando se le rompe lo que dice vigilar no es un
guard débil: es un guard **que no vigila**, que es peor, porque declara una
cobertura que no tiene. Y un needle que apunta a un fichero entero puede caer
sobre la ocurrencia equivocada: `get_cycle(cycle_id)?` aparece **dos veces** en
`cycle.rs` (la otra es `run_cycle_status`), y el needle incluye el comentario
ancla por eso.

### Lo que esto NO cierra

El ciclo `c3n-production-boundary-certification` sigue `OPEN` en
`approval-waiting` con su lease caducada. Este arreglo hace el bloqueo
**visible**; no lo concede. Conceder una aprobación de `surface.cycle_state` desde
quien pide la mutación anularía el gate, así que sigue siendo decisión de
operador.

