# Dependency Map

```text
HX0
 │
 ▼
HX1
 │
 ▼
HX2 ──────────────┐
 │                │
 ▼                │
HX3 ───────► UAT F12-F14
 │
 ├────► HX4
 │       │
 │       ▼
 ├────► HX5
 │       │
 └────► HX6
         │
         ▼
        HX7
```

## Parallelism

- UAT F0–F11 puede continuar paralelo a HX0–HX3.
- HX4 y HX5 pueden desarrollarse en paralelo después de HX3 si comparten schemas estables.
- HX6 espera event taxonomy estable.
