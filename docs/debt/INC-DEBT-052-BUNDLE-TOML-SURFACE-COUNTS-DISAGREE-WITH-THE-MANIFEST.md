# INC-DEBT-052 — `BUNDLE.toml` declara `prompts_count = 0` en un bundle que publica 44 prompts

**severity:** medium
**priority:** P1
**status:** resolved (session-65b)
**detected:** session-65b, por barrido de gates (no por lectura)
**component:** `sddk dev manifest --bundle` · `crates/sddk-cli/src/dev/manifest.rs`

---

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
