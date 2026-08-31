# Protocolo de archivo (Wayback Machine)

Cuando `reference-validator` detecta `ROTTED`, antes de marcar `INVALID` busca archivo.

## Cuándo buscar archivo
- Siempre para referencias críticas (afirmaciones de versión/API/historia).
- Para referencias L5+ (posts de mantenedores) que pueden desaparecer.

## Cómo
- Wayback CDX API: `http://web.archive.org/cdx/search/cdx?url={url}&output=json&limit=1`
- Si hay snapshot, usar `https://web.archive.org/web/{timestamp}/{url}` como referencia archivada.
- Registrar `archived_url` y `archived_at` en la evaluación.

## Reglas
- Una referencia archivada es **válida pero penalizada en frescura** (el contenido pudo cambiar).
- Si la afirmación depende de versión/cambio reciente, el archivo puede no reflejar la realidad actual → marcar `disputed`.
- Preferir siempre la URL viva si resuelve; el archivo es fallback.

## Anti-patrón
Usar el archivo para "resucitar" una afirmación que la fuente viva ya no sostiene. Si la doc oficial actual contradice el snapshot archivado, gana la actual.
