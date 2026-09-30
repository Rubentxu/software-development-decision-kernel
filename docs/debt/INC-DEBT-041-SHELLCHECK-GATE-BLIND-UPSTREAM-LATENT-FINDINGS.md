---
id: INC-DEBT-041
title: ShellCheck gate is blind upstream and carries latent findings
severity: medium
priority: P2
status: resolved
opened: session-45
resolved: session-47
resolution: Ruta 1 (limpieza real a severidad style, sin tocar ci.yml ni bajar tolerancia)
component: ci
surface: .github/workflows/ci.yml
---

# INC-DEBT-041: el gate de ShellCheck está ciego aguas arriba y arrastra hallazgos latentes

> **RESUELTA (session-47, 2026-09-30).** Se eligió la ruta 1 del triaje: limpieza
> real a severidad `style`, sin tocar `ci.yml`, sin bajar tolerancia y sin
> baseline declarado — el gate quedó en **0 hallazgos** con el comando exacto
> del step (`shellcheck -S style`, default 0.11.0). Re-medición previa: el
> alcance real era mayor que el de session-45 (≈49 hallazgos en 15 ficheros,
> la mayoría `SC2016` intencional: grep de literales `$VAR` contra YAML/sh);
> las medidas de esta sección se conservan como foto histórica del momento del
> registro. Tratamiento: 30 directivas `# shellcheck disable=SCnnnn` con
> justificación por línea (patrón de contrato de literales productivos), 2
> refactorizaciones mínimas (`SC2129` en apply_banner con grupo `{ … } > tmp`,
> verificado con smoke de ambas ramas) y `SC2034` documentada (`GATE_END` es
> anchor espejo, no código muerto). Tests de los 10 ficheros tocados: PASS
> (release×7, install asset contract, vault×2 + receipt_authority y
> pipeline_consistency). La ceguera aguas arriba sigue siendo una propiedad
> estructural de `ci.yml` (no corregible desde este repo: los jobs son
> secuenciales por diseño), pero deja de ocultar hallazgos reales: si el step
> llega a correr, está en verde genuino.

## Criterio verificable (y se sostiene)

`ci.yml` — job *Required quality gates*, step *Run ShellCheck on shell surfaces* — ejecuta:

```bash
for f in tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh; do
  if [ -f "$f" ]; then shellcheck "$f" || exit 1; fi
done
```

Con shellcheck 0.11.0 y sin flags, el default severity es `style`: **`info`, `style` y `warning` fallan el job**. Medido sobre el árbol de `origin/main` (sin los commits de session-45) y reproducido sobre HEAD:

- **9 ficheros, 28 hallazgos**: 25 `info`, 2 `style`, 1 `warning` (SC2034 `GATE_END`, código muerto en `tests/test_release_public_gate.sh:55`).
- Los dos ficheros nuevos de session-44 (`scripts/check_debt_index_coherence.sh`, `tests/test_debt_index_coherence.sh`) salen **limpios**.

Si los gates previos hubieran pasado, este step habría fallado el job.

## Por qué nunca se ha observado como fallo: ceguera aguas arriba

El step está **después** de `Run workspace tests` en el mismo job, y no hay `if: always()` ni `continue-on-error`. En el único run de `ci.yml` registrado sobre `main` (run `36450601924`, `ed0e3c47`, 2026-09-28) el job **murió en `Run workspace tests`** (los dos tests de vault de `INC-DEBT-032`, ya corregidos después en `182e74f5`), y **todo lo posterior quedó `skipped`**, shellcheck incluido:

```
success  Check formatting
failure  Run workspace tests
skipped  Run strict Clippy
skipped  Lint repository contracts
skipped  Run ShellCheck on shell surfaces   <-- nunca se evaluó
```

Es decir: los 28 hallazgos son **latentes**, no un fallo observado. El gate es real y fallaría, pero está ciego: si los tests upstream se arreglan y el job llega al step de shellcheck, falla.

## Consecuencia

`ci.yml` da `failure` hoy, y su último fallo real no era shellcheck. Mientras el job muera antes (tests, clippy, lint), el paso de shellcheck **nunca se ejecuta** y los 28 hallazgos nunca se ven. Esto hace que `ci.yml` dé una señal de calidad **engañosa**: un job rojo puede deberse a cualquier cosa salvo al lint que lo precede, y un job verde no habría detectado ninguno de los 28.

## Alcance exacto (medido, no estimado)

Ficheros y hallazgos, con el comando aggregate exacto del step:

```
 2  scripts/apply_banner.sh                       (SC2016, SC2129)
11  tests/test_install_asset_contract.sh          (SC2016, SC2129, …)
 1  tests/test_release_bump_derivation.sh         (SC2016)
 2  tests/test_release_bundle_layout.sh           (SC2016, SC2129)
 3  tests/test_release_ci_staging.sh              (SC2016, SC2129, …)
 4  tests/test_release_public_gate.sh             (SC2016 ×3, SC2034)
 1  tests/test_release_tag_anchoring.sh           (SC2016)
 2  tests/test_vault_adr_mirror_coverage.sh       (SC2016, SC2129)
 2  tests/test_vault_mirror_auto.sh               (SC2016, SC2129)
```

Mayoría `SC2016` (expresiones `$` en comillas simples dentro de `grep -E`, que es **intencionado**: el patrón es literal) y `SC2129` (`echo >>` en bucle, estilo). El único `warning` real es `SC2034` (`GATE_END` sin uso).

`scripts/release.sh` **no** cuenta: su único aviso (SC1091) solo aparece con `-x` y sale limpio en el comando aggregate del step.

## Decisión de triaje (pendiente del operador)

Ninguna implementada. Tres salidas, con su coste:

1. **Limpiar los 28** — formatter de estilo en 9 ficheros, incluido `GATE_END` muerto. Coste: diff ancho y cosmético, riesgo de tocar la superficie de release.
2. **Fijar severidad y baseline** — `shellcheck -S warning` (deja pasar info/style) o `-S error`, y documentar el baseline. Coste: baja el listón del gate; hay que dejarlo explícito para que no parezca verde por accidente.
3. **Declarar que la nube no gatea** (§2.5) y **sacarlo del checklist de `AGENTS.md` §5**. Coste: se pierde la superficie de shell del gate local; coherente con que la evidencia real es local, pero contradice el checklist tal como está escrito.

Lo que **no** es admisible: bajar la tolerancia sin declararlo, o llamar verde a un gate que no se ha ejecutado. La ceguera aguas arriba es el problema de fondo: si no se arregla el orden, cualquier fix de los 28 puede seguir sin observarse.

## Reproducción

```bash
# el comando exacto del step, con su exit code
shellcheck tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh; echo "EXIT=$?"   # EXIT=1, 28 hallazgos

# ceguera: el step está skipped aguas arriba cuando fallan los tests previos
gh run view 36450601924 --json jobs -q '.jobs[] | .steps[]? | "\(.conclusion)\t\(.name)"'
```

## Conocimiento negativo

- **Suponer que "el último CI rojo" es el shellcheck era falso.** El fallo real del único run fue `Run workspace tests` (los dos tests de vault de `INC-DEBT-032`, ya corregidos). Shellcheck nunca corrió. La hipótesis era plausible y sin verificar; falsarla costó leer el log del job, no el árbol local.
- **Un bucle `for` con `|| true` mal colocado dio un recuento erróneo** (10/29 frente al 9/28 real): el `|| true` tragaba el código de salida y `n` se contaminaba. El comando aggregate directo es el que se debe citar como número canónico.
