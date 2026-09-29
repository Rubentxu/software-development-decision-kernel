# Hitos, dependencias y criterios de salida

## Tabla resumida

| Hito | Depende | Valor | Exit falsable |
|---|---|---|---|
| C3i | current baseline | recovery coherente | 0/1/N cycle recovery + adoption idempotente |
| C3j | C3i | contexto durable | process restart conserva binding/capsule/delta |
| C6a | C4 + contracts existentes | vocabulario plataforma | resource/step/workflow compile-only |
| C6b | C6a | una autoridad workflow | default slice compila equivalent a IR |
| C6c | C6a, C2 | augmented steps | capability→provider automático/explainable |
| C6d | C6b,C6c,C3j | valor agent-first | vertical software persiste structured knowledge |
| C6e | C6d | HATEOAS agent UX | harness navega por affordances |
| C6f | C6e | extensibilidad usuario | custom workflow sin prompt plumbing |
| C7a | C6f | boundary core/pack | software assumptions fuera de core generic |
| C7b | C7a | segundo dominio | book workflow E2E |
| C7c | C7b | prueba de generalidad | portability UAT sin core fork |

## Critical path

```text
C3i → C3j ─────────────┐
                       ├→ C6d → C6e → C6f → C7a → C7b → C7c
C4 → C6a → C6b ───────┤
       └→ C6c ─────────┘
C2 ─────────→ C6c
```

## Parallelismo seguro

- C6b y C6c pueden desarrollarse en paralelo después de C6a.
- UAT de hypermedia read-only puede empezar en C3j pero mutation affordances esperan C6d.
- Book pack design puede prototiparse tras C6f contract freeze, no antes.
- C5 opcionales pueden avanzar independientes si no modifican contracts compartidos.

## Stop conditions globales

- Si legacy→IR equivalence no puede preservar una semántica crítica, STOP y corregir IR/operator contract antes del cutover.
- Si capability resolution requiere provider-specific types en domain, STOP boundary violation.
- Si durable context exige nueva canonical DB, STOP; reutilizar storage/CAS/event model.
- Si user workflow necesita lifecycle recipes manuales para evidence/handoff, C6f no está cerrado.
- Si Book workflow requiere `software.*` dentro de core contracts, C7 no está cerrado.
