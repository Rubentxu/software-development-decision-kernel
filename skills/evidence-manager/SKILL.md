---
name: evidence-manager
description: "Trigger: gestionar evidencia, asociar fuentes, vincular afirmaciones con fuentes, mantener evidencia, claims, provenance. Asocia cada afirmación técnica del libro con su fuente, versión y fecha, manteniendo el índice de claims verificable."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo para **mantener** el índice de claims (relación afirmación ↔ fuente), de forma transversal a `source-researcher`, `chapter-writer` y `hallucination-auditor`. `source-researcher` crea las cards; esta skill mantiene el índice global y su coherencia.

No la uses para buscar fuentes nuevas (`source-researcher`).

## Hard Rules

- Cada claim relevante del libro tiene un `claim_id` estable.
- Un claim solo puede ser `verified` si su `source_id` resuelve en `sources.yaml`.
- Si una fuente cambia de versión, todos los claims asociados pasan a `needs_recheck`.
- Ningún capítulo puede publicarse con claims críticos en `unverified` o `disputed`.

## Execution Steps

1. Leer `research/evidence-cards/*.yml` y los comentarios `// evidence: ev-xxx` de los `.adoc`.
2. Reconstruir `evidence/claims.jsonl` (una línea por claim, esquema en `assets/claim.schema.json`).
3. Detectar:
   - Afirmaciones en `.adoc` sin `claim_id` (gap de evidencia).
   - Claims cuyo `source_id` ya no existe en `sources.yaml`.
   - Claims con fuentes cuya `version` no coincide con `book-config.yml`.
4. Marcar claims afectados por cambios de versión (insumo para `version-drift-detector`).
5. Emitir `evidence/coverage-report.yml`: cobertura por capítulo y lista de gaps.

## Esquema de claim (claims.jsonl)

```json
{
  "claim_id": "claim-ch04-017",
  "chapter": "04-ownership",
  "text": "El préstamo mutable debe ser exclusivo durante su uso.",
  "card_id": "ev-borrowing-exclusive-mut",
  "source_id": "rust-book-ownership",
  "status": "verified",
  "verified_at": "2026-07-22"
}
```

## Output Contract

- `evidence/claims.jsonl` actualizado.
- `evidence/coverage-report.yml` (claims por capítulo, gaps, needs_recheck).
- Lista de claims que bloquean publicación (`unverified`/`disputed` críticos).

## References

- `assets/claim.schema.json` — esquema de un claim.
