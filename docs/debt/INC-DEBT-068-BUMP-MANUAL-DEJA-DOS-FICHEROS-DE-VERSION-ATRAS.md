---
id: INC-DEBT-068
title: "Un bump manual deja BUNDLE.toml y manifest.toml atras, y la release se abre en el 1b una vez por cada fichero, sin dejar estado parcial"
status: resolved
severity: medium
priority: P2
fingerprint: "manual_version_bump_moves_one_of_three_version_files"
fingerprint_aliases: []
cluster_id: CL-RELEASE
created: 2026-10-04
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-04
detected_in_session: session-78
component: scripts/release-bump.sh, BUNDLE.toml, manifest.toml, Cargo.toml
surface: scripts/release-bump.sh:275-300, tests/test_dev_install_source_guard.sh, tests/test_release_state_pointer.sh
related: [INC-DEBT-070]
references:
  - scripts/release-bump.sh
  - tests/test_dev_install_source_guard.sh
  - tests/test_release_state_pointer.sh
---

## Qué es

La version del workspace vive en **tres** ficheros que tienen que moverse
juntos, y un bump hecho a mano mueve uno:

| Fichero | Clave | Lo lee |
|---|---|---|
| `Cargo.toml` | `[workspace.package] version` | todo el build, el pre-push hook, `release_admission.sh` |
| `manifest.toml` | `version` | `test_release_state_pointer.sh` (paso 1b) |
| `BUNDLE.toml` | `version`, `binary_min_version`, `binary_max_version` | `test_dev_install_source_guard.sh` (1b) y **`sddk dev install --source`** en runtime |

`scripts/release-bump.sh` los mueve de una vez (`:275-300`, con verificacion
del rango `[min, max]`). Un bump manual —`sed` sobre `Cargo.toml`— deja los
otros dos atras, y el paso 1b lo detecta.

## OBSERVED — medido dos veces en la misma sesion, y ya habia pasado antes

La release de **v2.7.0** se lanzo dos veces y **murio en el 1b las dos**, sin
publicar nada (ni tag ni release; verificado contra la API de GitHub). Los dos
fallos tenian el mismo origen y se/dcubrieron **uno a uno**:

1. `BUNDLE.toml version 2.6.0 != workspace 2.7.0`
2. `manifest.toml dice '2.6.0', Cargo.toml dice '2.7.0'`

**No son dos defectos: es uno, y el pipeline los encuentra por orden
alfabetico de fichero.** Cada unoundle en su propia, y cada una consumio un
ciclo completo de release: paso 1 (suite completa del workspace, ~10 min bajo
carga) + paso 1b (los shell contract tests, ~25 min) + build de release
(~20 min atascado por el lock del `CARGO_TARGET_DIR` compartido). **Dos
detecciones de la misma causa, mas de dos horas de maquina.**

**Y ya estaba documentada.** `release-bump.sh:281-287` dice, con las tres
claves y el motivo:

> Este paso no es cosmetico: `tests/test_dev_install_source_guard.sh` (paso 1b)
> exige que la version del bundle sea la del workspace, y sin esto **toda
> release futura muere en 1b en el primer bump**, con un mensaje que habla de un
> fosil y no de la causa —que es que el bump se dejaba untracked file por
> delante sin avisar. **Medido en session-70**: el bump 2.5.3 -> 2.5.4 dejo
> `BUNDLE.toml` en 2.5.3 y la release murio ahi.

O sea: **tercera vez** que el mismo defecto para una release (session-70, dos
veces en session-78), con el mecanismo identificado y escrito desde la primera.

## Por qué `medium` y no `high`

Los guards **funcionan**: el 1b fallo cerrado las dos veces, no publico nada ni
a medias, y el diagnostico fue preciso. El daño es tiempo de ciclo, no
integridad — y el tiempo se puede gastar. Por eso no es `high`.

Por la misma razon **no es `low`**: el mecanismo ya se conoce y esta escrito
desde session-70, y aun asi recurrio dos veces en una sesion. Un defecto
documentado que se repite **no es un recordatorio que falle, es un control que
no existe**: el unico sitio donde se evita es el script que el operador puede
no usar.

## Por qué no se arregla aqui

El arreglo correcto **no** es tocar los guards (harían su trabajo peor) ni
recordarlo en el changelog (ya está escrito en el propio script desde
session-70). Es **una sola autoridad de version**: que el bump pase por
`release-bump.sh` siempre, o que un unico guard *fail-closed en el commit* (no
solo en la release) detecte que `Cargo.toml` se movio y los otros dos no. La
segunda es un guard nuevo, y un guard nuevo necesita su autofalsacion, asi que
es su propio bloque — no media linea pegada al final de un release.

Mientras tanto, **el operacion es known y esta en el commit**:
`chore(release): bump version` lo hace `release-bump.sh`, no a mano.

## Cómo refutarla

Cerrada cuando un bump **manual** de `Cargo.toml` no pueda dejar los otros dos
ficheros atras sin que nada lo detecte antes de la release. Hoy el
`test_release_state_pointer.sh` y el `test_dev_install_source_guard.sh` los
detectan, pero **en el paso 1b de la release**: cuando ya se ha gastado la suite
completa. El criterio de cierre es **anterior**: fallar en el commit que bumpea,
no en el release que publica.

---

## RESUELTA (session-85) — el control esta en el push, que es donde la
## deuda pedia que estuviera

### OBSERVED primero: el defecto seguia vivo, y el texto de la deuda se
### habia quedado a medias

La deuda decia que el mecanismo era un `BUNDLE.toml` **untracked** que el
bump se dejaba por delante. **Medido: los tres ficheros estan TRACKEADOS**
(`git ls-files` los devuelve a los tres, ninguno en `.gitignore`). Esa parte
del encuadre ya no se sostiene y no se arrastra a la resolucion.

Lo que si se sostenia, y se **falso con el hook real** antes de tocarlo:
un rango que mueve `[workspace.package] version` en `Cargo.toml` y deja
`manifest.toml` y `BUNDLE.toml` atras era **ADMITIDO** por `githooks/pre-push`
(`ACCEPT`), con los quince gates de la release en verde. El hook leia la
version —para admitir— pero no leia los otros dos ficheros.

### Donde va el control, y por que ahi

**En `githooks/pre-push`, y antes de cualquier ruta de admision.** No es un
recordatorio mas: es la superficie semantica que ya decide que entra a
`main`, y se ejecuta **antes** de la release, que es literalmente el
criterio de cierre que esta deuda se fijo: *«fallar en el commit que bumpea,
no en el release que publica»*.

Es un **veto, no una cuarta ruta**. Una ruta ADMITE un push; no CERTIFICA
que la version sea coherente. Por eso se evalua antes del primer `continue`:
colocado despues, decoraria los rangos que las otras rutas ya admitieron,
que es la mitad de los pushes de una release.

Y se **alinea con la autoridad en vez de stricter que ella** —error propio
de este bloque, medido y corregido antes de commitear—. La primera version
buscaba la clave bajo `[pack]` y `[bundle]`; `release-bump.sh` sustituye
por **ancla de linea** (`s/^version = "[^"]*"/…/`) y no sabe de tablas, y
el guard del 1b hace lo mismo (`test_release_state_pointer.sh:173`). Un hook
mas estricto que la autoridad no la vigila: la contradice, y el operador ve
un rechazo sin causa real. El predicado del veto es **el mismo** que el de
los 1b, no un tercero.

### Criterio de cierre, comprobado uno a uno

> *Cerrada cuando un bump manual de `Cargo.toml` no pueda dejar los otros
> dos ficheros atras sin que nada lo detecte antes de la release.*

| Que exige el criterio | Medido |
|---|---|
| el estado partido se detecta ANTES de la release | `REJECT` en el pre-push, no en el 1b |
| cada uno de los tres portable por separado | 3 casos `REJECT`, uno por carrier |
| el camino bueno no se rompe | el bump coherente de los tres `ACCEPT` |
| el veto no es una quarta ruta | caso dentro de la ventana tag-baseline: `REJECT` |
| un fichero ausente no es un fichero partido | los 48 casos previos siguen `ACCEPT`/`REJECT` como antes |

Matriz `tests/test_push_prevention_hook.sh`: `PASS=55 FAIL=0` (48 previos +
7 nuevos). Corre el hook **real** por `core.hooksPath`, no una copia.

### HALLAZGO PROPIO: una rama que diagnostica y no decide

`if [[ -z "$declared" ]]` —el carrier presente que no declara version— se
midio y **resulto no ser decisoria**: cuando `Cargo.toml` ya fijo `first`,
la rama siguiente (`elif declared != first`) reporta el mismo fichero por la
misma discrepancia. O sea que el caso «BUNDLE.toml sin clave version»
pasaba en verde **por la rama de discrepancia, no por la fail-closed**. Un
caso verde por la razon equivocada, que es el mismo patron que este repo ha
encontrado cuatro veces y que no se deja sin decir: el autofalsador lo
mide con M2 y **exige que el veredicto NO cambie**, en vez de buscarle una
mutacion que no existe. La rama se queda por su valor diagnostico —el
mensaje distingue «ilegible» de «distinto»— y **no se cuenta como diente**.

### El falsador, y sus tres instrumentos rotos

`tests/test_push_prevention_coherence_mutation.sh`, siete mediciones. Las
correcciones las puso **el falsador fallando**, no la inspeccion:

1. **M1 y M5 caian y se reportaban como «no cayo».** El needle buscaba el
   nombre del caso en la linea de contadores, que solo tiene numeros. Las
   dos mutaciones habian tumbado la matriz entera.
2. **M3 era M6 disfrazado.** La primera mutacion metia la llamada dentro
   de `if [[ -n "$VERSION_BUMP" ]]`, y en un rango sin bump de `Cargo.toml`
   ese bloque no se ejecuta: la llamada quedaba tan inalcanzable como si no
   existiera, luego no mediaba la COLOCACION que pretedia medir.
3. **M4 no se aplicaba y casi se contaba como deteccion.** Su ancla
   (`    return 1` seguido de `}`) aparece **dos veces** en el hook. Por eso
   `apply` exige que cada ancla case **exactamente una vez** y declara
   `NOTAPPLIED` si no: una sustitucion no unica mutaria el sitio
   equivocado, y una que no muta es `NOTAPPLIED`, nunca `PASS`.

M7 no existia como distincion y se creo para ello: el veto tiene que mirar
el **tip**, no el primer commit del rango, y el caso que lo separa es un
rango que **parte y se realinea** —tip coherente, asi que admitirlo es lo
correcto, y un veto que mirase el rango lo reprobaria por un estado partido
que ya no existe.

**El falsador corre la matriz ENTERA contra un hook mutado**, con
`SDDK_PREPUSH_HOOKS_DIR` apuntando a la arena: los casos no se reescriben
en el falsador, porque un falsador con sus propios casos prueba sus propios
casos — el defecto que INC-DEBT-074 cerro. El hook real nunca se muta, y la
trampa restaura y **compara el sha256** aun asi, porque una trampa que
restaura «en theory» envenena la suite siguiente.

### GATES

- `tests/test_push_prevention_hook.sh` — `PASS=55 FAIL=0`
- `tests/test_push_prevention_coherence_mutation.sh` — 7 mediciones
- `shellcheck --severity=warning` limpio en los tres ficheros tocados

### Lo que NO se corrige aqui

La superficie de gates del 1b sigue siendo una lista escrita a mano —lo
dejo escrito y medido la sesion anterior en INC-DEBT-076—, y este bloque no
la toca: es otro concernimiento y no es lo que esta deuda pedia.
