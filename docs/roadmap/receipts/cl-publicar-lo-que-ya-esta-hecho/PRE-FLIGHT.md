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
