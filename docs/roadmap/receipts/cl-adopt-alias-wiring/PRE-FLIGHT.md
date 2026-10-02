# cl-adopt-alias-wiring — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/identity-alias`
**Date:** 2026-10-02T18:45:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto v2.5.2)
**HEAD:** `b149451c` (`docs(debt): la superficie de INC-DEBT-059 era mas ancha y
context bootstrap es peor que adopt`), `HEAD == origin/main`, árbol limpio
**Authority:** INC-DEBT-059 (high/P1, open) · ADR-0152 `status: proposed` ·
SCOPE-CONTRACT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | La deuda está **verificada vigente**, no heredada | OK — reproducida con el binario de este checkout y sandbox aislado, §0 del SCOPE |
| 2 | Deuda con criterio, severidad razonada y criterios de cierre falsables | OK — INC-DEBT-059, 6 criterios de cierre |
| 3 | SCOPE con objetivo falsable, no-objetivos y STOP conditions | OK |
| 4 | Superficie mapeada y **leída**, no inferida | OK — abajo, con línea |
| 5 | Superficie acotada al lote | 4 ficheros de código + 2 de test + el ADR, ninguno fuera |
| 6 | Riesgo de datos | **diferente de cero, y es el punto**: este trabajo **escribe** sobre superficies que el defecto ya ha contaminado. Ver §Riesgo |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `prepare_adoption_plan` | `crates/sddk-cli/src/lib.rs:2133` | **el bypass nº 1**; lee el pin y lo pasa a `plan_adoption`, nunca carga la tabla |
| `plan_adoption` | `crates/sddk-engine/src/adoption.rs:195` | el punto donde se re-deriva; llama a `resolve_project_identity` en `adoption.rs:203` |
| `AdoptionPlanInput` | `adoption.rs:29-58` | los cuatro campos de derivación: `remote_url`, `pinned_project_id`, `scope`, `fallback_seed` |
| `AdoptionPlan.identity` | `adoption.rs:65` | **ya** es `ResolvedProjectIdentity` — el plan transporta la identidad completa; lo que falta es quién la produce |
| `validate_plan_input` | `adoption.rs:709` | valida el formato del pin; con el rediseño esa validación pasa a ser la de la CLI, que ya existe (`load_project_pin`, `lib.rs:1718`) |
| `resolve_identity_honoring_pin_with` | `lib.rs:1568` | **la autoridad**. Deriva, luego el pin sobre el id, luego el alias sobre las dos ramas. Su doc (`:1532-1537`) explica por qué el orden no es intercambiable |
| `adoption_result_text` | `lib.rs:2306` | donde iría la declaración `from -> to`; hoy no tiene de dónde sacarla |
| `resolve_identity` (context) | `context_cmd.rs:1003` | **el bypass nº 2**; llama a `sddk_domain::resolve_project_identity` directamente en `:1023-1024` |
| su doc | `context_cmd.rs:1001` | «with the SAME resolver as `adopt`» — literalmente cierto y exactamente lo contrario de lo que importa |
| `converge_adoption` | `context_cmd.rs:1050` | el segundo punto de llamada de producción; reenvía el id **solo** si `identity_source == Pinned` (`:1066-1069`) |
| `ResolvedProjectIdentity` | `crates/sddk-domain/src/identity.rs:255-257` | deriva `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` — **la STOP condition 4 no se dispara**, ya cumple lo que los tests del engine necesitan |
| 11 construcciones de `AdoptionPlanInput` | 2 producción + 9 tests | el coste real del rediseño, contado antes de empezar y no después |
| los 2 tests del pin que viven en el engine | `adoption_identity.rs:146` y `:199` | `pinned_project_id_wins_over_remote_derivation` y `malformed_pin_fails_closed_instead_of_falling_back_to_the_remote`; son los que cazaron INC-DEBT-049 (F51, F52) y los que migran a la CLI |
| `AliasTable` | `crates/sddk-domain/src/identity.rs` | registro **de datos**, append-only, sin `remove`: correcto, no se toca |

## Lote de este apply

**Lote 1 — el test RED y nada más.** Escribir los tests que deben caer **antes**
de tocar producción, y ejecutarlos para que caigan. Si alguno no cae, la STOP
condition 1 se dispara y se para.

- test **estructural**: `adoption.rs` no nombra `resolve_project_identity` en
  producción, y ni `prepare_adoption_plan` ni `resolve_identity` de
  `context_cmd.rs` lo llaman.
- test e2e de la costura: alias declarado + `adopt status` ⇒ `to` + `from -> to`.
- test e2e de la costura: alias declarado + `context bootstrap` ⇒ `to`, y el
  binding bajo el data dir del `to`.
- test e2e de no-escritura: `adopt apply` y `context bootstrap` **no** crean
  `adoption.json` ni binding bajo el `from`.

Un lote que solo escribe tests que caen es lo que permite que el siguiente lote
sea «hacerlos verdes» y no «comprobar a posteriori si algo se movió».

## §Riesgo: este trabajo escribe, y el storage real ya está tocado

A diferencia de casi todo lo anterior en este objetivo, este arreglo **escribe**
sobre superficies que el defecto ya ha contaminado. Concretamente:

1. Un proyecto con alias declarado que hoy tenga un recibo espurio bajo el
   `from` **seguirá teniéndolo** después del arreglo. El arreglo impide que se
   creen más; **no limpia los que ya están**. El criterio 5 de ADR-0152 (audit
   con 0 huérfanos) depende de esa limpieza y **no** la hace este trabajo.
2. `context bootstrap` ya pudo escribir bindings bajo data dirs de ids
   retirados. Tras el arreglo quedan **atrapados**: nadie los leerá, porque el
   bootstrap abrirá el data dir del `to`.
3. Por tanto: **el storage real de esta máquina no se toca en este ciclo.** Ni
   una escritura fuera del sandbox de prueba. Cualquier limpieza es otro SCOPE,
   con su propia medición, y la decisión de qué receipt espurios retirar es
   del operador.

Este riesgo está escrito **antes** de empezar y no se descubre al final, que es
la forma en que INC-DEBT-059 se midió a sí mismo: la primera versión declaraba una
superficie más estrecha que la real, y se descubrió al mapear el segundo punto de
llamada.

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún test que ya fuera verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`check_debt_index_coherence` · `test_adr_promotion_format` ·
`test_docs_script_contamination` · `test_gate_coverage` ·
`test_adr_0153_criteria` (ahora cableado, así que corre en el release y no
puede romperse en silencio) · `test_release_state_pointer` · un falsificador
end-to-end **propio** que cruce la costura `adopt` ↔ store.
