# Niveles de evidencia (L1–L7)

Definen **qué rigor exige cada tipo de afirmación**. No todas las afirmaciones necesitan el mismo nivel: una anécdota histórica tolera más que una firma de API.

## Niveles (de más a menos riguroso)

| Nivel | Fuente | Ejemplo |
|-------|--------|---------|
| **L1-exp** | Experimentación reproducible propia | Benchmark con metodología publicada y código |
| **L1** | Especificación / estándar formal | RFC de IETF, spec del lenguaje, W3C |
| **L2** | Documentación oficial vigente | docs.rs, doc.rust-lang.org, bevyengine.org/learn |
| **L3** | Código fuente del proyecto (versión concreta) | módulo, test, release notes |
| **L4** | Paper revisado por pares | arXiv, ACM, IEEE con DOI verificable |
| **L5** | Post del mantenedor con fecha | blog oficial, discourse del proyecto |
| **L6** | Libro de referencia canónico | "The Rust Programming Language" |
| **L7** | Comunidad (foros, blogs terceros) | SO, Reddit, Medium |

## Regla de asignación por tipo de afirmación

| `claim_type` | Nivel mínimo exigido |
|--------------|----------------------|
| `api-existence` | L2 (doc oficial) o L3 (código) |
| `version` | L2 o L3, con `retrieved_at` |
| `behavior` | L2 + L3 (doc + código que lo demuestre) |
| `performance` | L1-exp (benchmark) o L4 (paper); nunca L7 |
| `history` | L5 o L6 con fecha verificable |
| `opinion` | L5 atribuido, marcado explícitamente como opinión |
| `best-practice` | L5 del mantenedor + L3 que lo ejemplifique |

## Suelo por defecto
El `research-strategist` fija un `default_authority_floor` (normalmente L3): ninguna afirmación crítica se publica con fuente por debajo. L7 **nunca** se cita sola para una afirmación crítica.

## Anti-patrón
"Así lo recuerdo" o "lo vi en un blog" para una afirmación de versión/API. Esas son L7 sin corroboración y deben bloquearse hasta subir a L2/L3.
