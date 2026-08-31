# Tasas de decaimiento de la evidencia tecnológica

`research-knowledge-curator` asigna `decay_date` según la velocidad de cambio del tema. Una claim caducada requiere re-verificación antes de citarse.

## Tasas por tipo de tema

| Tipo de claim | Decay (revisar cada) | Ejemplo |
|---------------|----------------------|---------|
| `version-specific-api` | 6 meses | "avian2d es compatible con 0.19" |
| `framework-api` | 1 año | "bsn! usa esta sintaxis" |
| `crate-existence` | 1 año | "X crate existe en crates.io" |
| `language-semantics` | 2 años | "ownership mueve por defecto" |
| `performance-claim` | 1 año (o tras release) | "Bevy hace batching de X" |
| `history` | 5 años (o nunca) | "Bevy se anunció en 2020" |
| `best-practice` | 1 año | "usa observers para X" |
| `opinion` | no caduca (atribuida) | cita de un maintainer |

## Disparadores de re-check inmediato (sin esperar decay_date)
- Nueva release mayor del framework (→ `version-drift-detector`).
- Cambio en el workspace de ejemplos (→ `code-prose-coherence-checker` drift).
- Una claim `verified` que una nueva fuente refute.

## Anti-patrón
Asumir que una afirmación de versión sigue siendo válida meses después sin re-verificar. El libro Bevy falló exactamente aquí: crates con versiones rotas porque se asumió estabilidad. El `decay_date` fuerza re-verificar.

## Regla de honradez
Si no se conoce la tasa de cambio de un tema, asignar 1 año conservador y marcar `uncertain_decay: true` para que el curator lo revise.
