---
name: version-drift-detector
description: "Trigger: drift de versiones, dependencias obsoletas, breaking changes, comprobrar versiones, impacto de upgrade, comparar versiones del libro. Compara las versiones del libro contra documentación oficial, código fuente actual y changelogs, y emite el impacto (capítulos afectados, severidad, acciones)."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo de forma periódica o antes de una nueva edición (alimenta a `release-maintainer`). Es el **radar** que detecta que el libro se ha quedado atrás respecto a la realidad del framework.

**División de responsabilidades**: esta skill detecta drift de **versiones/dependencias** (Bevy, crates externos). El drift **código-libro** (qué capítulos quedan desactualizados cuando cambia el código del workspace) lo detecta `code-prose-coherence-checker` en modo `drift`. Ambos informes alimentan a `release-maintainer`.

No la uses para corregir el drift (eso es `release-maintainer`), ni para drift de código-libro (`code-prose-coherence-checker`).

## Hard Rules

- Compara siempre contra **fuentes vivas**: documentación oficial actual, crates.io, código fuente, changelog.
- El impacto se mide a nivel de **capítulo** (qué capítulos se ven afectados).
- Toda dependencia con drift se clasifica por severidad.
- La salida es insumo directo para `release-maintainer`.

## Execution Steps

1. Leer `book-config.yml` (versiones declaradas) y `examples/*/Cargo.toml`.
2. Para cada dependencia, consultar la versión actual real:
   - Framework principal: release notes / docs oficial.
   - Crates: crates.io / docs.rs.
3. Comparar `previous` (libro) vs `current` (realidad).
4. Leer changelog/breaking changes entre ambas.
5. Mapear breaking changes → capítulos afectados (usando `outline.yml` y `sources.yaml`).
6. Clasificar severidad y acciones requeridas.
7. Emitir `build/drift-report.yml`.

## Pipeline conceptual

```
Versiones del libro → Documentación oficial → Código fuente actual → Changelog/breaking → Capítulos afectados
```

## Esquema de salida

```yaml
impact:
  dependency: bevy
  previous: "0.18"
  current: "0.18"
  affected_chapters: [ch03-app, ch05-scheduling, ch08-rendering]
  severity: high
  required_actions:
    - Recompilar ejemplos
    - Revisar nombres de sistemas
    - Regenerar capturas
```

## Output Contract

- `build/drift-report.yml` (una entrada por dependencia con drift).
- Resumen ejecutivo: nº de dependencias con drift, severidad máxima, nº de capítulos afectados.
- Estado: `CLEAN` (sin drift) o `DRIFT_DETECTED`.

## References

- `references/drift-sources.md` — dónde consultar versiones reales por tecnología.
