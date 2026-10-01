---
id: INC-DEBT-047-CHANGELOG-DECLARED-NOT-COVERED-AND-UNGATED
title: el CHANGELOG declarado no cubria los commits que la release iba a publicar, y ningun gate lo impedia
status: resolved
severity: high
priority: P1
detected_at: 2026-10-01
detected_in_session: session-61
closed_in_session: session-61
component: release-governance
surface: CHANGELOG.md
references:
  - scripts/release.sh
  - tests/test_changelog_coverage.sh
  - tests/test_changelog_merge.sh
  - docs/RELEASING.md
  - docs/roadmap/ACCEPTANCE-TRUTHFULNESS-MATRIX.md
fingerprint: "changelog_declared_section_not_gated_against_actual_commits"
---

## Qué es

El `CHANGELOG.md` se edita a mano en el momento del bump, y **nada impedía
publicar una sección que no describiera el trabajo que se estaba
publicando**. `tests/test_changelog_merge.sh` existía desde session-30 y
comprueba que el merge de `release-bump.sh` **no duplique** una cabecera de
versión — pero no que la sección bajo esa cabecera describa los commits. Son
propiedades distintas, y la segunda no la comprobaba nadie.

Estado medido en session-61, al preparar v2.5.0:

```text
$ git log --oneline v2.4.2..HEAD | wc -l
26
```

Veintiséis commits sin publicar, entre ellos una slice `feat(architecture)`
entera (C3l.7), un `feat` de X07, un `fix(roadmap)` y tres `test`. La sección
`## [2.5.0]` listaba **dos** features: C3l.5 y C3l.3, las que ya estaban
cuando se commiteó el bump. `release-bump.sh --dry-run` confirma que la versión
no se deriva (`no bump to derive: the workspace already declares the pending
release (2.5.0)`) — es decir, **el trabajo crecía detrás de una sección
congelada sin ninguna señal**.

`scripts/release.sh` no mencionaba `CHANGELOG` en ningún paso: los 14 pasos
eran indiferentes a su contenido.

## Por qué importa

Es **la misma forma de defecto que C3l.7**, una capa más abajo: un artefacto
que no declara lo que es. El gate de arquitectura decía "conforme" sin
distinguir conformidad de deuda; el changelog diría "contenido" sin distinguir
contenido de lo que realmente viaja en el tarball.

El coste es asimétrico y ya se había pagado antes: con el release bloqueado
por `musl-gcc` la.publiación no ocurrió, pero en cuanto se levante habría
salido un artefacto cuyo changelog omite una feature completa. Un usuario que
lea el changelog para saber qué cambió no vería el gate de arquitectura.

El agravante específico del bloqueo: el release llevaba **tres sesiones
detenido** en un paso físico (instalar un paquete), y en ese intervalo se
accumularon dos slices enteras sin que la sección declarada se actualizara.
Ningún guard lo delató porque nadie miraba.

## Corrección

1. **`tests/test_changelog_coverage.sh`** (nuevo). Contrato:
   - la sección de la versión del workspace existe **exactamente una vez**;
   - tiene contenido;
   - todo commit `feat`/`fix`/`test` desde el último tag publicado está
     representado, con coincidencia por **huella** (tipo + scope + primeras
     4 palabras del payload), no por mera presencia del tipo.
   - fail-closed: changelog ilegible, sección ausente o rango vacío fallan
     con mensajes distintos; exit 0 sólo si la cobertura se sostiene.

2. **Integrado en `scripts/release.sh` como paso 2b**, después de leer la
   versión y **antes del build**. Un hueco detectado en el paso 9 —después de
   `gh release create`— cuesta borrar un release; detectado aquí cuesta un
   commit. `--dry-run` lo salta por diseño.

3. **CHANGELOG de 2.5.0 actualizado** con los 8 commits reales, más una nota
   de que la release sigue bloqueada: una sección que describe un artefacto no
   publicado debe decirlo, no dejar que el lector lo infiera.

## Falsificadores ejecutados

| # | Falsificador | Resultado |
|---|---|---|
| **F17** | Eliminar la línea de `feat(architecture)` del changelog | **OBSERVED** — `[FAIL] missing from section`, `PASS=9 FAIL=1`, exit 1 |
| **F18** | Sustituir el payload por uno genérico **conservando el scope** (`feat(architecture): improvements to the architecture subsystem…`) | **OBSERVED** — `[FAIL] declared type present but not this commit`, exit 1 |

**F18 es el que da valor al gate.** Sin la huella de payload, un `feat` de
cualquier cosa habría satisfecho a cualquier otro `feat` que compartiese scope,
y un changelog lleno de líneas genéricas habría pasado. El gate no comprueba
"hay una entrada de tipo feat", comprueba "está **esta** entrada".

## Criterio de cierre (alcanzado en session-61)

- (a) `bash tests/test_changelog_coverage.sh` → `PASS=11 FAIL=0`, exit 0.
- (b) `shellcheck tests/test_changelog_coverage.sh scripts/release.sh` → limpio.
- (c) `bash -n scripts/release.sh` → sin error de sintaxis.
- (d) El paso 2b aparece en el flujo y está condicionado a `DRY_RUN == 0`.

## Límites declarados

1. El gate compara **tipo, scope y las primeras 4 palabras del payload**. Un
   reescritura profunda del subject entre el commit y el changelog puede
   exceder la huella y producir un falso negativo. Se prefiere un falso
   negativo (revisar de más) a un falso positivo (publicar de menos).
2. Excluye `docs`/`chore` a propósito: un changelog que liste cada
   sincronización de punteros es ruido. El riesgo es que un `docs` contenga un
   cambio con efecto observable; en este repo los `docs` son narration y los
   cambios con efecto van como `feat`/`fix`.
3. El gate compara contra el **último tag publicado**, no contra
   `origin/main..HEAD`. Si hay commits sin publicar que no entran en la
   release, quedan fuera del contrato — que es lo correcto, porque no viajan.
4. El `--dry-run` de `release.sh` salta el paso 2b. Es intencionado: el
   dry-run no publica nada y su contrato es "pasos 0-8 sin publicar".
