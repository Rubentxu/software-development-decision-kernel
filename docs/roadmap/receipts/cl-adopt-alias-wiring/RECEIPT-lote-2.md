# RECEIPT — INC-DEBT-059, lote 2 (el arreglo)

**Cycle:** `p-63676b11dc0ef88f/identity-alias`
**Deuda:** INC-DEBT-059 (high/P1)
**Lote:** 2 de 2. El lote 1 (tests RED) es `07c3fd5c`.
**Fecha:** 2026-10-02T20:40:00Z
**Baseline del árbol al falsificar:** `sha256=d7961065c166135b` (tres ficheros), restaurado y verificado

---

## Qué cambió, y por qué así y no de otra forma

`AdoptionPlanInput` deja de llevar `remote_url`, `pinned_project_id`, `scope` y
`fallback_seed`, y lleva `identity: ResolvedProjectIdentity`. `plan_adoption`
deja de llamar a `resolve_project_identity` **por completo**.

La razón de que sea así y no «un campo opcional con la identidad resuelta» es
que el objetivo del lote —«un solo punto de decisión» de ADR-0152 — tiene que
ser cierto **por construcción**. Con la identidad ya resuelta, la posibilidad de
derivar por dentro **desaparece**, porque el input ya no tiene de qué derivar.
Con un campo opcional, alguien reañade la llamada y ningún test de
comportamiento lo nota: un reader que hardcodea una derivación se comporta
igual mientras el alias no esté declarado.

Las tres superficies de producción resuelven por
`resolve_identity_honoring_pin_with` y pasan el resultado:
`prepare_adoption_plan` (`lib.rs`), `converge_adoption` +
`resolve_identity` (`context_cmd.rs`, **dos** sitios de resolución, no uno) y
`generation_destination` (`lib.rs`, añadido en lote 2 — ver abajo).

## Las cuatro superficies, y por qué la cuarta no salió de la lectura

| superficie | qué hacía | cómo se midió |
|---|---|---|
| `sddk adopt *` | reportaba el id **retirado**, `status: absent`, sobre un ledger inexistente; y `apply` escribía un **segundo** recibo bajo ese id | `repro-c3{,b}.sh` |
| `sddk context bootstrap` | reportaba el id retirado con **`adoption: complete`** y escribía un binding de sesión en su data dir | `repro-c3c.sh` |
| `sddk generate docs` | escribía la documentación generada bajo el data dir del id **retirado** | `repro-c3d.sh` |
| el engine | derivaba la identidad, y no puede no hacerlo: no carga la tabla | test estructural |

Las tres primeras son la misma clase. La cuarta se la saltó **el falsificador**,
buscando un segundo resolutor, con el arreglo de las otras tres ya en verde. No
salió de leer el SCOPE ni de medir el ARRANCLO; salió de contar puntos de
llamada. Por eso el test estructural de la CLI cuenta llamadas y no busca
subcadenas: `lib.rs` debe tener **exactamente una**.

## El falsificador encontró tres defectos, y dos eran suyos

| # | mutación | veredicto |
|---|---|---|
| M1 | borrar la línea de declaración de `identity_alias` en `adopt status` | **detectada**, 6 → 5 |
| M2 | `plan_adoption` vuelve a derivar la identidad | **detectada**, 6 → 1 |
| M3 | `lib.rs` introduce un segundo `resolve_project_identity` | **detectada**, 6 → 4 |

**PASS=4 FAIL=0 SKIP=0** (el cuarto PASS es el baseline).

Los tres defectos del arnés, en orden:

1. **M1 no aterrizaba.** Quitar el `identity_alias: {}` del format string y
   dejar el argumento hace que `format!` falle: más argumentos que marcadores
   es un error de compilación, no un aviso. Se declaraba SKIP — correcto por la
   regla — pero el criterio 3 de ADR-0152 **exige ejercitar esa mutación**, así
   que hubo que quitar las dos mitades. Un SKIP honesto no cumple un criterio
   que pide la medición.
2. **M3 era una mutación mala, no un hueco.** Añadía un comentario con el
   nombre de la función; el guard filtra comentarios a propósito (documentación
   no es código, el criterio de `version_source.rs::production_only`) y un
   comentario que nombra una función **no la llama**. La primera lectura apuntaba
   a FAIL y era el guard el equivocado. M3 pasó a reintroducir una
   llamada real.
3. **El restore era a ciegas y se llevó trabajo.** El backup se hacía una sola
   vez (`cp … ||`), así que la segunda ejecución restauró un `lib.rs` de la
   primera y **borró el arreglo de `generation_destination`**, hecho después.
   Peor: no avisó. El arreglo es un backup **fresco en cada ejecución** más una
   **verificación por sha256** de que el árbol volvió al baseline, que es lo que
   convierte «restauré» en un hecho comprobado en vez de en una intención.

## Un arreglo demasiado amplio, cazado por un test que ya existía

El defecto preexistente que se encontró midiendo `generate docs` no era «falta
una semilla»: era que `find_persisted_fallback_seed` sólo encontraba recibos con
`identity_source == Fallback`, y un recibo escrito **bajo un pin** tiene
`Pinned`, luego era invisible. El primer arreglo derivó la semilla de la ruta
canónica cuando no había nada persistido — copiando lo que hace
`resolve_project_ids`.

Eso es demasiado: convierte **cualquier** directorio en un proyecto, deja el
fallback in-repo de los repos no adoptados como código muerto, y lo cazó un test
que ya existía, `real_cli_exit_status_tracks_lint_errors_and_stale_checks`, con
`error[SDDK009] docs/generated/workflow.md: generated workflow documentation is
missing or stale`. El arreglo correcto era **una cláusula en el predicado**
(`Fallback | Pinned`), no una fuente nueva de semillas. El fallo del arreglo
ancho es su propio dato: `find_persisted_fallback_seed` devuelve la semilla
porque el pin sobrescribe el `project_id`, **no** la semilla.

## La mudanza de los dos tests del pin, y el hueco que dejó al descubierto

`adoption_identity.rs` tenía dos tests del pin dentro del engine, que ya no
decide identidad. Se midió dónde vive ahora cada propiedad antes de mover nada:

- `pinned_project_id_wins_over_remote_derivation` → **ya estaba medido** e2e en
  `project_pin_e2e.rs::pin_overrides_remote_drift` y
  `::pinned_identity_is_authoritative_for_adopt_config_and_cycle`. La propiedad
  no se perdió: estaba en el sitio donde ahora se decide.
- `malformed_pin_fails_closed_instead_of_falling_back_to_the_remote` → **no**
  estaba medido fuera del engine. Es un hueco real, y se rellenó con
  `project_pin_e2e.rs::a_malformed_pin_fails_closed_instead_of_deriving` en el
  mismo cambio.

Eso es lo que evita la STOP condition 3: no se dejó de medir, se midió donde
mide. El pin malformado **falla cerrado con el mensaje nombrando el pin**
(`project pin … holds '…', which is not a project_id (expected p-*)`), y el
contraste con un pin **válido** da `status: absent` sin ese mensaje
(`ff-pin-malformed.sh`): la aserción que discrimina es la que nombra, no el
código de salida, que es 1 en los dos casos.

## Una propiedad mal especificada por mí, en mi propio guard

`the_cli_has_no_second_identity_resolver` contaba el **nombre** y daba 2 en vez
de 1: la línea `use` también cuenta. La propiedad es «exactamente una
**llamada**», y cuenta con paréntesis. Se corrige contando
`resolve_project_identity(`.

## Gates

| gate | resultado |
|---|---|
| `cargo test --workspace` | **5361 passed, 0 failed**, `cargo exit=0` |
| tests de la costura (`alias_adoption_wiring`) | **6/6** |
| `cargo fmt --all -- --check` | limpio |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** |
| falsificador `ff-inc059.sh` | **PASS=4 FAIL=0 SKIP=0** |
| `check_debt_index_coherence` · `test_docs_script_contamination` · `test_gate_coverage` · `test_release_state_pointer` | PASS |
| `test_adr_0153_criteria` | PASS=7 FAIL=0 (cableado, corre en el release) |

**Ningún test que ya fuera verde se reescribió.** El único test del engine que
se tocó de forma no mecánica es `refresh_preserves_identity_and_updates_runtime_metadata`,
que **cayó** al migrar el fixture: `v1` derivaba del remote y el helper nuevo
derivaba de la ruta, así que al mutar `remote_url` a posteriori el `project_id`
ya no cambiaba. La traducción fiel es re-resolver la identidad con ese remote —
que es lo que el engine hacía antes — y entonces pasa sin tocar una sola
aserción.

## Lo que NO se afirma

- **El storage real de esta máquina no se ha tocado.** Ni una escritura fuera del
  sandbox de prueba.
- **Este arreglo no limpia lo que ya está sucio.** Un recibo espurio bajo un id
  retirado **sigue ahí**; los bindings atrapados **siguen atrapados**. El
  criterio 5 de ADR-0152 depende de esa limpieza y **no** la hace este trabajo:
  la decisión de qué receipt espurios retirar es del operador.
- **Los criterios 5 y 6 de ADR-0152 siguen sin medir.** Cerrar esta INC devuelve
  el **criterio 3** a la condición de medirse — y está medido, por su propio
  falsificador M1. El ADR **no se promueve**: seis criterios no se suman.
- La ruta **forge** de `release apply` contra un GitHub real sigue sin medir.
  Pendiente propio, declarado.

## Criterio 3 de ADR-0152, con su falsificador

`adopt status` y `context bootstrap` declaran el `from` y el `to`, y
`project resolve` ya lo hacía. El criterio exige: *«borrar la línea de
declaración y exigir que el test falle»*. Eso es **M1**, y cae: 6 → 5. **C3 pasa
de ROJO a medido.**
