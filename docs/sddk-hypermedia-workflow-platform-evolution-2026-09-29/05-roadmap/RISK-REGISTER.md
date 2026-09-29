# Registro de riesgos

| ID | Riesgo | Prob. | Impacto | Mitigación / falsador |
|---|---|---:|---:|---|
| R-01 | Crear otro workflow engine sin querer | M | Muy alto | ADR-002; WorkflowRuntime única authority; fitness import/dependency |
| R-02 | Hypermedia projection se vuelve state DB paralela | M | Alto | affordances derived; canonical refs only; rebuild UAT |
| R-03 | Augmentor se convierte en god-object | M | Alto | pipeline de resolvers puros; explain provenance; bounded responsibilities |
| R-04 | StepDefinition se convierte en prompt monolítico | M | Alto | schema contract + anti-pattern lint; prompts externos/versionados |
| R-05 | Providers contaminan domain | M | Alto | capability IDs + anti-corruption; no provider types fitness |
| R-06 | ContextCapsule crece sin límite | Alta | Alto | progressive disclosure, refs, size budgets, context read metrics |
| R-07 | Reuse de stale observations produce decisiones incorrectas | M | Alto | multi-basis staleness incl provider/analyzer/config |
| R-08 | Automatic augmentation ejecuta herramientas caras innecesarias | M | M | preferred/required/optional + cost hints/budgets + explain |
| R-09 | Hidden provider fallback falsifica evidence | Baja/M | Muy alto | typed gaps; fallback receipt; REQUIRED fail closed |
| R-10 | Workflow custom abre arbitrary shell privilege | M | Muy alto | capabilities governed; custom step binding admission; no wildcard |
| R-11 | Big-bang prompt rewrite rompe UAT | M | Alto | ADR-008; one responsibility/slice; compatibility aliases |
| R-12 | Persistent context migrations dañan storage | Baja/M | Muy alto | previous-schema fixtures, crash/reopen, preflight monotonic model |
| R-13 | Session binding confunde host session con run | Baja | Alto | arch-spec-024 invariant + UAT multi-session |
| R-14 | Step cache/memoization replays side effects | Baja | Muy alto | no side-effect memoization; defer feature until C8 |
| R-15 | Core “genérico” sólo de nombre | Alta | M/Alto | Book pack as falsifier; C7c acceptance |
| R-16 | Pack ecosystem introduce compatibility hell | M | Alto | versions/digests/ranges + compile-time resolution + fail isolated |
| R-17 | Resource URIs cambian y rompen provenance | M | Alto | decide identity rules before public SDK; version URI grammar |
| R-18 | Dynamic Discover causa unbounded expansion | M | Alto | existing budgets/max_nodes/depth/convergence + permission gate |
| R-19 | MCP surface explota en cientos de tools | Alta | M | generic resource/action MCP tools |
| R-20 | Human UX empeora por exceso de machine concepts | M | M | human text projection/dashboard separated from protocol |

## Riesgos que bloquean public stable API

R-01, R-02, R-05, R-10, R-12, R-17 deben tener UAT y ADR aceptados antes de una API estable externa.
