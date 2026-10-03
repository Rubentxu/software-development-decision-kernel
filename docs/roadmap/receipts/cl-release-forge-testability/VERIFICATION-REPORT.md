# VERIFICATION-REPORT — cl-release-forge-testability

**Cycle:** `p-63676b11dc0ef88f/cl-release-forge-testability` (`OPEN/verify`)
**Subject SHA:** `13076dda` (código del ciclo en `a6dfb5f2`)
**Workspace:** 2.5.3 declarada, no publicada (último tag remoto `v2.5.2`)
**Date:** 2026-10-03

> El `cycle_id` de este ciclo lleva el prefijo `cl-`, a diferencia de
> `ledger-export-total` y `ledger-watch-total`. El slug desnudo da
> `STORAGE_NOT_FOUND`, y no es un fallo del ciclo: es el identificador.

---

## §1 — Qué se afirma, y qué no

**Se afirma:** que el cuerpo de `release apply --route forge` se ejecuta entero
bajo un doble, sin publicar a un repositorio real, y que al hacerlo no se ha
relajado ningún control de la ruta.

**No se afirma:** que la ruta forge funcione contra GitHub. **No se ejecutó
contra un repositorio real**, y queda `NOT_RUN` declarado en §7. Lo que se
verifica es una propiedad del código —el seam existe y la cadena corre bajo
prueba— que antes no era observable.

## §2 — La premisa, medida

Dos comentarios del propio código afirmaban que esta ruta «no tiene test» y que
«no es alcanzable sin red». La primera era cierta. La segunda no, y medirse
costó una superficie entera:

| Afirmación | Medido |
|---|---|
| «no tiene test» | cierta: ningún test alcanzaba la rama |
| «no alcanzable sin red» | **falsa**: `GitHubForge::with_runner` ya existía (`forge.rs:133`), `plan_release` y `apply_release` ya tomaban `&mut dyn Forge`, y `MockForge` ya era `pub` y re-exportado |

El mecanismo de inyección estaba entero. Faltaba **el seam en el call site**:
`release_cmd.rs:882` construía `GitHubForge::new(repo)` con el runner real. «No
se puede probar» no es «no se ha conectado para poder probarse».

## §3 — Falsificación: 5 mutaciones, 5 detectadas

`15-falsify-forge.py` muta el producto en la dirección de cada modo de fallo y
exige que caigan los guards. Restaura **por bytes**; el sha del fichero vigilado
se comprueba al terminar.

| Mutación | Cae |
|---|---|
| M1 el cuerpo vuelve a la rama | **R3** (+R1) |
| M2 la funcion devuelve el outcome sin aplicar la cadena | **R2** (+R4) |
| M3 `authorize_release` deja de exigir `pr.merge` | **R4** |
| M4 `apply_release` sale del `AdmissionTicket` | **R4** |
| M5 el doc vuelve a afirmar que la ruta no tiene prueba | **R5** |

**M2 es la que más importa**: es el modo de fallo de un test decorativo —una
envoltura que devuelve un outcome plausible sin ejecutar nada—, y R2 lo
distingue exigiendo que el doble **registre** el release publicado.

## §4 — Perfil completo

Toda cifra de esta tabla se ejecutó para este informe, no heredada del lote de
implementación.

| Comprobación | Resultado |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5414 passed / 0 failed / 24 ignored / 283 binarios** |
| `cargo test -p sddk-cli` | **1461 passed / 0 failed / 3 ignored / 62 binarios** |
| `cargo test -p sddk-cli --lib release_cmd` | **15 passed / 0 failed** |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `tests/test_changelog_coverage.sh` | **PASS=69 FAIL=0** |
| `tests/test_debt_index_coherence.sh` | **PASS=12 FAIL=0** |
| `15-falsify-forge.py` (5 mutaciones) | **5/5 DETECTADA**, 0 no medibles, exit 0 |
| scanner de caracteres no latinos | CLEAN |

La aritmética del workspace cierra sola: base 5409 / 24 / 283, más **5 tests**
(R1–R5, todos in-module en `release_cmd.rs`) → 5414 / 24 / 283. En el crate, base
1456 más esos mismos 5 → **1461**. Ningún verde fue reescrito: los 283 binarios
del workspace y los 24 ignorados son los mismos.

## §5 — Un error aritmético del propio recibo, corregido

`RECEIPT.md` decía «los 11 tests previos del módulo». Medido por commit:

| Commit | `#[test]` en `release_cmd.rs` |
|---|---|
| `034d098a~1` (antes del ciclo) | **10** |
| `034d098a` (lote 1) | 14 — añade R1, R3, **R4** y R5 |
| `a6dfb5f2` (lote 2) | 15 — añade R2, y renombra R3 |
| `HEAD` | 15 |

La base es **10**, no 11, y el lote 1 añade cuatro tests, no tres: R4 entró
ahí, verde por diseño, porque es el guard de no-regresión. Los diez nombres
originales siguen presentes, idénticos, en `HEAD`: la afirmación de fondo —ningún
verde reescrito— sí es cierta; la cifra no lo era.

## §6 — Los cuatro instrumentos que fallaron, y dónde estaba el defecto

Ninguno estaba en el producto. La regla del repo es que un FAIL de un
falsificador se corrige en el guard, y cuando el FAIL es del falsificador, en el
falsificador.

1. **El falsificador era ciego y no lo decía.** Usaba `^test (\w+) \.\.\. FAILED`,
   pero `cargo test` imprime el nombre completo con `::`, que no es `\w` —el
   conjunto de fallos era **siempre vacío**. Medido: `regex VIEJO captura: []`,
   `regex NUEVO captura: ['release_cmd::tests::r4_…']`. No distinguía «los tests
   pasaron» de «el crate no compiló». Consecuencia: la primera pasada reportó
   **cuatro** detecciones como ninguna. La mutación M3 aplicada a mano la
   detectaba R4 correctamente: el guard estaba bien.

2. **Un guard que contradecía el DISEÑO.** R3 afirmaba que el brazo no puede
   construir `GitHubForge::new`; el diseño dice que sí, y que delega. Un guard
   que contradice el diseño obliga a romper uno de los dos.

3. **Un guard que se detectaba a sí mismo.** R5 buscaba en todo el fichero
   frases que su propio doc comment contiene.

4. **R4 medía lo que tenía al lado**: la lista de **pasos** del plan en vez de
   la de **capacidades**, y el `use` de la cabecera en vez de la llamada al
   ticket. Ahora ancla la **función** y la **llamada**.

Más un quinto defecto, en los documentos y no en el código: `04-req-testable.py`
devolvió objetivos vacíos porque el SCOPE-CONTRACT escribía `**O1** —` donde el
instrumento lee `1. **O1.**`. Se corrigieron los documentos.

## §7 — Lo que NO se verificó, y se declara

- **La ruta forge no se ejecutó contra un GitHub real.** `NOT_RUN`. Son tres
  escrituras privilegiadas sobre un repositorio ajeno, y AGENTS.md §1 prohíbe la
  cero intrusión. El binario, el repo, las credenciales y la red son reales: la
  decisión es del operador.
- **No se verificó la instalación.** La release 2.5.3 no se ha construido ni
  publicado: sigue bloqueada por la **clave KMS**. Ningún resultado de este
  informe depende de la release.
- **No se regeneró `MANIFEST.sha256`**: el ciclo no toca ninguna superficie del
  bundle (`agents/`, `skills/`, `prompts/sddk/`, `assets/`, `specs/`).
- **No se ejecutó CI en la nube**, por la regla del repo: el gate es local y el
  cloud no bloquea.
- **Los instrumentos no viven en el repo.** `15-falsify-forge.py` está en
  `/var/home/rubentxu/f63/`, fuera del árbol versionado. Quien lea este informe
  puede re-ejecutar los tests —que sí están en el repo— pero no la falsificación
  sin ese fichero. Es una limitación de la evidencia, no del producto, y se dice
  aquí en vez de dejarse implícita.
- `pipelinek validate` es **NO_APPLICABLE**: no hay script `.kts` en este repo.

## §8 — Deuda

**Este ciclo introduce cero deuda**, y no por declaración sino por medición, con
cinco criterios objetivos sobre el diff `034d098a~1..HEAD`:

| Criterio | Medido |
|---|---|
| dependencias tocadas | **0** |
| `#[allow(...)]` nuevos | **0** |
| marcadores `TODO` / `FIXME` / `HACK` / `XXX` | **0** |
| `unimplemented!` / `todo!` nuevos | **0** |
| código alcanzable-nunca | **0**: `apply_release_forge` se llama desde el brazo (`:866`) y desde R2 (`:2439`) |

Las 369 líneas añadidas se reparten **87 en producción** y **282 en tests**. Las
87 de producción son el brazo, el cuerpo extraído y su documentación: es código
movido, no lógica nueva.

Sin deuda nueva que clasificar, `debt-severity-assigned` y
`debt-priority-assigned` se cumplen con esta medición como evidencia, y no
inventando un INC para tener algo que clasificar. El ciclo **tampoco cerró**
ningún INC: su hallazgo —las dos afirmaciones falsas del propio código— se
corrigió dentro del fichero y R5 lo fija, pero no era un ítem del índice de
deuda.

## §9 — Contexto

**Real, no simulado.** El doble es `MockForge`, el `Forge` real del gateway; el
ticket es el `AdmissionTicket` de verdad (ADR-0132), y R4 exige que `apply_release`
siga **dentro** de él. El fallo de lockstep de la primera ejecución
(`VERSION LOCKSTEP ERROR: no known manifest found`) no era del entorno: era el
lockstep fallando cerrado, que es su comportamiento correcto.
