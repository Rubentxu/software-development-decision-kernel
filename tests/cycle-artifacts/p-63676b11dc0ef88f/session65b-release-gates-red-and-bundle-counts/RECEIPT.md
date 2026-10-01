# RECEIPT — dos gates de admisión de release en rojo, y el `prompts_count = 0` que destaparon

**Slice:** `session65b-release-gates-red-and-bundle-counts`
**Fecha:** 2026-10-01 · **Baseline:** `3ac7da41` (HEAD == origin/main al abrir)
**Workflow:** `A-lite` · **Deuda:** INC-DEBT-052 (nueva, resuelta) · **Semver:** PATCH

---

## §1 Qué se cierra y qué no

Se cierran **dos gates de admisión de release que estaban rojos** y se registra
**INC-DEBT-052**. No se toca ningún contrato entre crates, ni el binario
publicado, ni la ruta de release.

El punto de partida no fue una lectura del código: fue un **barrido de los 27
gates shell de `tests/`**. Nadie los había ejecutado en bloque.

## §2 El barrido

```text
gates: 27 · verdes: 25 · rojos: 2
  test_release_bump_derivation.sh   exit=1
  test_dev_install_source_guard.sh  exit=1
```

`test_supply_chain_authenticity.sh` appeared rojo en el barrido y **no lo
era**: exige `--tag` y con `--tag v2.4.2` da **PASS=13 FAIL=0**. Es un
falso positivo del barrido, no un defecto; queda dicho para que el número
"27 gates" no se lea como "27 verdes".

## §3 Defecto 1 — el gate que se fijaba al valor de retorno (P0 para el gate)

`tests/test_release_bump_derivation.sh` daba **PASS=0 FAIL=7**, con
`expected v2.5.0, got mavis-trash: moved to trash: …`.

**Verificado como preexistente**: worktree limpio en `ff10f662` reproduce
`PASS=0 FAIL=7` idéntico. Mi hipótesis previa —registrada en el diario de
session-65— era que la captura se comía el **stderr** de `mavis-trash`.
**Es falsa, y el modo de fallo importa**: en este entorno `rm` se enruta a
`mavis-trash`, que escribe a **stdout**, no a stderr.

El mecanismo real es una fuga de valor de retorno:

```bash
out=$(cd "$dir" && bash scripts/release-bump.sh --dry-run 2>&1)
rm -rf "$dir"                 # imprime en stdout
echo "$out" | grep -oE …      # imprime el tag
```

Las dos líneas escriben en el stdout de la función, y el caller lee ese
stdout **como valor**. El mensaje del borrado se concatena al tag y
`[[ "$got" == "$expect" ]]` falla.

En CI `rm` no imprime, así que el gate pasaba. El defecto no era
"dependencia del entorno": era que la función **devuelve más de lo que
promete**, y cualquier limpieza que hable ruin el resultado.

**Corrección**: el valor derivado se calcula en una variable y se emite con
`printf`, y el `rm` se silencia explícitamente en los tres sitios (las dos
funciones y el `trap`), en vez de confiar en que el borrado es mudo.

**Falsificador OBSERVED**: worktree con la corrección revertida →
`PASS=0 FAIL=7`; con ella → `PASS=7 FAIL=0`.

## §4 Defecto 2 — `BUNDLE.toml` describía un bundle que no era el suyo

`test_dev_install_source_guard.sh` fallaba por **deriva de versión**:
`BUNDLE.toml` en 2.3.2 contra workspace 2.5.0, desde la sesión que bumpeó a
2.5.0. Verificado también en el baseline: ahí ya fallaba.

Regenerar `BUNDLE.toml` para arreglar la deriva destapó lo que había debajo:

```text
MANIFEST.sha256 dice:  72 agents · 17 assets · 44 prompts · 244 skills
BUNDLE.toml declaraba: 73 agents · 18 assets ·  0 prompts · 244 skills
```

**Un bundle que declara cero prompts y publica 44.** Dos causas:

1. `MANIFEST_SURFACES` nombra la superficie `prompts/sddk` y el `match` de
   `count_surface_entries` buscaba el literal `prompts`. El brazo **nunca**
   dispara y `_ => {}` traga el desajuste. `prompts_count` ha valido 0 en
   **todos** los `BUNDLE.toml` que la herramienta ha escrito.
2. Los conteos venían de un `read_dir` independiente del recorrido que
   genera el manifest, así que contaban ficheros que el manifest no lista
   (un caché sin trackear bajo `agents/`) y declaraban 73 y 18.

**Gravedad medium, y el motivo está medido, no supuesto**: los campos
`*_count` **no los lee nadie**. `verify_manifest_anchor` sólo valida
`contents.manifest_sha256`, y una búsqueda de `.contents` en todo `crates/`
devuelve el productor, el serializador y ese validador, que no los toca. No
hay rotura funcional: el daño es la declaración falsa. Se registra con su
alcance real y no inflado; se eleva a high en cuanto algo los consuma.

**Por qué nadie lo vio**: los tests de `bundle_manifest_tests.rs` construyen
un `ContentsSection` a mano y comprueban que **round-trip**-ea. Un
round-trip demuestra que el campo vuelve, no que el número sea cierto. Es el
mismo patrón que `f(x) == f(x)` en `properties.rs:20` (session-64) y que el
test de conformidad de C3l.7 que hacía `skip` y reportaba `ok`.

## §5 Corrección del defecto 2

`count_surface_entries` ya no recorre el disco: **lee el `MANIFEST.sha256`
recién escrito** y cuenta por prefijo de superficie. Los conteos pasan a
describir el artefacto por construcción y desaparece el segundo recorrido
que podía divergir. Una superficie sin campo donde incrementar **aborta**,
en vez de escribir 0.

```toml
agents_count  = 72     # antes 73
skills_count  = 244
prompts_count = 44     # antes 0
assets_count  = 17     # antes 18
```

Idéntico al contenido real del manifest, que no cambió: los 377 ficheros
tienen el mismo `manifest_sha256` (`0a79a8a5…`) antes y después.

## §6 Pruebas y falsificadores

3 tests nuevos en `crates/sddk-cli/src/dev/tests/manifest_tests.rs`:
`surface_counts_tally_prompts_that_the_manifest_lists` ·
`surface_counts_describe_the_manifest_not_the_filesystem` ·
`surface_counts_fail_closed_without_a_manifest`.

**Falsificador OBSERVED**: reintroduciendo el defecto original (brazo
`"prompts"` + rama silenciosa) el test falla — y falla **por la rama
fail-closed**, que dispara *antes* de que el 0 llegue a escribirse. Es un
resultado mejor que el previsto: el guard falla donde antes no fallaba nada.

**Falsificador del gate OBSERVED**: corrección revertida → `PASS=0 FAIL=7`;
corregido → `PASS=7 FAIL=0`.

## §7 GATES

```text
cargo test -p sddk-cli --lib            862 passed / 0 failed / 1 ignored
surface_counts                            3 passed
cargo fmt --all -- --check                limpio
cargo clippy -p sddk-cli --all-targets -- -D warnings   exit 0
barrido de gates shell                    27 · verdes 26 · rojos 1 (falso positivo)
test_dev_install_source_guard.sh          PASS  (estaba en rojo)
test_release_bump_derivation.sh           PASS=7 FAIL=0  (estaba en rojo)
test_debt_index_coherence.sh              PASS=10 FAIL=0
git diff --check                          limpio
```

**Alcance de test acotado y justificado**: el cambio vive en
`crates/sddk-cli/src/dev/manifest.rs` y no altera ningún contrato entre
crates — `BUNDLE.toml` conserva su `schema_version` y sus campos. El lote es
`sddk-cli`, no el workspace.

## §8 Lo que este slice NO hace

- **No corrige `test_supply_chain_authenticity.sh`**: no estaba roto; mi
  barrido lo invocó sin el `--tag` que exige.
- **No toca los conteos de los bundles ya instalados** en
  `~/.local/share/sddk/framework/2.4.2/`: son artefactos de una versión ya
  publicada y reescribirlos sería reescribir historia.
- **No ejecuta `cargo test --workspace`**: no justificado por este cambio.
- **No arregla `release plan` para proyectos no-Rust** (INC-DEBT-051, sigue
  abierta, requiere SCOPE + ADR).

## §9 Riesgo residual

El fail-closed de la rama `other => anyhow::bail!` protege del *mismo*
desajuste, pero no de que `MANIFEST_SURFACES` y `ContentsSection` se mantengan
en sincronía cuando **ambos** tienen el campo y la semántica se separaría
—por ejemplo, si una superficie pasara a listarse con prefijo distinto. Lo
que queda atado hoy es que los conteos se leen del manifest, lo cual cubre
la deriva de contenido pero no la de esquema. Cubrirlo exigiría un test que
compare `MANIFEST_SURFACES` con las claves de `ContentsSection` por
reflexión o por lista explícita; **queda anotado, no implementado**.
