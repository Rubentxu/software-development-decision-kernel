# RECEIPT — CL-release-version-source, lote 1 (el contrato en el engine)

**Fecha:** 2026-10-02T11:15:00Z
**Commits:** `a9c5bb5a` (ADR-0153 + SCOPE + PRE-FLIGHT), este commit (lote 1)
**Superficie:** `crates/sddk-engine/src/version_source.rs` (nuevo),
`version.rs` (un call site), `lib.rs` (registro del módulo).
**Riesgo de datos:** cero. Ningún ledger, ningún storage, ninguna release.

## 0. La deuda estaba vigente, y se reprodujo

El goal exige no tratar como deuda una alerta caducada. Reproducido con el
binario 2.5.3 compilado de este checkout, sobre el repo real:

```text
$ sddk release plan --root /var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin --tag v0.45.0
error: VERSION LOCKSTEP ERROR: could not read
  .../pipeline-kotlin/Cargo.toml: No such file or directory (os error 2)
exit 1
```

El repo tiene `build.gradle.kts`, `settings.gradle.kts`, `gradle.properties` y
`gradle/`, y **no** tiene `Cargo.toml`.

## 1. Criterios del ADR, uno a uno

| # | criterio | resultado |
|---|---|---|
| 1 | `Cargo.toml` no aparece en `version.rs` | **OK** — verificado por `awk` sobre el código de producción: las dos coincidencias restantes son doc comments. El nombre vive solo en `version_source.rs` |
| 2 | Los ocho ecosistemas resuelven o fallan por una razón declarada | **OK** — 8 tests, uno por ecosistema, sobre fixtures |
| 3 | Añadir un ecosistema es **solo datos** | **OK**, y con un matiz importante (§3) |
| 4 | Un repo Rust se comporta igual que antes | **OK con un cambio de redacción declarado** (§4) |
| 5 | Dos manifiestos con versiones distintas ⇒ error duro que nombra ambos | **OK** |
| 6 | Un manifiesto corrupto ⇒ error duro, sin caer al siguiente candidato | **OK** — el criterio central |
| 7 | Go y Bazel ⇒ `TagIsTheOnlyAuthority`, distinto de `CrossChecked` | **OK** |

```text
cargo test -p sddk-engine --lib            1397 passed; 0 failed; 1 ignored
cargo test -p sddk-engine --lib version       50 passed; 0 failed
cargo fmt --check -p sddk-engine             limpio
cargo clippy -p sddk-engine --all-targets -- -D warnings   limpio
```

## 2. Lo que un repo real enseñó que ningún fixture habría dado

Terminado el contrato completo y falsificado, la reproducción sobre
`pipeline-kotlin` cambió de fondo: el error pasó de «no encuentro `Cargo.toml`» a **«no encuentro `version`
en `gradle.properties`»**. Ya no busca un fichero inexistente por una razón
falsa: encuentra el manifiesto correcto del proyecto y dice la verdad.

Pero el release **seguía sin poder salir**, y ahí estaba el hallazgo:

- `core/build.gradle.kts:10` tiene `version = "1.0-SNAPSHOT"` — un placeholder,
  en un subdirectorio, en un monorepo.
- El `gradle.properties` raíz **no declara versión**.
- `git describe` → `v0.46.0-46-g60dad30e`. **La convención real del proyecto es
  el tag.**

O sea: hay un **tercer estado** que el contrato de dos no cubría. No es «el
ecosistema no puede declarar versión» (Go, Bazel) ni «no hay manifiesto». Es
«el manifiesto existe y este proyecto no declara versión en él, porque su
convención es otra».

No se degradó ese caso a `TagIsTheOnlyAuthority` por inferencia, porque
entonces un `Cargo.toml` de Rust al que se le quite la `version` publicaría en
verde. Se añadió la **declaración explícita** que el ADR ya describía en su §4
—`.sddk/version-source.json`—, con tres límites que son el contenido del
rescate:

- Rescatable: `NotDeclared` y `NoSource`. Es decir, *nadie declaró versión*.
- **No** rescatable: `Unreadable` ni `Unparsable`. Un manifiesto que no se lee
  puede estar escondiendo algo; declararlo no lo repara.
- **No** rescatable: `Divergent`. Si dos manifiestos discrepan, eso es una
  pregunta humana y una declaración no la responde.
- Y no se consulta si hay versión: una declaración es un **rescate**, no una
  prioridad.

Cinco tests fijan esos límites, incluido `a_declaration_does_not_rescue_a_broken_manifest`.

## 3. La mutación que escapó, y el test estructural que la cazó

**12 mutaciones. La novena no la detectaba nada.**

La mutación reintroducía `if spec.ecosystem == "rust" { … }` en el bucle de
resolución: un `if` por ecosistema, exactamente la lista codificada que
ADR-0153 dice evitar. **Dejó la suite verde, 44 passed / 0 failed.**

La causa es que la mutación es **comportamentalmente idéntica**: da la misma
respuesta para todos los casos que los tests ejercitan. Ningún test de
comportamiento puede detectarla, porque no cambia comportamiento. La propiedad
que el ADR quiere proteger es **estructural** —los ids de ecosistema viven en el
registro y en ningún otro sitio— y eso solo se puede afirmar leyendo la fuente.

Solución: `ecosystem_ids_exist_only_inside_the_registry`, que hace
`include_str!` de los dos ficheros, recorta el bloque `REGISTRY` y el módulo de
tests, y falla si un id aparece en el resto. Re-falsificado: **detectada**, y
rompe precisamente ese test.

En su primera ejecución el propio test encontró dos cosas: un error de índices
míos al recortar el bloque (`begin <= end`), y que no excluía su propio módulo
de tests —que nombra ecosistemas a propósito. Ambas corregidas.

## 4. Un test cuyo texto cambié, y por qué

`lockstep_errors_when_cargo_toml_missing` afirmaba `err.message.contains("could
not read")`.

**El comportamiento no cambió: sigue fallando cerrado.** Lo que cambió es que
`could not read` era una redacción **incorrecta**: en un repo sin `Cargo.toml`
no falló ninguna lectura; el fichero no está, que es el caso normal en un
proyecto Gradle. El mensaje nuevo lista los ocho manifiestos buscados.

La aserción era sobre redacción; la que lo sustituye es sobre el hecho y es más
específica: falla cerrado, nombra la raíz, y nombra tanto `Cargo.toml` como
`go.mod` — porque lo que se busca ya no es solo Rust. El cambio está escrito en
el propio test, con su motivo, para que no parezca un ajuste silencioso.

El otro mensaje sí se preservó: `could not find \`version\`` sale idéntico, y
sale **derivado de los datos** (la hoja del locator más sus prefijos), sin una
rama por ecosistema.

## 5. Prueba de extremo a extremo, con el binario real

```text
# antes de este trabajo, en el repo Gradle real
error: VERSION LOCKSTEP ERROR: could not read .../Cargo.toml: No such file or directory

# despues, el mismo repo (fixture que replica su forma; NO se escribio dentro
# del repo ajeno, por la regla de cero intrusion de AGENTS.md §1)
error: VERSION LOCKSTEP ERROR: could not find `version` of .../gradle.properties

# el mismo repo declarando que su version es el tag
route: local
branch: main
tag: v0.46.0
steps:
- push_main
- verify_main_sha
exit 0
```

## 6. Falsificación: 12/12

```text
broken_manifest_falls_through                 detectado (5 tests)
divergent_picks_the_first                     detectado
no_source_becomes_tag_only                    detectado (2 tests)
rust_takes_the_tag_only_path                  detectado (2 tests)
agreement_reported_as_single                  detectado
registry_loses_python                         detectado
tag_only_shadows_a_declaring_source           detectado
absent_conflated_with_unparsable              detectado (3 tests)
resolution_learns_an_ecosystem                detectado, tras el test estructural
declaration_rescues_a_broken_manifest         detectado
declaration_overrides_an_existing_version     detectado
malformed_declaration_ignored                 detectado
```

Una mutación no compilaba en su primera forma (un `if` antes de un brazo de
`match`): eso es una **mutación inválida**, no un hueco del test, y se corrigió
la mutación.

## 7. Lo que este lote NO hizo

- No tocó `release_cmd.rs`. El comentario de la línea 847 sigue diciendo
  «workspace Cargo.toml version» y `version_lockstep_passed = true` sigue fijo:
  ambos son el **lote 2**, que es donde `TagIsTheOnlyAuthority` tiene que llegar
  a la salida para que un release sin comprobación se distinga de uno con ella.
- No promovió ADR-0153 de `proposed` a `accepted`.
- No escribió en `/var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin` ni en
  ningún otro repo: la end-to-end se montó en un fixture temporal.
