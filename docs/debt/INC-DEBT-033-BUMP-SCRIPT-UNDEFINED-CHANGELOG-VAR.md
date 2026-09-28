---
id: INC-DEBT-033-BUMP-SCRIPT-UNDEFINED-CHANGELOG-VAR
title: "El guard de merge del CHANGELOG referencia $CHANGELOG, variable que el script nunca define; con set -u el bump aborta"
status: closed
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-31 (OBSERVED, suite completa + bisect)
cluster_id: CL-RELEASE
related: [INC-DEBT-031-CHANGELOG-DUPLICATE-AND-PHANTOM-VERSIONS]
fingerprint: "release_bump_changelog_unset_variable_aborts"
closed_by: session-31 (fix + suite verde)
---

## Qué pasó (OBSERVED)

El fix de CHANGELOG de session-30 (`17d9b804`, "no duplicar la cabecera al
re-declarar una versión") introdujo 12 referencias a `"$CHANGELOG.md"`.
`release-bump.sh` **nunca define** una variable `CHANGELOG`: el código
original usaba el literal `CHANGELOG.md`. Con `set -euo pipefail` (línea 18),
bash aborta:

```
scripts/release-bump.sh: línea 224: CHANGELOG: variable sin asignar
scripts/release-bump.sh: línea 245: CHANGELOG: variable sin asignar
```

`"$CHANGELOG.md"` se expande a `".md"`, así que el script no solo aborta:
habría apuntado a un fichero llamado `.md` en el directorio del repo.

## Por qué llegó a commitearse

Los tests de la sesión anterior pasaban. Los dos que deberían cubrir el
cambio (`tests/test_changelog_merge.sh` y
`tests/test_release_bump_derivation.sh`) ejecutan **una copia del bloque**
del script, no el script. El bloque copiado en `test_changelog_merge.sh` usa
`CH="$SB/CHANGELOG.md"` — un nombre que sí existe — así que nunca tropieza
con `$CHANGELOG`. El test era verde probando algo que el script real no
hace.

El defecto real lo cazó `release_bump_prepends_changelog_and_resets_manifest_version`
(`cli.rs:12576`), que sí copia y ejecuta el script entero. Y por el mismo
bug rompía `cli_dev_install_default_layout_is_executable_and_verify_passes`:
ambos comparten el lock de `dev install`, y el bump abortando a mitad dejaba
el estado compartido inconsistente para el segundo.

## Bisect (OBSERVED)

| commit | `cli_dev_install_default_layout_is_executable_and_verify_passes` |
|--------|--------------------------------------------------------------|
| `ed0e3c47` (previo a session-30) | ok |
| `cf481d11` | ok |
| `e6998f01` | ok |
| `17d9b804` | **FAILED** |

Aislado también falla (`cargo test -p sddk-cli --test cli …` → FAILED), así
que no es flake ni contaminación de la suite completa.

## Corrección

Las 12 referencias vuelven al literal `CHANGELOG.md`, que es lo que el
resto del script ya usaba de forma consistente (líneas 179, 180, 251).

Verificado post-fix:

- `release_bump_prepends_changelog_and_resets_manifest_version` → ok
- `cli_dev_install_default_layout_is_executable_and_verify_passes` → ok
- `tests/test_changelog_merge.sh` → PASS (5/5)
- `shellcheck scripts/release-bump.sh` → limpio
- suite completa → ver commit de la corrección

## Lección que deja (deuda de método, no de código)

`tests/test_changelog_merge.sh` copia un bloque del script a un sandbox. Un
test que **reimplementa** lo que debe comprobar no puede notar que cambió.
Este INC es exactamente el fallo de ese patrón, y ya se había payado una
vez en session-30 con el guard de estáticidad (mismo patrón, test que
sobrevivía a la mutación). La corrección aplicada en el CI de esta sesión
—`tests/test_release_ci_manifest_anchor.sh` **extrae** el patrón real del
workflow— es la forma correcta, y este INC es el argumento de por qué hay
que migrar los tests que copian bloques.
