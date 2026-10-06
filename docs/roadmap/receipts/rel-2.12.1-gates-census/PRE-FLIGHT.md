# PRE-FLIGHT — REL-2.12.1 «la superficie de gates cierra su census»

## Que se va a publicar

`v2.12.1`, con el worktree ya declarado en los cuatro punteros (`Cargo.toml`,
`manifest.toml`, `BUNDLE.toml`, `Cargo.lock` — todos `2.12.1`) y el ultimo tag
publicado siendo `v2.12.0`. **No hay bump pendiente**: segun AGENTS.md 2.3, la
proxima release ES `2.12.1`, no una minor inventada.

## Lo que se verifica ANTES de tocar nada

Cuatro condiciones, todas medidas en este turno y no heredadas:

1. **Los tres stop conditions del PRE-FLIGHT de VA17 se cumplen**, y el 1 se
   ha vuelto a medir en el directorio correcto (ver la nota de la condicion 1,
   que corrige una afirmacion mia de la sesion anterior).

2. **El censo de gates esta verde**: `SIN runner y SIN motivo: 0`, con su
   autofalsacion en `PASS=33 FAIL=0` y el sujeto real intacto por sha256. Este
   era el bloqueante declarado del bloque anterior; sin el, la release no podia
   arrancar.

3. **El changelog describe lo que se publica**: `PASS=10 FAIL=0`, los 7
   commits `feat`/`fix`/`test` desde `v2.12.0` representados por huella.

4. **El worktree esta limpio** y `HEAD == origin/main` (`6b1a73ab`).

## Condicion 1 — la que casi se lleva el bloque

El journal de la sesion anterior afirmaba que «los 212 ficheros de sesion del
layout antiguo ya NO EXISTEN». **Es falso, y lo medi en el sitio equivocado**: los
almacenes cuelgan de `data_home` (`~/.local/share/sddk`) y yo medi
`state_home` (`~/.local/state/sddk`), donde solo vive `ledger.sqlite`.

Remedido: **92 bindings, 94 deltas, 27 capsules**, los tres en
`projects/<p>/context/`, y **0** en el layout nuevo. El `PRE-FLIGHT` decia
91/94/26, luego hay **+1 binding y +1 capsule**, y 213 en agregado. Corregido en
el `RECEIPT.md` (que es la autoridad del bloque) y en el journal.

**Lo que no cambia es la conclusion**: los 92 bindings se leen (**0 ilegibles**),
ninguno lleva `workspace_id`, y quedan inertes porque migrarlos exigiria inventar
uno. El stop condition 1 dice «si no pueden quedar inertes sin riesgo de perdida
silenciosa, se para antes de tocar nada» — no hay riesgo porque **no hay nada que
perder**: estan integros y abandonables. Se cumple.

**El resto de las condiciones tambien, medidas hoy:**

- Stop 2 (ningun binding antiguo ilegible): `serde_deriva_option_ausente_a_none_sin_atributo` y
  su control pasan. Y se cumple por una razon que el `PRE-FLIGHT` no podia saber:
  **el `#[serde(default)]` no es load-bearing**, porque serde_derive ya trata
  `Option<T>` ausente como `None`. Fijado en
  `crates/sddk-engine/tests/va17_legacy_binding_serde.rs`.
- Stop 3 (el canario en verde sin que el codigo cambie): `EXPECT_LEAK="no"`, y las
  tres mutaciones mueven el veredicto (`PASS=4 FAIL=0 SKIP=0`).

## Por que esta release y no otra

La version esta declarada y sin publicar desde `v2.12.0`, el censo —que era el
bloqueante— esta verde, y los criterios del worktree estan verificados. Publicar
ahora convierte trabajo medido en artefacto; esperar deja otra vez la ventana
*declarada-pero-no-publicada*, que es la que el `pre-push` de AGENTS.md 2.1
documenta como cinta transportadora.

## Que NO se hace aqui

- **No se bumpea mas alla de `2.12.1`.** El bump lo aplica `release.sh` si hace
  falta, y el estado commiteado ya declara la version que se va a publicar.
- **No se toca INC-DEBT-050/061** (identidad e historia): decision del operador.
- **No se fabrica clave de firma ni ancla.** `SDDK_SKIP_SIGNING=1`, `UNSIGNED`
  declarado. Una release sin firma se declara; una con firma inventada es peor
  que sin firma.
- **No se repara el orden inestable de `candidates`**, ni el catch-all de
  `recovery:` en `sddk-gateway`, ni los 525 candados `/tmp/sddk-excl.*`: deuda
  declarada, cada una con su propia evidencia.

## Riesgo asumido

El layout de sesion es **rompente por naturaleza** (el propio `RECEIPT.md` §3.3):
quien tenga un `sddk` viejo construido contra el layout anterior seguira
escribiendo donde el anterior leia. No hay migracion ni aviso de vuelta atras.
Esta release es la que hace visible ese corte, y por eso su nota dice exactamente
eso.

## Readiness

**READY.** Las cuatro condiciones estan medidas en este turno, los tres stop
conditions del worktree se cumplen con la medicion corregida, y el unico
bloqueante que quedaba —el censo de gates— esta verde y con su autofalsacion.