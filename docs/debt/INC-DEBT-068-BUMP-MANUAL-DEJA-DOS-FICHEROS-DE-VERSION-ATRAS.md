---
id: INC-DEBT-068
title: "Un bump manual deja BUNDLE.toml y manifest.toml atras, y la release se abre en el 1b una vez por cada fichero, sin dejar estado parcial"
status: open
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
