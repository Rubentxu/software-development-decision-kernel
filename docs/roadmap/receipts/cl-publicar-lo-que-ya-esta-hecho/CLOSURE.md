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

**VERIFICACIÓN FINAL: RELLENAR**

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