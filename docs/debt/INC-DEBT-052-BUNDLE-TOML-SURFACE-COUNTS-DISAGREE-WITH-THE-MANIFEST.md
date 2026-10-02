---
id: INC-DEBT-052-BUNDLE-TOML-SURFACE-COUNTS-DISAGREE-WITH-THE-MANIFEST
title: "`BUNDLE.toml` declara `prompts_count = 0` en un bundle que publica 44 prompts"
status: resolved
resolved_at: 2026-10-01
resolved_in_session: session-65b
severity: medium
priority: P1
detected_at: 2026-10-01
detected_in_session: session-65b
component: sddk-dev-manifest
surface: crates/sddk-cli/src/dev/manifest.rs
fingerprint: "bundle_toml_surface_counts_disagree_with_the_manifest"
---

# INC-DEBT-052 — `BUNDLE.toml` declara `prompts_count = 0` en un bundle que publica 44 prompts

> **Nota de formato (session-65j).** Este documento declaraba su estado como
> `**status:** resolved (session-65b)` en markdown bold. Ese no es ninguno de los
> dos dialectos que `scripts/check_debt_index_coherence.sh` sabe leer: ni
> frontmatter YAML, ni `**Estado:**`. El guard lo reportaba como
> `unreadable: sin frontmatter y sin **Estado:**` y salía con **exit 1** — y
> llevaba así desde session-65b sin que nadie lo notara, porque el guard no está
> cableado en ningún runner (ver el addendum del final).
>
> La conversión a frontmatter **no cambia la afirmación**: `resolved` sigue
> siendo `resolved`. Lo que cambia es que ahora es legible por máquina, que es
> literalmente lo que el `doc_status` del guard dice que es el contrato
> ("frontmatter is the machine contract").

## Qué declara el artefacto y qué contiene

Regenerado `BUNDLE.toml` con el binario 2.4.2 instalado, sobre este repo:

```toml
[contents]
agents_count  = 73
skills_count  = 244
prompts_count = 0        # ← el bundle publica 44 ficheros bajo prompts/sddk/
assets_count  = 18
```

Contraste contra el propio `MANIFEST.sha256` que el mismo comando genera:

```text
72 agents · 17 assets · 44 prompts · 244 skills   (lo que se lista)
73 agents · 18 assets ·  0 prompts · 244 skills   (lo que se declaraba)
```

Dos defectos, no uno.

## Defecto 1 — el brazo del `match` nunca dispara

`crates/sddk-cli/src/dev/common.rs:10`:

```rust
pub(crate) const MANIFEST_SURFACES: [&str; 4] = ["agents", "skills", "prompts/sddk", "assets"];
```

`crates/sddk-cli/src/dev/manifest.rs`, función `count_surface_entries`:

```rust
match surface {
    "agents" => counts.agents_count = count,
    "skills" => counts.skills_count = count,
    "prompts" => counts.prompts_count = count,   // ← la superficie se llama "prompts/sddk"
    "assets" => counts.assets_count = count,
    _ => {}                                      // ← se traga el desajuste sin decir nada
}
```

La comparación es literal y `"prompts"` no es `"prompts/sddk"`. El brazo no se
ejecuta jamás y `_ => {}` convierte el desajuste en silencio. **`prompts_count`
ha valido 0 en todos los `BUNDLE.toml` que esta herramienta ha escrito**, no
en este repo solamente.

## Defecto 2 — dos recorridos distintos del mismo conjunto

El manifest se construye con `manifest_entries` (que dentro de un worktree usa
`git ls-files`, así que lista 72 agents: excluye
`agents/.atl/.skill-registry.cache.json`, que está sin trackear). Los conteos
venían de `count_files_recursive`, un `read_dir` independiente que sí cuenta
ese fichero: 73. Lo mismo con `assets` (18 vs 17).

El artefacto, por tanto, **no describía lo que contenía**: ni por el valor
ausente (prompts) ni por el valor inflado (agents, assets).

## Por qué nadie lo detectó

- Los campos `*_count` **no los lee nadie**. Lo verificado:
  `verify_manifest_anchor` (`bundle_manifest.rs:229`) sólo valida
  `contents.manifest_sha256`; una búsqueda de `.contents` en todo `crates/`
  devuelve el productor, el serializador y ese validador, y el validador no
  toca los contadores. No hay ningún consumidor que pueda delatar el 0.
- Los tests de `bundle_manifest_tests.rs` construyen un `ContentsSection` a
  mano (`agents_count: 142, prompts_count: 18, …`) y comprueban que se
  **round-trip**-ea. Un round-trip demuestra que el campo vuelve; no que el
  número sea cierto. Es el mismo patrón que `properties.rs:20` upholding
  `f(x) == f(x)` (session-64) y que el test de conformidad de C3l.7 que hacia
  `skip` y reportaba `ok`.
- El `BUNDLE.toml` commiteado (2.3.2) **no tenía sección `contents` con
  conteos**, así que el 0 no se veía reading el repo. Sólo aparecía al
  regenerarlo.

## Gravedad: por qué es medium y no high

No hay rotura funcional. `sddk dev install` valida `schema_version`, el
rango de binario y el ancla `manifest_sha256`; los conteos son declarativos y
no participan. El daño es que un artefacto **afirma** una cosa y es otra —
la misma familia que el gate de cobertura del CHANGELOG (INC-DEBT-047) o el
veredicto de conformidad de C3l.7.

Se eleva a **high** en cuanto algo consuma esos campos. La razón de
registrarlo con su alcance real, y no inflado, es que un incidente cuyo
número real se exagera pierde valor de triage para el siguiente.

## Cómo se encontró

Por un barrido de los 27 gates shell de `tests/`, no por leer el código. El
barrido salió con dos rojos:

- `test_release_bump_derivation.sh` (PASS=0 FAIL=7) — **otro defecto**, en
  el propio test: su función `derive` hace `rm -rf` y devuelve el tag por
  stdout; el mensaje del borrado se concatena al valor de retorno y la
  comparación falla. Verificado también en el baseline `ff10f662`.
- `test_dev_install_source_guard.sh` — este. Fallaba por deriva de versión
  (`BUNDLE.toml` en 2.3.2 contra workspace 2.5.0, desde la sesión que bumpeó
  a 2.5.0). **Regenerar para arreglar la deriva destapó el `prompts_count =
  0`**, que llevaba ahí desde que se añadieron los campos.

Es la segunda vez en esta sesión que **arreglar un síntoma revela el defecto
de fondo**, y la segunda que el fallo real estaba en lo que nadie probó.

## Corrección

`count_surface_entries` ya no recorre el sistema de ficheros: **lee el
`MANIFEST.sha256` que se acaba de escribir** y cuenta por prefijo de
superficie. Los conteos pasan a describir el artefacto por construcción, y
desaparece el segundo recorrido que podía divergir.

Además, una superficie sin campo donde incrementar **falla en voz alta** en
vez de escribir 0:

```rust
other => anyhow::bail!(
    "MANIFEST_SURFACES declares the surface {other:?} but ContentsSection has no \
     field for it; add the field and its arm here instead of shipping a 0 count"
),
```

Que el defecto original ahora sea un **error** y no un 0 silencioso es
precisamente lo que evita la reincidencia: el mismo desajuste que pasó
desapercibido ahora rompe la ejecución.

**Resultado tras regenerar con el binario corregido:**

```toml
agents_count  = 72
skills_count  = 244
prompts_count = 44
assets_count  = 17
```

Idéntico al contenido real del manifest.

## Pruebas

3 tests nuevos en `crates/sddk-cli/src/dev/tests/manifest_tests.rs`:

- `surface_counts_tally_prompts_that_the_manifest_lists` — dos ficheros bajo
  `prompts/sddk/`, incluido uno anidado, deben contarse 2.
- `surface_counts_describe_the_manifest_not_the_filesystem` — con un fichero
  sin trackear que el manifest no lista, la suma de los conteos debe igualar
  las líneas del manifest.
- `surface_counts_fail_closed_without_a_manifest` — sin manifest, error; no
  un cero.

**Falsificador OBSERVED:** reintroduciendo el defecto original (brazo
`"prompts"` + rama silenciosa), el test falla — y falla por la rama
fail-closed, que dispara **antes** de que el 0 llegue a escribirse. Eso es un
resultado mejor que el previsto: el guard falla donde antes no fallaba nada.

## Falsificadores exigidos si vuelve a tocarse

- **F68** contar por recorrido de filesystem en vez de por manifest → los tres
  tests deben fallar.
- **F69** añadir una superficie a `MANIFEST_SURFACES` sin campo en
  `ContentsSection` → debe abortar, no escribir 0.
- **F70** cambiar el formato de línea del manifest (separador) →
  `surface_counts_describe_the_manifest_not_the_filesystem` debe fallar.

## Addendum session-65f — el riesgo residual queda cerrado

Esta entrada dejó anotado que el fail-closed de `other => anyhow::bail!` cubre
la **deriva de contenido** pero no la **deriva de esquema**, y que cerrarla
exigía *«un test que compare `MANIFEST_SURFACES` con las claves de
`ContentsSection` por reflexión o por lista explícita; queda anotado, no
implementado»*.

**Implementado en session-65f**, y no por casualidad: al añadir `specs` como
quinta superficie del bundle. Es exactamente la operación que el riesgo
residual describía, y hacerlo sin el guard habría reproducido el defecto en su
forma nueva.

`tests/test_bundle_surface_coverage.py` fija las **cuatro** copias del
contrato, que es una más de las que el recibo llega a enumerar:

| copia | fichero |
|---|---|
| lista de superficies | `crates/sddk-cli/src/dev/common.rs` |
| contador por superficie | `crates/sddk-cli/src/dev/bundle_manifest.rs` (`ContentsSection`) |
| brazo de conteo | `crates/sddk-cli/src/dev/manifest.rs` (`count_surface_entries`) |
| lo que se empaqueta | el `tar` de `scripts/release.sh` **y** el de `.github/workflows/release.yml` |

**El hallazgo que el guard hizo al construirse:** los nombres de superficie y de
campo **no se corresponden**. La superficie es `prompts/sddk` y el campo es
`prompts_count`. La primera versión del guard derivaba el campo del nombre de la
superficie —`prompts/sddk` contra `prompts/sddk_count`— y por eso **falló en
verde sobre el propio repo sano**: que era, literalmente, el mecanismo del
defecto original reproducido en el instrumento. La forma correcta es una tabla
explícita `SURFACE_TO_FIELD`, y el guard exige que las dos mitades del contrato
queden cubiertas por ella en ambos sentidos.

**Falsificadores, 6 mutaciones OBSERVADAS** — las seis detectadas, y el árbol
restaurado en verde al terminar:

| # | mutación | detectada por |
|---|---|---|
| M1 | quitar `specs` de `MANIFEST_SURFACES` | superficie sin contador |
| M2 | añadir una superficie sin campo en `ContentsSection` | **F69, el que esta entrada exigía** |
| M3 | quitar el brazo de conteo de `specs` | `count_surface_entries` lo abortaría |
| M4 | quitar `specs` de un solo `tar` | el bundle local omite la superficie |
| M5 | quitar `specs` del otro `tar` | local y cloud publican distinta cosa |
| M6 | cambiar la tabla del guard sin tocar el código | tabla obsoleta |

M4 y M5 son la mitad que el recibo no enumeró: la deriva **entre las dos rutas
de producción** habría producido un release cloud distinto del que se prueba en
local, desde el mismo commit.

**Lo que sigue sin cubrir, y se declara en vez de omitirse:** el guard ata las
cuatro copias por *nombre*, no por *significado*. Si mañana alguien renombra
`prompts/sddk` y renombra el campo a la vez, las cuatro copias siguen
coherentes entre sí y la superficie cambia de significado sin que nada se queje.
El comentario de la constante lo dice, para que quien lo lea sepa qué protección
tiene y cuál no.

---

## Addendum session-65j: el guard llevaba rojo desde session-65b y nadie lo ejecutaba

### Verificación previa: el contenido de esta entrada es cierto

Antes de tocar nada, se contrastó el claim contra el árbol real. Los conteos de
`[contents]` en el `BUNDLE.toml` actual **cuadran exactamente** con
`MANIFEST.sha256`:

| superficie | `BUNDLE.toml` | entradas en el manifest |
|---|---|---|
| `agents` | 72 | 72 |
| `skills` | 245 | 245 |
| `prompts` | 44 | 44 |
| `assets` | 17 | 17 |
| `specs` | 14 | 14 |
| `impeccable_reference` | 2 | 2 |

(`skills` sube de 244 a 245 por la sexta superficie de session-65h, no por una
regresión.) **El arreglo de session-65b se sostiene.**

### El hallazgo: dos defectos encadenados

**Defecto 1 — el documento declaraba su estado en un dialecto ilegible.** Este
fichero usaba `**status:** resolved (session-65b)` en markdown bold. El guard
`scripts/check_debt_index_coherence.sh` lee exactamente dos dialectos:
frontmatter YAML (`^status:`) y la prosa `**Estado:**`. El nuestro no era
ninguno de los dos, así que lo reportaba como
`unreadable: sin frontmatter y sin **Estado:**` — **fail-closed, correctamente**.
Lo grave no es que el guard fallara: es que **llevaba fallando desde
session-65b** y nadie se enteró.

**Defecto 2 — el guard no estaba cableado en ningún sitio.** Referenciado solo
por su propio test:

- `ci.yml:46` hace `shellcheck` de `scripts/*.sh` y `tests/test_*.sh` — eso es
  **lint**, no ejecución.
- `scripts/release.sh:242` corre `tests/test_debt_index_coherence.sh`, que
  monta un **árbol desechable por caso** vía `SDDK_DEBT_DIR`.
- Nadie ejecutaba `check_debt_index_coherence.sh` contra el repo real.

Sus **10 casos pasaban**. Ese verde es lo que hacía el defecto invisible: leía
como cobertura y no lo era. Es la forma del INC-DEBT-033 **un nivel más
hondo** — el propio header del guard advierte que *"un test que no puede mover
el sujeto bajo test no puede falsificarlo"*, y aun así el suite entero pasaba
porque pasaba contra fixtures.

### Método: casi se diagnostica al revés

La primera hipótesis fue **incorrecta**: que el guard era ciego al dialecto y
por eso no veía esta entrada. Se casió «arreglar» el guard para que aceptara el
dialecto. La evidencia lo refutó: `grep -m1 "^status:"` sobre el documento sale
con **código 1**, y al ejecutar el guard real contra el repo sale **exit 1**
señalando precisamente esta entrada.

**Lo que se había ejecutado antes era el TEST, no el GUARD** — y el test hace
exactamente lo que su propio comentario dice que no debe hacer: probar contra
fixtures y no contra el sujeto real. Un PASS=10 FAIL=0 leído como «el guard
funciona» es exactamente el falso positivo que este repo lleva tres slices
persiguiendo.

### Corrección

1. **El documento se ajustó al contrato”**, no al revés: frontmatter YAML
   canónico. `resolved` sigue siendo `resolved`; lo único que cambia es que
   ahora es legible por máquina.
2. **El guard se ejecuta en `release.sh`**, junto al bucle de tests pero como
   paso propio y fail-closed.
3. **El test ata el cableado.** Una aserción nueva comprueba que
   `release.sh` invoca el guard. Sin ella, el cableado puede borrarse sin que
   nada se ponga rojo — que es como desapareció en primer lugar.

### Falsificadores: 2 mutaciones, 2 detectadas

| Mutación | Resultado |
|---|---|
| Guard acepta `**status:**` (ablandar el contrato) | `[FAIL] status in an unknown dialect is unreadable` (esperaba exit 1, obtuvo 0) |
| Cableado borrado de `release.sh` | `[FAIL] release.sh no invoca el guard` |

La primera es la que más dice: **ablandar el guard para absolver al documento
está prohibido por el propio test**. La dirección correcta es el documento se
ajusta al contrato. Un guard que acepta cualquier dialecto no está siendo
 tolerante, está dejeando de medir.

**Resultado:** test **12/12**, guard real **41/41** sobre el repo (antes 40/41),
`shellcheck` limpio, y el guard pasa a ejecutarse en cada release.
