---
id: INC-DEBT-040-PREPUSH-BUMP-PREDICATE-UNSATISFIABLE-FOR-DECLARED-RELEASE
status: open
severity: high
priority: P1
detected_at: 2026-09-30
detected_in_session: session-44
blocks: [release-v2.2.32, release-v2.2.33]
references:
  - githooks/pre-push
  - scripts/release-bump.sh
  - scripts/release.sh
  - scripts/lib/release_admission.sh
  - docs/debt/INC-A5-PUSH-RELEASE-MARKER-FRICTION.md
---

# INC-DEBT-040: el predicado del pre-push es insatisfacible para un release ya declarado

- **Estado**: open (high/P1)
- **Detectada**: session-44 (2026-09-30), al ejecutar el primer paso que
  session-43b dejó escrito
- **Bloquea**: la publicación de v2.2.32; cualquier release cuyo bump
  declarado ya esté en `origin/main`

## Contexto

El primer paso de session-43b, escrito como verbatim, era:

```bash
git push origin main
bash scripts/release.sh
```

El push **falla** con `HOOK_EXIT=1`. El motivo reportado no es el que el
puntero anticipaba, y esa es la razón de esta incidencia: **no es un bump
pendiente, es un predicado que no se puede satisfacer.**

## Causa raíz

Dos gates de la release toman **baselines distintos** para la misma
pregunta («¿hay un bump de versión que publicar?») y no coinciden:

| Gate | Baseline | Fuente |
|---|---|---|
| `githooks/pre-push` (A) | `origin/main` (el rango que se va a pushear) | `remote_sha..local_sha` |
| `release_admission_check_v2` | **el último release publicado** | tag de GitHub Releases |

El bump a `2.2.32` se commiteó en `3d4e457a`, que **ya es ancestro de
`origin/main`**. Por tanto, en el rango `5440b2e8..f717dca6` **ningún
commit cambia `[workspace.package] version`** — comprobado commit por
commit, salida vacía. El predicado (A) exige un cambio de versión *dentro
del rango*, y ese cambio ocurrió 44 commits antes de `origin/main`: es
inobservable por construcción.

`release_admission_check_v2` responde `ACCEPT last-publish=2.2.27 ->
2.2.32` porque su baseline es el tag publicado (`v2.2.27`), donde `2.2.32`
sí está por encima. **Los dos gates no se contradicen: responden a
preguntas distintas sobre la misma frase, y cada una tiene razón.**

El bump ceremonial de session-43b (`061afe26`, subject
`chore(release): bump version`) **no bumpeó nada**: su padre ya era
`2.2.32`. Su subject cumplía la convención, y el propio hook advierte que
el subject no es autoridad — el bump fue ceremonial en el sentido literal
de la palabra, y por eso el rango no lo puede ver.

## El auto-desbloqueo tampoco existe

`scripts/release-bump.sh` se niega a derivar un bump:

```text
workspace (2.2.32) is ahead of last tag (v2.2.27); the workspace declares
the pending release (2.2.32), no bump to derive
```

Es decir: la vía diseñada para «hay un release pendiente, bumpea» se
autodesactiva precisamente cuando hay un release pendiente ya declarado.
`--force-version` sí funciona (ver Falsación), pero es una decisión
humana, no una recuperación automática.

## Falsación

Ejecutada en un clon aislado (`--no-hardlinks`, sin red, remoto real
intacto). El hook se invocó **directamente**, no vía `git push`, para que
el exit code fuera el del hook y no el de un comando del pipe:

| Escenario | `githooks/pre-push` | `release_admission_check_v2` |
|---|---|---|
| Rango real `5440b2e8..f717dca6`, sin bump (**control**) | `HOOK_EXIT=1` REJECT | `ACCEPT ... 2.2.27 -> 2.2.32` |
| Tras bump real `2.2.32 -> 2.2.33` (`--force-version`) | `HOOK_EXIT=0` **ACCEPT** | `ACCEPT last-publish=2.2.27 -> 2.2.33` |

**Cuidado con el método**: una primera medición dio `REJECT` después del
bump y se registró como «el hook rechaza incluso un bump real». Era
**falso**, y la causa fue medir el exit code de `head` en lugar del del
hook (`cmd | hook && echo ACCEPT || echo REJECT` mide el de `head`).
Repetido con captura explícita del exit code: `HOOK_EXIT=0`. Cuarta vez
que un artefacto de medición —no el código— afirma algo falso; la
preceden INC-DEBT-033, INC-DEBT-037 y los dos bugs del guard de
autenticidad en session-43.

## Impacto

- **La release está bloqueada, no rota.** El árbol (44 commits, perfil
  completo verde en `061afe26`) es publicable; lo que no existe es un
  camino que lo publique con la etiqueta `2.2.32`.
- Publicar `2.2.32` exigiría saltar el hook (`--no-verify`), lo que
  contradice el contrato de `AGENTS.md` §2.3 (el tag declara la versión
  publicada) sin que exista bump visible que lo respalde.
- `release.sh` **no puede** ejecutar su paso 1c: hace `git push origin
  main` y muere con el mismo error (línea 293). No hay flag para saltar
  ese paso; `--skip-tests` y `--skip-install` no lo cubren, por diseño
  («pushing is part of the release contract, not the test gate»).
- El rojo de `tests/test_release_state_pointer.sh` (2 checks) es
  consecuencia de esto: `current_sha` no puede estar contenido en
  `origin/main` mientras el push está bloqueado.

## Opciones (ninguna implementada — decisión del operador)

1. **Publicar como `2.2.33`** con bump real. Verificado: hook acepta,
   admission acepta, el changelog fusiona en una sección nueva. La
   etiqueta describe honestamente un árbol que contiene 2.2.32 más 44
   commits. Coste: el tag `2.2.32` queda sin publicar para siempre.
2. **Publicar como `2.2.32` con `--no-verify`.** Coste: se publica con
   una etiqueta sin bump visible en el rango, y el changelog de 2.2.32
   no describe el árbol que se publica.
3. **Corregir el predicado del hook** para que compare contra el último
   tag publicado, igual que `release_admission_check_v2`, en vez de
   exigir el cambio dentro del rango. Es la opción que elimina la deuda
   en vez de rodearla, pero **altera un gate de admisión**: requiere su
   propia decisión, tests que falsifiquen el caso nuevo, y no es
   emergency work.

## Desviación de contrato que conviene registrar

`AGENTS.md` §2.1 afirma que el hook de prevención se activa con
`git config core.hooksPath githooks` y «rechaza push a main sin commit
`chore(release): bump version`». **Esa frase describe el hook
ceremonial anterior, no el actual**: el hook vigente rechaza el marker
ceremonial vacío *y* exige un cambio de versión real o un rango
docs-only. La formulación vigente está en
`INC-A5-PUSH-RELEASE-MARKER-FRICTION` (closed) y en
`tests/test_push_prevention_hook.sh` (matriz de ~40 casos). La
documentación de `AGENTS.md` §2.1 va por detrás del código y es la
razón de que session-43b creyera que el bump de `061afe26` desbloquearía
el push. Corregir esa frase es parte de la remediación de esta INC.
