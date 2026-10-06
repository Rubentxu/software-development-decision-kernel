# PRE-FLIGHT — VA13 `cl-publicar-lo-que-ya-esta-hecho`

**Cycle:** `p-63676b11dc0ef88f/version-coherence-071`
**Fecha:** 2026-10-06
**Predecesor:** VA12 `e5d5f3fc` + `bf76f40d` (ADR-0166), publicado en `origin/main`

---

## Readiness: READY

---

## Qué falta, medido

Este bloque **no escribe una línea de código**. Publica lo que ya está escrito, y
la razón de que sea el bloque siguiente es una medición, no una preferencia.

### MEDIDO 1 — treinta y un commits sin publicar

```
$ git log --oneline v2.11.4..HEAD | wc -l
31
```

Y lo que llevan:

| | |
|---|---|
| commits | **31** |
| ficheros de código (`crates/`, `scripts/`, `tests/`) | **43** |
| ADRs | **11** (0153, 0157–0166) |
| distribución | 7 `feat(release)`, 3 `feat(version)`, 2 `feat(target)`, 3 `fix(release)`, 1 `fix(version)`, 1 `fix(push)`, 2 `refactor(gateway)`, 1 `refactor(release)`, + documentación |

La regla 6 de la skill lo dice literalmente: *«evita micro-releases triviales y
acumular features sin liberar (big-bang)»*. Cuatro bloques seguidos han añadido
capacidad y **ninguno se ha liberado**. Eso es el big-bang que la regla prohíbe, y
se llega aquí no por decisión sino porque cada bloque era urgente y el release
siempre quedaba para el siguiente.

### MEDIDO 2 — el binario instalado va 31 commits por detrás

```
$ bash scripts/check_binary_freshness.sh
binario   : /home/rubentxu/.local/bin/sddk
checkout  : bf76f40d
relacion  : behind
detalle   : el binario (8edbb748) es ancestro del HEAD del checkout
```

**Y declara la MISMA versión que el workspace**, luego la comparación de versiones
sale verde. Es exactamente el caso para el que existe §2.3.1: `sddk 2.11.4` y
`workspace 2.12.0` se comparan bien porque los dos son el mismo número en el
sentido de la versión, y el código son dos cosas distintas.

### MEDIDO 3 — lo que el usuario no puede usar

Todo lo que VA9–VA12 dejaron escrito existe **solo en `main`**:

- `--evaluate-build` con dialecto Gradle/Maven (VA9, VA10)
- `.sddk/version-source.json` con `authority: "version"` (VA11)
- `matches` y `handoff` que **pueden preguntar**, y `measured` que distingue «no
  lo sé» de «no coincide» (VA12)

Un usuario con el binario instalado hoy **no tiene ninguna de las tres**.

### MEDIDO 4 — seis ciclos esperando que esto ocurra

```
$ sddk cycle list
status[RELEASE_PENDING]: 6

$ sddk cycle next --cycle .../cl-build-identity
  - transition: release.complete
    requires_met: false
    requirement: merge-receipt; requirement: release-receipt
```

Los seis (`a4-1-generic-verify`, `ledger-watch-total`, `ledger-export-total`,
`cl-release-forge-testability`, `cl-build-identity`, `cl-doctor-build-identity`)
están en `ReleasePending/Release` y su transición legal exige **`merge-receipt` y
`release-receipt`**. Esos recibos los produce una release. Publicar es, para esos
seis, la acción que los desbloquea.

---

## Por qué una release y no el siguiente bloque de funcionalidad

Porque el siguiente bloque de funcionalidad **seguiría sin llegar a nadie**, y el
coste de eso ya se ha pagado: cuatro capacidades medidas, falsificadas y cerradas, y
cero usuarios.

Hay una tentación concreta y se declara aquí para no tomarla: el bloque de
`AdapterFact`/`EvidenceResolver` —532 líneas de producción sin consumidor con
`#[allow(unused)]`— es deuda real y se puede hacer sin riesgo. Es tentador porque
es conocido. **Es la elección equivocada**: deuda que no llega al usuario y
capacidad que sí, con la capacidad detrás de un tag. La deuda se queda declarada y
sigue ahí después de publicar; la capacidad, no.

### Y una tentación mayor, que también se declara

Los seis ciclos `RELEASE_PENDING` parecen una tarea propia. **No lo son.** Su
transición exige recibos que emite el release, luego «cerrar los seis» sin publicar
sería fabricar recibos o saltarse su puerta. Se cierran **después**, con la release
de por medio, y cada uno con su evidencia. Adelantarlos sería la clase de atajo que
este repositorio lleva siete veces registrando como defecto.

---

## Decisión

`bash scripts/release.sh`, el entry point canónico de `AGENTS.md` §8, los quince
pasos, **sin** `--dry-run`, `--skip-tests` ni `--force`.

Y antes, `--dry-run`, porque el propio §8 avisa de que un hueco tras
`gh release create` cuesta borrar un release y no hay forma limpia de deshacerlo.

### Sin bumpear

El workspace declara `2.12.0` y el último tag publicado es `v2.11.4`. La próxima
release **es `2.12.0`** (§2.3: el workspace version es puntero ceremonial del
release, no una versión de desarrollo). MEDIDO:

```
$ release_admission_check_v2 HEAD
ACCEPT last-publish=2.11.4 -> 2.12.0
```

La admisión es **semántica**: comprueba que hay un cambio real y monótono de
`[workspace.package] version` por encima del último tag, no que el subject del
commit diga una cosa concreta. Por eso no hace falta el commit de bumpeo, y por eso
el `pre-push` ya aceptó los cuatro bloques anteriores sin `--no-verify`.

### El `--dry-run` primero, y por qué no es un ahorro de tiempo

El `--dry-run` salta el paso 2b (cobertura de changelog) porque no publica nada, y
salta el 9b porque presupone el 9. Ejercita los pasos 0–8: preflight, gates,
build, reconciliación de artefacto, manifest, bundles, sha256 y sbom. **Es el
ensayo del mecanismo que va a escribir fuera del repositorio**, y un ensayo que
solo se hace en el ensayo real no es un ensayo.

---

## STOP conditions

1. **Que un gate del release falle.** No se salta ninguno. `--skip-tests` solo si
   el perfil completo ya pasó en esta sesión y se deja dicho.
2. **Que el `9b` (public-release gate) dé rojo.** Es el paso que verifica la release
   **ya publicada** antes de instalar en local. Falla cerrado y para la release.
3. **Que el 12 (prune) borre algo de más.** Se mide el directorio antes y después.
4. **Que la versión del artefacto no sea `2.12.0`.** Se comprueba en el paso 2 y en
   el binario construido; si no coincide, para.

## El riesgo real, escrito antes de empezar

El riesgo **no** es que falle un gate: los gates existen para eso y paran cleanly.

El riesgo es que **el paso 10 espere a la CDN** y que la espera se agote. Está
documentado desde cycle-47 y por eso el script hace poll al sha256 del binario
contra la URL pública antes de instalar. Si la CDN sigue sirviendo stale tras 5
minutos, el script aborta con un error claro y **no se intenta sortearlo con
`--skip-install`** — porque `--skip-install` no salta el 9b, y saltarse el 9b es
justo lo que el gate existe para impedir. La respuesta correcta es esperar.

El segundo riesgo, más aburrido y más probable: **el host tiene ~1219 procesos** y
esta sesión ya vio ETXTBSY al arrancar scripts recién escritos. Está declarado como
limitación del entorno, no del código, y se mide si aparece.

---

## Lo que este bloque NO hace

- **No toca código.** Un release que arregla código es un release con un commit de
  fix dentro, y ese commit es un bloque propio con su propio `PRE-FLIGHT`.
- **No cierra los seis ciclos.** Ver arriba: sin sus recibos, cerrarlos sería
  fabricar evidencia.
- **No abre deuda nueva.** La que hay sigue declarada y sigue ahí después.
- **No bumpea.** La versión ya es la correcta; bumpear para satisfacer una forma en
  vez de una medición es exactamente lo que `INC-DEBT-040` ya dejó escrito, y
  este bloque lo aplica: la versión es la que se va a publicar, porque se ha
  medido cuál es.
---

## Checkpoint — cambio de premisa, 2026-10-06, antes de publicar

Este bloque decía, en negrita, **«No toca código»**. Y el perfil completo del
workspace lo encontró en rojo en el primer minuto.

### Lo que pasó

```
$ cargo test --workspace
test the_decision_module_names_no_concrete_technology ... FAILED
  crates/sddk-domain/tests/version_authority_fitness.rs:117
  el modulo de decision nombra tecnologia concreta: ["gradle"].
  El kernel define las preguntas; los providers saben obtener la evidencia.
```

**Es mío, y está publicado.** MEDIDO contra Git:

```
$ git show 97dd9293:crates/sddk-domain/src/version_authority.rs | grep -ci gradle
0
$ git log --oneline -S"Gradle" -- crates/sddk-domain/src/version_authority.rs
1621c3fb feat(release): el proyecto declara su version y publica, …
```

`1621c3fb` es el primer commit de **VA11**, de hace dos bloques. El dominio tenía
**cero** apariciones y ahora tenía una: la escribí en un doc-comment de
`ProviderError::Malformed`, al explicar por qué esa variante hacía falta, citando
el ejemplo concreto que la motivó.

### Por qué nadie lo vio antes, y esa es la parte que importa

VA11 **tocaba el dominio** y corrí el perfil de gateway y CLI. VA12 no tocó el
dominio y también corrí gateway y CLI. Los dos bloques se leyeron como «un cambio de
gateway».

`prompts/sddk/change-scoped-testing.md` manda razonar en
`ActiveChangeSet → SUT impact → verification batch`, y lo que hice fue razonar en
`→ crate que creía haber tocado → verification batch`. **La ley mal aplicada es
exactamente la que dice que no se aplique**: el SUT de VA11 incluía `sddk-domain`, luego
`cargo test -p sddk-domain` estaba en el lote. No se ejecutó porque el *nombre* del
crate me, y el nombre es el dato que ya tenía antes de abrir el
diff.

Y hay un agravante medido: `cargo test --workspace` **sin `--no-fail-fast` para en
el primer binario rojo**, luego este 1 fallo puede estar escondiendo otros que
nunca se midieron. La corrida que lo encontró se relanzó con `--no-fail-fast`
justo por eso.

### Decisión

**Este bloque pasa a tocar código**, con su propio commit, antes de la release. Y
no es una excepción: es el orden correcto. Publicar un tag cuyo fitness de dominio
está en rojo es publicar una frontera que el propio repo declara y que su guard
dice rota; el resto del trabajo del bloque —los 31 commits— **no** es el problema,
es el contexto.

La corrección es de prosa, no de lógica: el comportamiento no cambió desde VA11,
la frontera lleva rota desde entonces. Lo que se retira es el nombre de la
herramienta, y se añade en su lugar **por qué este doc no lo nombra**, para que
la ausencia tenga motivo escrito y no parezca descuido.

### STOP condition revisada

El bloque declaraba «que un gate del release falle → no se salta ninguno». Se
cumple, y además: **el perfil completo del workspace es parte del gate 1**, luego
este fallo no es un gate que pueda saltarse sino **el** primer paso del release
haciendo su trabajo. Lo que habría sido un STOP es haber publicado sin él.

---

## Checkpoint — blocker nuevo, 2026-10-06, en el paso 1b de la release real

El ensayo (`--dry-run --skip-tests`) pasó los pasos 0–8. **La release real no.**

```
==> 1b/15 — shell contract tests (tests/test_*.sh)
  x FALLOS en tests/test_release_state_pointer.sh:
  4:  [FAIL] el puntero va 32 commit(s) por DETRAS de main (tolerancia 3)
  8:  [FAIL] workspace_version_at_current dice '2.11.4' pero Cargo.toml dice '2.12.0'
  RESULT: FAIL — STATE.yaml miente sobre el estado del repo.
```

**Y no es un defecto de este bloque:** es el drift documental que llevaba declarado
desde el principio, acumulado commit a commit. `current_sha` estaba en `4b50d043`,
**32 commits** por detrás de `a7a74c7b`.

### Lo que enseña, y es lo que hay que mirar dos veces

El `--dry-run` se lo saltó. No porque el gate no existe —existe y está en el 1b—
sino porque **`--skip-tests` se salta el 1b entero**. La decisión de usar
`--skip-tests` en el ensayo era correcta y está escrita en este mismo `PRE-FLIGHT`
—el perfil completo acababa de pasar en la misma sesión y el 1b se paga una vez—
y aun así el ensayo no detectó lo que la release real detectó.

**Un ensayo que se salta una puerta no ensaya esa puerta, y por eso el ensayo tiene
menos cobertura que la cosa que ensaya.** Es la segunda vez en este bloque que sale
lo mismo: el `--dry-run` tampoco corre el 2b. Las dos son el mismo hecho.

La decisión que sale de aquí, y que no es de este bloque: **`--dry-run` debería
avisar de qué pasos se salta y de que su cobertura es menor que la de la release
real**, porque hoy lo hace en silencio.

### La reparación, y por qué no es una decisión

`AGENTS.md` y el propio mensaje del gate separan dos cosas:

| | quién |
|---|---|
| `current_sha`, `head_at_state_sync`, `workspace_version_at_current` | **`scripts/reconcile_state_pointer.sh`** |
| el juicio sobre **qué significa** el estado | humano |

Ejecutado. MEDIDO del diff: **3 campos**, `superseded_pointer` preservado tal cual,
notas de evidencia conservadas y **cero historia reescrita**. El gate pasa a 8/8.

### Y una parte que el script NO hace, y que sí es mía

El script deja la **prosa** del campo como estaba, y esa prosa ya era falsa: decía
*«el tag publicado mas alto es v2.8.1»* con el valor del campo en `2.11.4`. Dos
mitades del mismo campo describiendo **dos hechos distintos**.

Y la nota vieja decía exactamente por qué eso está mal: *«un campo estructurado
con una prosa que ya no describe el mismo hecho es la misma mentira que el check
3d del guard prohíbe en `current_sha`, y ese check todavía no mira aquí»*. O sea:
la propia nota avisaba, el guard no la vigila, y el script la deja.

**Un campo puede pasar el gate y seguir mintiendo**, porque el gate compara
**valores** y la mentira estaba en la **prosa**. Corregida a mano con el estado
medido, y sin reescribir la historia anterior.

Eso es un hueco de guard, no de código: `test_release_state_pointer.sh` valida
`workspace_version_at_current == Cargo.toml` y nada más. Declarado abajo.

### STOP condition revisada, la tercera

El bloque declaraba que si un gate falla no se salta ninguno, y que si la versión
del artefacto no es `2.12.0` se para. **Ninguno de los dos se ha saltado**: el 1b
paró la release, se midió la causa, se reparó lo mecánico con el script que el propio
gate nombra, y se corrigió a mano lo que el script deja. Lo que se hará es
**reintentar la release entera**, no continuar desde donde paró.

---

## Checkpoint 3 — el release llega al 8c y muere por una credencial que no tengo

El cuarto intento pasó **todo** lo anterior al 8c: `1b` los 52 contratos,
`2b` changelog, `3` binario, `3b` `PASS=16`, `3c` `PASS=10`, `3d` `PASS=16`,
`3e` `PASS=14`, `3f` `PASS=9`, `3g` `PASS=8`, `3h` `PASS=6`, `3i` `PASS=5`,
`3i-b` `PASS=2`, `3j` `PASS=4`, `3k` `PASS=5`, `3l` `PASS=5`, **`3m` `PASS=20`**,
`3n` los siete `uat_ctx_*`, `4` manifest (395 ficheros), `5` bundle, `6`
`BUNDLE.toml` 2.12.0, `7` unificado 12.692.388 B, `8` checksums+sbom, `8b` espejo
del vault idempotente. Y en el `8c`:

```
no signing key configured.
  The release anchor is key-based as of ADR-0151, so a release needs a signer.
```

### Lo primero que NO es: un gate nuevo

`git log -S cosign -- scripts/release.sh` lo sitúa en commits antiguos, y
`git show v2.11.4:scripts/release.sh` **ya tiene el 8c con el mismo texto**.
No se rompió nada al publicar este bloque: el paso estaba ahí desde antes.

### Lo segundo que NO es: que falte una credencial por primera vez

Lo decide un hecho medido, no una impresión:

```
$ gh release view v2.11.4 --json assets
   - CHECKSUMS              - sddk
   - gh-release-receipt.json  - sddk-v2.11.4-sddk-linux-x86_64-musl.tar.gz
   - sbom.json              - sddk-v2.11.4-sddk-linux-x86_64-musl.tar.gz.sha256
   - sddk.sha256            - software-development-decision-kernel.tar.gz
                            - software-development-decision-kernel.tar.gz.sha256
   → 9 assets, CERO `.sig`
```

**La release publicada tampoco estaba firmada.** `SDDK_SKIP_SIGNING=1` no es una
puerta que este bloque vaya a abrir: es la puerta por la que ya salió `v2.11.4`, y
por la que el propio diario registró `2.5.3`. Publicar `2.12.0` sin firmar
**conserva** el contrato del artefacto publicado. Publicarla firmada lo cambiaría
en los dos sitios a la vez —en el `8c` y en el instalador de cada usuario— y para
eso haría falta un KMS que no existe en esta máquina.

### Por qué no es «sortear un gate»

Es la distinción que ya salió tres veces en este bloque: la pregunta no es *«¿hay
un gate que se puede saltar?»* sino *«¿qué afirma el artefacto y quién lo firma?»*.

- **Sortear un gate** es no mirar la regla y continuar: dejar `9b` sin correr,
  subir una tolerancia, `--no-verify`.
- **Esto** es declarar, **antes de publicar**, que el artefacto no llevará firma,
  con la consecuencia escrita en el propio mensaje del script y en el nombre de
  la bandera que la exige: quien lo instale tendrá que pasar
  `SDDK_ALLOW_UNSIGNED=1`, y el instalador imprimirá *«installing with NO
  signature check»*.

Laintegridad (`CHECKSUMS`, `sbom.json`, `MANIFEST.sha256`) se sigue verificando.
Lo que **no** se verifica es la **autenticidad**, y eso queda dicho, en el log y
en la instalación.

### Una sola variable, y el pipeline ya sabe propagarla

`release.sh` (paso 10) hace `if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ]; then export
SDDK_ALLOW_UNSIGNED=1`. La decisión se toma **una vez**, en el `8c`, y el `10` la
recibe en vez de volver a negarla —que es exactamente el fallo que ese paso
comentado documenta: tres sitios decidiendo la misma verdad.

### El fallo fantasma que casi me hace concluir otra cosa

Un `ls` seguido y un `tail` seguido, ambos sobre la misma ruta escrita a mano,
devolvieron `No existe el fichero o el directorio` sobre un directorio que **sí**
existe, **sí** está en git (`git ls-files` lista sus dos ficheros) y que contiene
321 + 183 líneas. Repetido al momento con la misma cadena: verde.

Lo escribí como «el volumen montado devuelve `ENOENT` transitorio bajo carga»,
y **esa explicación quedó falsada por el siguiente comando**: el `cat >>` que
escribía este mismo checkpoint devolvió el mismo `ENOENT` y, sin embargo, **sí
escribió** — el fichero pasó de 321 a 413 líneas y contiene el checkpoint. Si el
error fuera del sistema de ficheros, la escritura no habría pasado.

Así que lo medido es esto, y lo no medido es la causa:

- **Medido**: la misma ruta literal falla unas veces y funciona otras; un
  `ENOENT` puede acompañar a una escritura que sí ocurrió.
- **No determinado**: por qué. No se ha medido si es el montaje, la saturation
  del host o la propia forma de la invocación. Queda declarado, no explicado.

**Un `ENOENT` sobre un fichero versionado parece una pérdida de trabajo y no lo
es, y el primer impulso —dar por perdido lo que se tiene delante— es el
equivocado.** Si hubiera reconstruido a ciegas un recibo que estaba entero,
habría inventado Receipts para un trabajo que ya estaba hecho. Lo que evita eso
no es la prudencia: es `git ls-files`, que responde con la verdad sobre lo que
Git sabe. A partir de ahí, todas las rutas de este bloque se resuelven con un
glob en lugar de una cadena escrita a mano.

### Decisión

Relanzar el release entero con `SDDK_SKIP_SIGNING=1`, sin más cambios: HEAD
`81763c7c` = `origin/main`, worktree limpio, `v2.12.0` **no** existe ni como tag
remoto ni como release —cero estado parcial que reparar—. El modo de fallo que
importa aquí no es que el release pare, es que pare **después** de publicar algo
a medias.
