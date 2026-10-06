# RECEIPT — `cl-adr0157-criteria-repoint`

**Bloque:** desbloquear la entrega — `REL-2.12.1` murió en el 1b por este gate
**Cerrado:** 2026-10-06
**Workspace:** 2.12.1 (declarada, no publicada; último tag remoto `v2.12.0`)
**PRE-FLIGHT:** [`PRE-FLIGHT.md`](./PRE-FLIGHT.md) (`Readiness: READY`)

---

## §1 — Lo que pasó, medido

`REL-2.12.1` (la释放 que lanzó la otra sesión) murió en el paso `1b/15` con
`RELEASE_RC=1`:

```
tests/test_adr_0157_criteria.sh → PASS=24 FAIL=0 NOT_APPLIED=2
  RESULT: FAIL — ADR-0157 no cumple todos sus criterios; NO se sostiene como accepted
```

**`FAIL=0` y aun así falla.** No hay ni una aserción rota: fallan dos criterios
porque **no se aplicaron**. El gate lo dice explícito y es lo correcto — un
criterio que no se ejecutó no está cumplido, no se sabe— y por eso el release se
paró en vez de publicar con un ADR sin certificar.

## §2 — Los dos, y la causa única

Los dos N/A son **del lado `sddk-domain`** y salen de **un solo commit**:

| Criterio | Test nombrado | Destino real |
|---|---|---|
| C1 | `the_decision_module_names_no_concrete_technology` | `el_nucleo_no_nombra_tecnologia_concreta` |
| C2 | `the_fitness_scanner_can_actually_see_a_name` | `el_guard_cubre_todos_los_modulos_del_dominio` |

`e60dae75` — *"la ley de pureza del núcleo se vigila en los cuarenta y seis
módulos, no en uno"* — consolidó los dos tests de pureza del dominio, y **nadie
repuntó los criterios del ADR**. El criterio C2 sigue declarando el test de
dominio **y** el de motor; el de motor existe y pasa, luego esa mitad nunca estuvo
rota.

Repuntados a los sucesores, que miden **lo mismo o más**: la pureza ya no se
vigila en un módulo sino en 46, y el contrapeso de C2 comprueba que el guard
mire los 46 y no un `include_str!` de uno solo — que era justo el riesgo que el
control viejo tapaba.

## §3 — El gate no se tocó, y por eso importa

El mecanismo que detectó esto —`NOT_APPLIED` no cuenta como `PASS`, y un criterio
con cero tests ejecutados es *no medido*— **se queda exactamente como estaba**.
Ablandar el umbral para que el release pasara habría sido comprar una entrega
con un ADR sin certificar.

Y el propio gate ya tenía el precedente escrito en sus comentarios, en C4: la
fila `lockstep_passes_when_the_ref_has_no_v_prefix` «FE con ADR-0159, y no por un
descuido», y la conclusión de que *«repuntar el criterio a un test que dijera lo
contrario habría sido dejar el gate verde sobre una ley que ya no existe, que es
peor que un N/A»*. Este bloque sigue esa instrucción al revés: repunta a tests
que miden la misma propiedad o más amplia.

**El sistema funcionó.** El fallo estaba aguas arriba, en quien movió los tests y
no actualizó el ADR que los certifica. El gate hizo lo único que puede hacer un
gate: negarse a decir que algo está bien sin haberlo medido.

## §4 — Gates

```
antes    PASS=24 FAIL=0 NOT_APPLIED=2   rc=1
después  PASS=26 FAIL=0 NOT_APPLIED=0   rc=0
          RESULT: PASS — los quince criterios de ADR-0157 están verdes medidos uno a uno
```

Los dos criterios reparados, ejecutados y no solo nombrados:

```
[ok] C1 — el modulo que decide no nombra tecnologia concreta (1 test(s))
[ok] C2 — el fitness puede ver un nombre y su excepcion no crece (1 test(s))
```

Nótese que `PASS` sube de 24 a **26**, no a 25: el criterio C1 tenía dos mitades
(la de dominio y la de motor) y C2 otras dos, y la mitad de dominio de cada una
pasó de *no medido* a *medido*. Un gate que reporta el mismo número antes y
después de un arreglo es un gate que no distingue el arreglo de no hacerlo.

## §5 — Conocimiento negativo

- **No se reconstruyeron los tests antiguos.** Si la consolidación hubiera dejado
  la propiedad sin cubrir, la corrección sería inventar un test que no existe.
  Se verificó que los sucesores cubren lo mismo antes de repuntar.
- **No hay guard nuevo**, y es deliberado: el guard ya existía y fue él quien lo
  encontró. Añadir otro habría sido crear un sitio más donde no mirar.
- **El segundo N/A no era del motor.** Comprobar la existencia por crate, y no
  sobre todo `crates/`, es lo que lo separa: `the_fitness_scanner_can_actually_
  see_a_name` **existe** —en `sddk-engine`— y aun así su mitad de dominio daba
  N/A. Un grep global habría dicho «existe» y cerrado el hallazgo.
