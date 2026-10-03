# RECEIPT — cl-release-forge-testability

**Cycle:** `p-63676b11dc0ef88f/cl-release-forge-testability`
**Session:** session-69n
**Date:** 2026-10-03
**Concernia:** el cableado de testabilidad de la ruta `release apply --route forge`, y nada más
**Deuda que cierra:** ninguna nueva. **Deuda que reencuentra:** el guard que mira lo que tiene al lado.

---

## Qué se verificó antes de tocar nada

Dos comentarios del propio código (`release_cmd.rs:2064` y `:2110`) afirmaban que
`release apply --route forge` «no tiene test» y «no es alcanzable sin red».

**La primera es cierta. La segunda no**, y ahí está el hallazgo: el adaptador
`GitHubForge` ya tenía `pub fn with_runner`, que dos tests del gateway usaban;
`plan_release` tomaba `&dyn Forge`; `apply_release` tomaba `&mut dyn Forge`; y
`MockForge` era `pub` y estaba re-exportado. **Todo el mecanismo de inyección
existía y funcionaba.** Lo único que faltaba era el seam en el call site, que
construía `GitHubForge::new(repo)` con el runner real y no dejaba sustituirlo.

Es la diferencia entre *«esto no se puede probar»* y *«esto no se ha conectado para
poder probarse»*. La segunda es un defecto de cableado, y la segunda se arregla.

## Qué se cambió

Un solo fichero de producto, `crates/sddk-cli/src/release_cmd.rs`:

- El cuerpo de la rama `ReleaseRoute::Forge` pasa a
  `fn apply_release_forge(gateway, forge: &mut dyn Forge, project_id, args, root, timestamp, actor)`.
- El brazo queda en tres líneas: resolver `--repo`, construir el adaptador,
  delegar.
- **No** se tocó `GitHubForge`, `apply_release`, `plan_release` ni
  `authorize_release`: ya eran correctos. El defecto estaba **solo** en el
  call site.
- **Sin** ensanchar ninguna visibilidad: los tests del módulo ya llegan con
  `super::`.
- Los dos comentarios que afirmaban que la ruta carece de prueba, corregidos.

**Lo que NO cambió**, y R4 lo fija: mismas capacidades (`pr.create`, `pr.merge`,
`release.create`), mismo orden `CreatePr → MergePr → CreateRelease`, y
`apply_release` sigue **dentro** del `AdmissionTicket` (ADR-0132). STOP 1
descartaba el arreglo si la extracción relajara cualquiera de esas cosas.

## Verificación

| Comprobación | Resultado |
|---|---|
| `cargo test -p sddk-cli --lib release_cmd` | **15 passed / 0 failed** |
| `cargo test -p sddk-cli` | **1461 passed / 0 failed** (base 1456, +5) |
| `cargo fmt --check` | exit 0 |
| `cargo clippy -p sddk-cli --all-targets -- -D warnings` | exit 0 |
| `15-falsify-forge.py` (5 mutaciones) | **5/5 DETECTADA**, 0 no medibles, exit 0 |
| scanner de caracteres no latinos | CLEAN |

Ningún verde reescrito: los 11 tests previos del módulo pasaron sin tocarlos.

## Falsificación

`15-falsify-forge.py` muta el producto en la dirección de cada modo de fallo y
exige que caigan los guards. Restaura **por bytes**.

| Mutación | Cae |
|---|---|
| M1 el cuerpo vuelve a la rama | **R3** (+R1) |
| M2 la función devuelve el outcome sin aplicar la cadena | **R2** (+R4) |
| M3 `authorize_release` deja de exigir `pr.merge` | **R4** |
| M4 `apply_release` sale del `AdmissionTicket` | **R4** |
| M5 el doc vuelve a afirmar que la ruta no tiene prueba | **R5** |

**M1 cambió de forma al corregirse R3**, y eso hay que decirlo: el PLAN declaraba
«la rama vuelve a construir `GitHubForge::new` en línea», pero ese **no** era el
defecto — el DISEÑO dice que el brazo construye el adaptador y delega. R3
afirmaba lo contrario, luego R3 estaba equivocado.

**M2 es la que más importa**: es el modo de fallo de un test decorativo —una
envoltura que devuelve un outcome plausible sin ejecutar nada—, y R2 lo
distingue exigiendo que el doble registre el release publicado. Cae R2 **y** R4,
porque quitar el ticket rompe los dos.

## Los cuatro defectos encontrados, y dónde estaban

Ninguno estaba en el producto del release. Tres estaban en guards y uno en el
propio instrumento.

### 4.1 — R3 afirmaba lo contrario que el DISEÑO

`R3` decía que el brazo no puede construir `GitHubForge::new`. Eso es
exactamente lo que el DISEÑO prescribe. Un guard que contradice el diseño obliga
a elegir entre romper el diseño o romper el guard, y en ese conflicto el que se
equivoca es el guard.

La propiedad que de verdad importa no es *quién elige el runner*, sino que **el
cuerpo no vuelva a la rama**: si alguien pega el cuerpo de vuelta y deja
`apply_release_forge` como envoltura fina, R1 seguiría verde —la función existe—
y R2 ejecutaría la envoltura, no la ruta. Las anclas son ahora las del cuerpo
(`with_github_releases_ticket::<`, `plan_release(`, `apply_release(`), no las del
brazo.

### 4.2 — R5 se detectaba a sí mismo

Buscaba en todo el fichero frases que su **propio doc** contiene, así que caía
con el defecto ya corregido. Ahora busca solo antes de su propia definición. Un
guard que se veta con su propia redacción no vigila el código, vigila cómo está
escrito.

### 4.3 — R4 llevaba dos anclas que medían lo que tenían al lado

La primera ancla, `ReleaseRoute::Forge => vec![`, se llevó la lista de **PASOS**
del plan (`"create_pr"`, `"merge_pr"`, `"create_release"`) en vez de la de
**CAPACIDADES** (`"pr.create"`, …), que está en otra función y tiene la misma
forma. La segunda, `with_github_releases_ticket`, se llevó el **`use` de la
cabecera** en vez de la llamada. Las dos caían siempre —se extrajera o no— y
además nunca vigilaban lo que decían vigilar: el ancla es ahora la **función**
(`fn authorize_release`, cuerpo acotado) y la **llamada** (`::<`).

### 4.4 — Y el falsificador era CIEGO, y no lo decía

Este es el más grave de la serie, porque producía **falsos negativos sobre la
propiedad misma que el ciclo viene a demostrar**:

- Su extractor usaba `^test (\w+) \.\.\. FAILED`, pero `cargo test` imprime el
  nombre **completo** —`release_cmd::tests::r4_...`— y `::` no es `\w`. El
  conjunto de fallos salía **siempre vacío**. Comprobado sobre la línea real:
  `regex VIEJO captura: []` / `regex NUEVO captura: ['release_cmd::tests::r4_…']`.
- No distinguía «los tests pasaron» de «el crate no compiló»: `cargo test` sale
  con código distinto de cero en ambos casos, y un binario que no compila no
  imprime ninguna línea `FAILED`.

Consecuencia medida: la primera pasada reporto **cuatro** detecciones como ninguna.
Aplicadas a mano, **M3 la detectaba R4 correctamente** — el guard estaba bien y
el instrumento era ciego.

Un instrumento ciego que **admite** que no encuentra nada se descubre. Uno que es
ciego y reporta `DERIVA — detectada igualmente` **afirma que un guard notó algo
que nadie notó**, que es el peor modo de fallo posible.

Corregido: el regex captura `[\w:]+` y reduce al último segmento; una mutación
que no compila es `SKIP` **con su causa** y nunca `DERIVA`; y dos mutaciones se
reescribieron hasta que compilaron, porque un `SKIP` sin explicación es una
excusa.

## Lo que NO se verificó, y se declara

- **La ruta NO se ejecutó contra un GitHub real.** `pr.create`, `pr.merge` y
  `create.release` son tres escrituras privilegiadas sobre un repositorio ajeno,
  y AGENTS.md §1 lo prohíbe. Sigue siendo un `NOT_RUN` declarado: **este ciclo
  entrega que deje de ser imposible comprobarla, que es condición necesaria y no
  suficiente.** Que la ruta *funcione* contra GitHub real no se sabe.
- **No se tocó la ruta local**, que está cubierta y es la que usa
  `scripts/release.sh`.
- **No se regeneró el manifest**: ninguna superficie del bundle se tocó.
- **La release 2.5.3 no se construye ni se publica** — sigue bloqueada por la
  clave KMS, que es del operador.

## Commits

```
034d098a  test(release): R1, R3 y R5 en ROJO para la ruta forge; R4 verde por diseño
a6dfb5f2  fix(release): la ruta forge se ejecuta bajo prueba, y R2 la ejecuta con un doble
```

Contexto **real** en todas las verificaciones: `MockForge` es un doble en
memoria, no una simulación de resultados, y `R2` comprueba que el doble
**registró** el release — un outcome plausible sin efecto real no lo registra y
por tanto no pasa.
