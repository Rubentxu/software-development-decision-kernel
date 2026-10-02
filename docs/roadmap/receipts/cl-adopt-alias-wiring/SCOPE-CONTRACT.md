# SCOPE-CONTRACT — INC-DEBT-059

**Cycle:** `p-63676b11dc0ef88f/identity-alias`
**Deuda:** [INC-DEBT-059](../../debt/INC-DEBT-059-ADOPT-REDERIVES-IDENTITY-AND-WRITES-A-SECOND-RECEIPT-UNDER-A-RETIRED-PROJECT-ID.md) (high/P1, open)
**ADR:** ADR-0152 `status: proposed` — este trabajo es lo que deja su criterio 3 en condiciones de volverse verde
**Fecha:** 2026-10-02T18:40:00Z

---

## §0. La deuda está verificada vigente, no heredada

Medida en esta sesión, con el binario construido de este checkout y un sandbox
aislado fuera del repositorio (`/var/home/rubentxu/repro-c3{,b,c}.sh`;
`HOME`/`XDG_*` propios, porque montarlo dentro falsearía la prueba — AGENTS.md
§1). Checkout nuevo, `adopt apply` con el pin en `X`, pin retirado, alias
`Y -> X` declarado. Mismo checkout, sin pin, mismo instante:

```
$ sddk project resolve --root ws --scope .
project_id: p-0000000000000aaa
identity_alias: p-c4319c598bc98be8 -> p-0000000000000aaa     exit 0

$ sddk adopt status --root ws --scope .
status: absent
project_id: p-c4319c598bc98be8        # el id RETIRADO
receipt: .../p-c4319c598bc98be8/.../adoption.json
ledger:  .../p-c4319c598bc98be8/ledger.sqlite                exit 1

$ sddk context bootstrap --root ws --scope . --session probe-c3
project: p-c4319c598bc98be8        # el id RETIRADO
adoption: complete                 # afirma complete
binding: sddk/context/bindings/probe-c3.json (written: true)
```

**Tres comandos, tres respuestas.** Y `adopt apply` y `context bootstrap` escriben
**cada uno** un segundo recibo bajo el id retirado.

---

## Objetivo, falsable

Que **`adopt` y `context bootstrap` resuelvan la identidad por el mismo sitio que
`project resolve`**, y lo declaren.

Formulado como propiedad con dientes, no como intención:

1. **Un solo resolvedor, por construcción.** `plan_adoption` **no** llama a
   `resolve_project_identity`. No «deja de llamarlo en el camino de producción»:
   no lo llama en absoluto, porque el campo que lo alimentaba desaparece del
   input. Medible con un test estructural que lea el fuente.
2. **La declaración es visible en las dos superficies.** Con alias declarado,
   `adopt status` **y** `context bootstrap` reportan el `to` y nombra
   `from -> to`.
3. **Ningún escritor escribe bajo el id retirado.** `adopt apply` y
   `context bootstrap` sobre un checkout con alias **no** crean un
   `adoption.json` bajo el `from`, ni un binding bajo su data dir.

## No-objetivos, explícitos

- **No** se toca `crates/sddk-domain/src/identity.rs`. `ProjectAlias`,
  `AliasTable` y `AliasResolution` están correctos y sus 6/6 tests verdes; el
  defecto no está en la resolución sino en que **dos superficies no la piden**.
- **No** se toca el store (`crates/sddk-cli/src/project_alias.rs`). Append-only,
  sin `remove`, con el ciclo rechazado en la declaración: correcto.
- **No** se migra ningún recibo. `project_id` está horneado en el `content_hash`
  de un fact log append-only; eso ya se midió y se cerró en session-66. Este
  trabajo **no vuelve a intentarlo**.
- **No** se retira ni se reconstruye el alias de skillgraph. Es decisión del
  operador y depende de la sesión concurrente.
- **No** se toca `resolve_identity_honoring_pin_with`. Su semántica —derivar,
  luego el pin sobre el id, luego el alias sobre las dos ramas— ya es la
  correcta, y su doc explica por qué el orden no es intercambiable. Es el
  **consumidor** el que está mal cableado.
- **No** se acepta ADR-0152 al cerrar esto. Cerrar la INC devuelve el criterio 3 a
  la condición de medirse; la aceptación es un acto posterior que exige los
  **seis** criterios, y el 5 y el 6 siguen sin medir.

## Decisión de diseño, y la forma corta que se descarta

**Se hace:** `AdoptionPlanInput` deja de llevar `remote_url`,
`pinned_project_id`, `scope` y `fallback_seed`, y lleva
`identity: ResolvedProjectIdentity`. `plan_adoption` deja de derivar. Los dos
puntos de llamada de producción resuelven **una vez** por
`resolve_identity_honoring_pin_with` y pasan el resultado.

**Se descarta:** pasar el id resuelto como `pinned_project_id`. No exige tocar
nada y **no funciona** — el engine lo volvería a tratar como pin,
`identity_source` no viajaría y `alias_origin` se perdería igual, un nivel más
adentro y con una forma que **parece** correcta. Y `context_cmd.rs:1066-1069` ya
hace exactamente eso hoy, condicionado a `identity_source == Pinned`, que es por
lo que un checkout con alias y **sin** pin es el caso que se rompe. Esa condición
se **elimina**, no se propaga.

**Por qué esta y no una tercera:** la afirmación de «un solo punto de decisión» de
ADR-0152 tiene que ser cierta **por construcción**, no por nota. Con la
resolución antes del engine, la posibilidad de derivar por dentro **desaparece**;
con un campo opcional, alguien la vuelve a añadir y nada la ve.

## STOP conditions

Se para y se reporta, sin forzar, si ocurre cualquiera de estas:

1. **El test RED no cae antes del arreglo.** Si la propiedad 1 ya se cumple, este
   trabajo no tiene objeto y hay que decirlo, no buscar un recorte de SCOPE que
   la produzca.
2. **El engine necesita acceso a filesystem** para construir la identidad
   resuelta. El engine es filesystem-free **por diseño** y `plan_adoption` es una
   función pura; si el diseño propuesto lo rompe, se para y se replantea.
3. **Los dos tests del pin que viven en el engine no se pueden portar a la CLI**
   sin perder lo que medían. `pinned_project_id_wins_over_remote_derivation` y
   `malformed_pin_fails_closed_instead_of_falling_back_to_the_remote` son los
   que cazaron INC-DEBT-049 (F51, F52). Perder cobertura **no** se acepta: si no
   hay dónde medirlos, se para y se reporta el hueco en vez de dejar de medir.
4. **`ResolvedProjectIdentity` no satisface lo que los tests del engine
   necesitan** (`PartialEq`, `Eq`, `Clone`) y hacerlo satisfacible exigiría
   tocar el dominio, que está en no-objetivos.
5. **Aparece un tercer punto de llamada de `plan_adoption` en producción** que no
   se haya mapeado. Se para: ampliar la superficie a ciegas es exactamente el
   defecto que esta INC denuncia.

## Superficie autorizada, exactamente esta

| fichero | qué se toca |
|---|---|
| `crates/sddk-engine/src/adoption.rs` | `AdoptionPlanInput`, `plan_adoption`, `validate_plan_input`, `AdoptionStatus.alias_origin` |
| `crates/sddk-cli/src/lib.rs` | `prepare_adoption_plan` resuelve por el canónico; `adoption_result_text` declara el salto; `generation_destination` resuelve por el canónico (**añadido en lote 2**, ver abajo); doc de `ProjectPin` corregido |
| `crates/sddk-cli/src/context_cmd.rs` | `resolve_identity` y `converge_adoption` resuelven por el canónico; se elimina la condición de reenvío; doc corregido |
| `crates/sddk-engine/tests/adoption_identity.rs` | los dos tests del pin migran a la CLI |
| `crates/sddk-cli/tests/adoption_contract.rs` | los tests que cruzan la costura, que hoy no existen |
| `crates/sddk-cli/tests/alias_adoption_wiring.rs` | **fichero nuevo**, añadido en lote 1: la costura tiene su propio hogar |
| `docs/architecture/adrs/ADR-0152-…md` | solo si el criterio 3 queda medible y medido |

**Por qué un fichero nuevo y no `adoption_contract.rs`.** El SCOPE inicial solo
autorizaba el segundo. El doc de ese fichero dice qué es —«migrated from
`tests/test_adoption_contract.sh`», asserts sobre los tokens que lleva el
`agents/sddk-adopt.md`— y meter ahí tests de cableado del store de alias
ensuciaría un contrato que es de otra cosa, para que quien lo lea tenga que
saltarse medio doc para saber de qué va. El fichero nuevo lleva su propio doc
explicando **por qué existe**: los dos ficheros que cubren `adopt` y el pin tenían
**cero** ocurrencias de «alias», medido, luego la costura no la cruzaba nadie.

### Enmienda de lote 2: `generation_destination` entra en el SCOPE

**Medido antes de decidir, no inferido** (`repro-c3d.sh`, checkout con alias
declarado):

```
$ sddk project resolve --root ws --scope .
project_id: p-0000000000000aaa
identity_alias: p-c4319c598bc98be8 -> p-0000000000000aaa

$ sddk generate docs --root ws
wrote docs/generated/workflow.md

$ find …/data/sddk/projects -type d -name generated
p-c4319c598bc98be8/generated          <-- el id RETIRADO
```

`generation_destination` (`lib.rs:1217`) era un **cuarto** resolutor, con el
mismo bypass que los otros tres y con un doc que afirma lo contrario: «We reuse
the same resolution as adoption» — reutilizaba la *forma* de la resolución de
adopción, no la resolución. Se lo saltó el falsificador de este mismo lote,
mientras buscaba un segundo resolutor; no salió de la lectura del SCOPE.

Se añade al SCOPE por dos razones, y la segunda es la que manda: está en un
fichero ya autorizado, y **dejarlo haría falsa la afirmación central de este
lote**. El lote dice que hay un punto único de decisión; si `generate docs` sigue
derivando por su cuenta, esa afirmación es la misma clase de doc que este
trabajo lleva tres corrigiendo — y sería el cuarto.

El arreglo es el mismo patrón, no uno nuevo: usar el resolver canónico, sin
añadir ninguna vía.

**Fuera de esta superficie, cualquier cambio necesita otro SCOPE.**

## Verificación exigida antes de cerrar la INC

Los seis criterios de cierre de INC-DEBT-059, cada uno con su falsificador, y
además:

- `cargo test --workspace` verde, **sin reescribir ningún test que ya fuera
  verde antes del cambio** (así se hizo con los 3 tests de
  `failed_precondition` en session-67b, y con
  `cli_release_plan_refuses_on_version_mismatch` en session-67).
- `cargo fmt --check` y `clippy --workspace --all-targets -- -D warnings` limpios.
- Un falsificador end-to-end que **cruce** la costura `adopt` ↔ store, porque
  ningún test la cruzaba y esa es la razón del defecto.
