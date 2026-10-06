# Cierre de `cl-publicar-lo-que-ya-esta-hecho` (VA13)

**Fecha:** 2026-10-06
**Estado:** cerrado y publicado

---

## Qué mide esto

Este bloque no añadió capacidad. **Publicó treinta y un commits que ya estaban
escritos, y una de ellas iba una frontera del dominio rota que nadie había visto.**

La segunda parte es la que importa, y no se habría encontrado sin lo primera.

---

## Lo que había acumulado

MEDIDO antes de decidir nada:

| | |
|---|---|
| commits sin publicar desde `v2.11.4` | **31** |
| ficheros de código (`crates/`, `scripts/`, `tests/`) | **43** |
| ADRs | **11** (0153, 0157–0166) |
| distribución | 7 `feat(release)`, 3 `feat(version)`, 2 `feat(target)`, 3 `fix(release)`, 1 `fix(version)`, 1 `fix(push)`, 2 `refactor(gateway)`, 1 `refactor(release)` |
| binario instalado vs checkout | `behind` — y **declara la misma versión**, luego la comparación sale verde |
| ciclos en `RELEASE_PENDING` | **6**, todos con `release.complete` bloqueado por `release-receipt` + `merge-receipt` |

Cuatro bloques seguidos de capacidad, ninguno liberado. Eso es exactamente el
big-bang que la regla 6 prohíbe, y se alcanzó no por decisión sino porque cada
bloque parecía urgente y el release quedaba siempre para el siguiente.

**Y la tentación concreta, declarada para no tomarla:** `AdapterFact` /
`EvidenceResolver` —532 líneas de producción sin consumidor con
`#[allow(unused)]`— es deuda real y sin riesgo. Es tentador porque es conocido.
Es la elección equivocada: deuda que no llega al usuario, capacidad que sí, detrás
de un tag. La deuda se queda declarada; la capacidad, no.

---

## Lo que encontró el perfil completo, y era mío

```
$ cargo test --workspace
test the_decision_module_names_no_concrete_technology ... FAILED
  crates/sddk-domain/tests/version_authority_fitness.rs:117
  el modulo de decision nombra tecnologia concreta: ["gradle"].
  El kernel define las preguntas; los providers saben obtener la evidencia.
```

Contra Git, sin ambigüedad:

```
$ git show 97dd9293:crates/sddk-domain/src/version_authority.rs | grep -ci gradle
0
$ git log --oneline -S"Gradle" -- crates/sddk-domain/src/version_authority.rs
1621c3fb feat(release): el proyecto declara su version y publica, …
```

`1621c3fb` es el primer commit de **VA11**, de hace dos bloques y **publicado en
`origin/main`**. El dominio tenía **cero** apariciones y pasó a tener una porque
escribí el nombre de la herramienta en el doc-comment de
`ProviderError::Malformed`, al explicar por qué esa variante hacía falta, citando
el ejemplo concreto que la motivaba.

### Por qué nadie lo vio, y esa es la parte que enseña

VA11 **tocaba `sddk-domain`** y ejecuté el perfil de gateway y CLI. VA12 no tocaba
el dominio y ejecuté gateway y CLI también. Los dos bloques se leyeron como «un
cambio de gateway».

`prompts/sddk/change-scoped-testing.md` manda razonar en
`ActiveChangeSet → SUT impact → verification batch`. Lo que hice fue razonar en
`→ crate que creía haber tocado → verification batch`. **La ley mal aplicada es
exactamente la que dice que no se aplique**: el SUT de VA11 incluía
`sddk-domain`, luego `cargo test -p sddk-domain` estaba en el lote. El perfil
correcto era el que no se corrió, porque el dato que lo decidía —el diff— se leyó
por el nombre del crate y no por su contenido.

### Y un hecho del propio comando que lo contiene

`cargo test --workspace` **sin `--no-fail-fast` para en el primer binario rojo**.
Luego ese 1 fallo podía estar escondiendo los que nunca se midieron. La corrida
que lo encontró se relanzó con `--no-fail-fast` por eso, y el total se mide sobre
esa: **workspace 5680/0 en 308 binarios**, que es el número que vale.

### Y lo que NO lo detecta

Nombrar una herramienta en un doc-comment **no rompe ninguna regla de tipos**.
Compilador verde, clippy verde. La frontera entre «el núcleo define las preguntas»
y «los providers saben obtener la evidencia» se erosiona un fraseado a la vez,
hasta ser ficción, y el único guard que lo ve es el que escanea el fuente.

El arreglo retira el nombre y **añade en su lugar por qué ese doc no lo nombra**,
para que la ausencia tenga motivo escrito y no parezca descuido el día que alguien
venga a completarlo.

---

## El segundo defecto, y lo produjo el ensayo

El `--dry-run` pasó los pasos 0–8 limpios. **Se salta el 2b** —cobertura de
changelog— porque no publica nada. Y el 2b corre **después** de construir.

```
$ bash tests/test_changelog_coverage.sh
PASS=19 FAIL=1
```

El commit de arreglo es un `fix`, y todo `fix` desde el último tag tiene que estar
representado en la sección declarada. No lo estaba.

**Un ensayo que se salta una puerta no ensaya esa puerta**, luego el `--dry-run`
tiene menos cobertura que la cosa que ensaya, y el hueco caía en el paso más caro
de revertir: uno que para después del build y antes de `gh release create`.

Lo que convirtió un fallo que habría costado un tag en una entrada de changelog
fueron dos mediciones separadas: el perfil completo con `--no-fail-fast` y el 2b
corrido por su cuenta.

---

## La release

```
$ bash scripts/release.sh --dry-run --skip-tests     # pasos 0-8, verde
$ bash scripts/release.sh                            # 0-13, con los contratos shell
```

**`--skip-tests` solo en el ensayo**, y escrito: el perfil completo acababa de
pasar en esta sesión sobre el mismo árbol, y el paso 1b —los ~40 contratos shell—
se paga una vez. En la release real **no se saltó**, porque lo que se publica es un
artefacto nuevo y esos contratos leen el artefacto.

Sin bumpear: el workspace declara `2.12.0` y el último tag es `v2.11.4`, luego la
próxima release **es `2.12.0`** (§2.3). MEDIDO:

```
$ release_admission_check_v2 HEAD
ACCEPT last-publish=2.11.4 -> 2.12.0
```

La admisión es semántica: comprueba que hay un cambio real y monótono de
`[workspace.package] version` por encima del último tag. Por eso no hizo falta el
commit ceremonial, y por eso el `pre-push` aceptó los cinco bloques anteriores sin
`--no-verify`.

## VERIFICACIÓN FINAL — la release 2.12.0 está publicada y instalada

Quinto intento del release real, y el primero que llegó entero. El cuarto murió en
el `8c`; lo que se midió y se decidió está en el `PRE-FLIGHT` (checkpoint 3). El
quinto relance con `SDDK_SKIP_SIGNING=1`, **sin ningún otro cambio**.

### Lo que pasó por verde, en orden

| Paso | Gate | Resultado |
|------|------|-----------|
| 0 | preflight, admisión | `ACCEPT last-publish=2.11.4 -> 2.12.0` |
| 1 | fmt + clippy + test workspace | 308 binarios, 0 fallos |
| 1b | 52 contratos shell | 52/52 |
| 2b | changelog coverage | `PASS=22 FAIL=0` |
| 3 | binario release musl | estático, build-id `8f494f54…` |
| 3b | reconciliación del artefacto | `PASS=16 FAIL=0` |
| 3c | autofalsación de la reconciliación | `PASS=10 FAIL=0` |
| 3d | los cuatro estados de `build_identity` | `PASS=16 FAIL=0` (O6 `NOT_RUN`) |
| 3e | receipt de frontera exigible | `PASS=14 FAIL=0` |
| 3f | el nombre no promete una frontera | `PASS=9 FAIL=0` |
| 3g | autofalsación de la política de nombres | `PASS=8 FAIL=0` |
| 3h | cita de spec anclada a un documento | `PASS=6 FAIL=0 SKIP=0` |
| 3i | conjunto cerrado de descarte | `PASS=5 FAIL=0 SKIP=0` |
| 3i-b | línea base del bump desde el remoto | `PASS=2 FAIL=0 SKIP=0` |
| 3j | vista del operador derivada | `PASS=4 FAIL=0 SKIP=0` |
| 3k | enumeración de decisiones | `PASS=5 FAIL=0 SKIP=0` |
| 3l | línea base publicada del 2b | `PASS=5 FAIL=0 SKIP=0` |
| 3m | el release nombra su propia causa de fallo | `PASS=20 FAIL=0 SKIP=0` |
| 3n | los siete `uat_ctx_*` contra el binario a publicar | 7/7 |
| 4 | `MANIFEST.sha256` | 395 ficheros, verifica |
| 5–7 | bundle, `BUNDLE.toml`, unificado | 679 KB + 12.692.388 B |
| 8 | checksums + sbom | presente |
| 8b | espejo del vault | 69 aceptados, 0 creados (idempotente) |
| **8c** | **firma** | **`SDDK_SKIP_SIGNING=1` declarado, sin firmar** |
| 9 | `gh release create` | publicada |
| 9b | public-release gate | tag SHA `f7d2118c`, **9/9 assets HTTP 200** |
| **9c** | **autenticidad** | **`NOT_RUN` con su motivo** |
| 10 | install desde la URL real | CDN convergiu a los 10 s |
| 11–13 | doctor, prune, round-trip | binario y bundle coherentes |

### El artefacto, medido desde fuera del script

```
$ gh release view v2.12.0 --json tagName,isDraft,isPrerelease,assets
tag: v2.12.0 | draft: False | prerelease: False | 2026-10-06T01:32:58Z
assets: 9        firmas .sig: 0

$ git ls-remote origin refs/tags/v2.12.0
f7d2118c9b77259df54a00be249d60a5b27eac3a   refs/tags/v2.12.0

$ sddk --version
sddk 2.12.0        -> /home/rubentxu/.local/bin/sddk

$ ls ~/.local/share/sddk/framework/
2.12.0    current
```

**9 assets y 0 firmas es exactamente el contrato de `v2.11.4`.** No se cambió el
contrato del artefacto publicado: se mantuvo.

### Lo que este release NO verifica, dicho por el propio release

`9c` no corrió, y no por un fallo: `signature files present: 0` luego **no hay
nada que verificar**, y el gate lo **declaró** en vez de dejarlo en silencio. La
integridad se verificó —el `sha256` del binario servido por el CDN y coincidente
con el local, `CHECKSUMS`, `sbom.json`, `MANIFEST.sha256` con 395 ficheros—. **La
autenticidad no se verificó**, y el instalador lo dice con esas palabras a quien
lo ejecuta:

```
warning: integrity (sha256) is verified, authenticity is NOT.
warning: a compromised download origin would be accepted.
```

Firmar requiere un KMS que esta máquina no tiene. Approvisionarlo cambia el ancla
de confianza de un artefacto ya publicado, y eso es una decisión de política, no
de automatización. Declarado, no forzado.

### El instrumento que se rompió a sí mismo, por quinta vez

Entre el quinto lanzamiento y el `1b` hubo un `ls` y un `tail` sobre el directorio
de este mismo recibo que devolvieron `No existe el fichero o el directorio`,
cuando el directorio existe, está en `git` y tenía 321 líneas. Repetido al
momento: verde. **Y la primera explicación que escribí —«el volumen montado
devuelve `ENOENT` transitorio bajo carga»— quedó falsada por el comando
siguiente**: el `cat >>` que escribía el checkpoint devolvió el mismo `ENOENT` y
**sí escribió** (321 → 413 líneas). Si el error fuera del sistema de ficheros, la
escritura no habría pasado.

Lo medido es que la misma ruta literal falla unas veces y funciona otras; **la
causa no está determinada** y queda así, sin explicar. Lo que sí se cambió, y es
lo único que se cambió: **todas las rutas de este bloque se resuelven con un glob
en lugar de una cadena escrita a mano**, y antes de dar por perdido un fichero
versionado se pregunta a `git ls-files`, que responde con la verdad sobre lo que
Git sabe.

---

## Lo que este bloque NO hizo, y por qué

- **No cerró los seis ciclos `RELEASE_PENDING`.** Lo había anunciado antes de
  empezar y sigo sin hacerlo, porque no era de este bloque. MEDIDO el motivo:
  `release.complete` exige **`merge-receipt`**, y `MergeReceipt` lo emite
  `Forge::merge_pr` — **un merge de pull request**. `AGENTS.md` §2.2 dice que este
  repo **no usa PRs**: `main` es la rama única y todo va directo. Esos seis ciclos
  se movieron a `RELEASE_PENDING` el 2026-10-03, bajo un modelo de workflow que el
  proyecto ya abandonó.

  O sea: **no están atascados por un bug, están atascados por una política que
  cambió debajo.** Cerrarlos exigiría decidir qué es un `merge-receipt` cuando la
  respuesta canónica es «no hay merge, hay push directo». Eso es una decisión de
  política y se declara, no se fuerza.
- **No añadió deuda.** La que hay sigue declarada.
- **No abrió el guard que falta**, aunque se vio el hueco.

## Deuda declarada, no abierta

- **El guard de pureza del kernel cubre UN módulo de N.** Existe
  `version_authority_fitness.rs` y escanea `version_authority.rs`. El dominio tiene
  muchos más módulos —`release_ref`, `actor_authority`, `planning_*`,
  `event_envelope`, `workflow_yaml`— y **ninguno** tiene equivalente. La ley «el
  núcleo no nombra tecnología» se vigila en uno de ellos, y el coste de añadirla
  al resto es bajo: es un escaner que ya existe y una lista de ficheros.
- **El `--dry-run` tiene menos cobertura que la release.** El paso 2b y el 9b no se
  ejecutan. Declarado arriba, con lo que costó.
- **Sin cambios:** `AdapterFact`/`EvidenceResolver` con 532 líneas sin consumidor,
  seis de los siete targets built-in sin cuerpo, cero observabilidad estructurada,
  cinco de cinco workflows de CI en `workflow_dispatch`, 553 `unwrap`/`expect` en
  producción, y el guard de StuckTogether en prosa que no ve la basura sin cambio
  de caja.