---
name: deep-reference-validator
description: "Trigger: validar URL viva, verificar DOI, link rot, referencia muerta, validación de fuente en vivo, ¿la URL sigue funcionando?, ¿el paper sigue accesible?. Valida que cada referencia del corpus siga accesible y vigente. Ejecuta en paralelo con deep-source-credibility-assessor (R3). Detecta link rot, drift de versiones, DOIs revocados. Crítico para no citar URLs muertas en el libro."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: r-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
  based_on: "reference-validator (rubentxu), generalizado"
---

## Activation Contract

Úsalo en **R3b del pipeline R**, en paralelo con `deep-source-credibility-assessor` (R3a). Recibe `research/candidate-pool.yml` (o parte de él) y produce `research/reference-validation.jsonl` con el estado de cada URL/referencia.

No lo uses para: puntuar credibilidad (eso es R3a), descubrir fuentes (R2), triangular (R4). Esta skill **valida que la referencia está viva y vigente**.

## Hard Rules

- **HEAD request primero** (no GET completo) para validar sin consumir ancho de banda.
- **Timeout corto** (5-10 segundos por URL) para no bloquear el pipeline.
- **Reintentos con backoff** solo si hay 5xx; no reintentar en 4xx.
- **Marcar `dead`** sin ambigüedad: status 4xx, dominio expirado, DNS failure, o contenido claramente irrelevante.
- **Wayback Machine como fallback**: si una URL está muerta, buscar `web.archive.org/web/*/{URL}`.
- **Version drift**: para software, verificar que la versión citada sigue siendo la actual (si no, marcar `version_drift`).
- **Idempotente y reproducible**: misma entrada → mismo resultado (salvo cambios reales en la web).

## Execution Steps

1. Lee `research/candidate-pool.yml` (o un subconjunto si es re-check focal).
2. Para cada candidato con URL:
   a. Ejecuta `curl -I -L --max-time 10 {url}` o equivalente.
   b. Registra `http_status` (200, 301, 404, 500, etc.).
   c. Registra `final_url` (después de redirects).
   d. Registra `content_type` y `last_modified` (si están disponibles).
3. Para URLs muertas:
   a. Busca en Wayback Machine: `https://web.archive.org/web/*/{url}`.
   b. Si hay snapshot, marca `wayback_url` con la fecha del snapshot más cercano.
4. Para software (código fuente, paquetes):
   a. Si la URL es un repo (GitHub, GitLab), verifica que el repo sigue activo.
   b. Si la URL es un paquete (crates.io, npm, PyPI), verifica que la versión citada sigue publicada.
   c. Marca `version_drift: true` si hay una versión más reciente.
5. Para papers con DOI:
   a. Verifica el DOI: `https://doi.org/{doi}`.
   b. Registra el destino final.
6. Genera `research/reference-validation.jsonl` (un JSON por línea, una por candidato validado).

## Esquema de reference-validation.jsonl

```json
{
  "source_id": "src-bevy-source-schedule",
  "url_checked": "https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/schedule/",
  "http_status": 200,
  "final_url": "https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/schedule.rs",
  "content_type": "text/plain",
  "last_modified": "2026-08-15T12:34:56Z",
  "wayback_url": null,
  "version_drift": null,
  "validation_status": "live",
  "validated_at": "2026-08-16T10:00:00Z",
  "notes": null
}
```

Estados posibles de `validation_status`:
- `live`: 200 OK, contenido relevante.
- `live-but-redirected`: redirige pero llega a contenido válido.
- `dead-4xx`: 404, 403, etc.
- `dead-5xx`: 500, 502, 503, etc. (reintentar).
- `dead-domain`: DNS failure o dominio expirado.
- `dead-content`: 200 OK pero contenido no relevante (drift de contenido).
- `version-drift`: la versión citada es antigua; hay más reciente.

## Decision Gates

| Situación | Acción |
|-----------|--------|
| URL `live` | OK; consumir en R4 |
| URL `live-but-redirected` | OK si el destino final es equivalente (mismo paper/libro); warning si no |
| URL `dead-4xx` | Buscar en Wayback; si no hay, marcar `evidence_level` degradada (L1 → L2) |
| URL `dead-domain` | Buscar espejo o repositorio alternativo; si no, rechazar |
| URL `version-drift` (tech) | Actualizar URL a la versión actual; re-evaluar la claim (puede haber cambios) |
| DOI revocado | Buscar el nuevo DOI; si no, marcar `dead` y rechazar |
| URL que requiere autenticación | Marcar `auth-required: true`; puede usarse si el lector tiene acceso, pero no es verificable por HEAD |

## Output Contract

- `research/reference-validation.jsonl` con el estado de cada candidato.
- Las fuentes con `validation_status: dead` y sin wayback son **rechazadas** automáticamente para claims `critical`.
- Las fuentes con `version-drift` disparan re-check de las claims que las usan.

## References

- `references/wayback-strategies.md` — cómo buscar en Wayback efectivamente.
- `references/version-drift-detection.md` — heurísticas para tech.
- `assets/reference-validation.schema.json` — esquema.
