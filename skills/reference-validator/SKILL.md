---
name: reference-validator
description: "Trigger: validar referencia, verificar URL, comprobar DOI, validar crate, comprobar versión de crate, link vivo, link roto, referencia existe, verificar API existe, crates.io, docs.rs, GitHub. Verifica de forma viva y reproducible que cada referencia del libro realmente existe y dice lo que afirmamos: URLs resuelven, DOIs válidos, crates existen con la versión citada, APIs están en la versión declarada."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **durante la Macro-fase R** (valida el candidate-pool antes de triangular) y **en Macro-fase C** (valida las referencias finales que aparecen en el capítulo). Es la verificación **viva y concreta**: ¿esta cosa existe y dice lo que decimos?

No lo uses para evaluar credibilidad (`source-credibility-assessor`) ni para triangular (`evidence-cross-validator`). Esta skill es **empírica**: fetch + parse + aserción.

## Hard Rules

- Toda referencia del libro (URL, DOI, crate, API, issue/PR) se verifica **vivamente** antes de publicar.
- Una referencia que no resuelve o no dice lo afirmado es `INVALID` (critical, bloqueante).
- Las verificaciones son **reproducibles**: el script en `assets/validate-references.sh` las ejecuta.
- Se registra `verified_at` + `http_status` + `found_claim` para cada referencia.
- Link rot → buscar archivo (Wayback) antes de rechazar.

## Tipos de referencia y método de verificación

| Tipo | Método |
|------|--------|
| URL web | HTTP GET, comprobar status 200 y que la página contiene el término clave |
| DOI | Resolver `https://doi.org/{doi}`, comprobar metadata |
| Crate (Rust) | `crates.io/api/v1/crates/{name}`, campo `max_version` y compatibilidad |
| docs.rs | `docs.rs/{crate}/{version}`, comprobar que el item existe |
| API/símbolo | docs.rs del módulo, buscar el símbolo en la versión declarada |
| GitHub issue/PR | API `api.github.com/repos/{org}/{repo}/issues/{n}` |
| GitHub release/tag | comprobar que el tag existe |
| RFC | resolver la URL del RFC oficial |

## Execution Steps

1. Recibir el conjunto de referencias (del candidate-pool o del `.adoc` final).
2. Para cada referencia, ejecutar su método de verificación (delegar a Bash con `assets/validate-references.sh`, o WebFetch para contenido).
3. Para afirmaciones de contenido ("esta página dice X"): fetch + buscar el término clave en el body.
4. Clasificar resultado:
   - `VALID` — existe y contiene lo afirmado.
   - `VALID_DIFFERENT` — existe pero dice otra cosa (la afirmación del libro está mal).
   - `ROTTED` — no resuelve; buscar archivo Wayback (`web.archive.org/web/*/{url}`).
   - `INVALID` — no existe o no dice lo afirmado y no hay archivo.
5. Registrar en `research/reference-validation.jsonl`.
6. Cualquier `INVALID` o `VALID_DIFFERENT` crítico → bloqueante.

## Esquema de resultado (jsonl)

```json
{
  "reference": "https://docs.rs/bevy_scene/latest/bevy_scene/trait.SceneComponent.html",
  "type": "docs.rs",
  "claim": "SceneComponent trait existe en bevy_scene",
  "http_status": 200,
  "found_claim": true,
  "verified_at": "2026-07-23",
  "status": "VALID"
}
```

```json
{
  "reference": "avian2d 0.18 compatible con Bevy 0.19",
  "type": "crate-version",
  "claim": "avian2d soporta Bevy 0.19",
  "crates_io_max_version": "0.7.0",
  "manifest_bevy_dep": "0.18",
  "found_claim": false,
  "verified_at": "2026-07-23",
  "status": "VALID_DIFFERENT",
  "detail": "avian2d 0.7 depende de bevy 0.18, no 0.19"
}
```

## Decision Gates

| Resultado | Acción |
|-----------|--------|
| `VALID` | Aceptar referencia |
| `VALID_DIFFERENT` | Devolver a `source-researcher`/`chapter-writer` para corregir |
| `ROTTED` con archivo Wayback | Sustituir por URL archivada, nota de frescura |
| `ROTTED` sin archivo | `INVALID`, eliminar referencia |
| `INVALID` | Bloqueante; buscar alternativa o eliminar afirmación |

## Output Contract

- `research/reference-validation.jsonl` (una entrada por referencia).
- `research/reference-validation-summary.yml` (conteos por status).
- Lista de `INVALID`/`VALID_DIFFERENT` críticos (bloqueantes).
- `hallucination-auditor` consume esta salida en Macro-fase C.

## References

- `assets/validate-references.sh` — script reproducible de verificación.
- `references/wayback-protocol.md` — cuándo y cómo usar el archivo.
