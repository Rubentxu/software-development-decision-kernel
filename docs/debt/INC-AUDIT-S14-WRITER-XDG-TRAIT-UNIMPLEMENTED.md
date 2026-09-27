---
id: INC-AUDIT-S14-WRITER-XDG-TRAIT-UNIMPLEMENTED
title: "WriterXdgFailClosed: contrato XDG exportado, 0 implementadores, fail-closed sin wiring"
status: open
severity: low
priority: P3
created: 2026-09-27
discovered_by: session-14 audit (code-based)
cluster_id: CL-UNFULFILLED-CONTRACT
fingerprint: "sddk_writer_xdg_trait_unimplemented_v1"
---

## Observación

`crates/sddk-cli/src/writer.rs:117` define el trait `WriterXdgFailClosed`
con un método `write` por defecto que valida la ruta de salida contra
`xdg_data_dir()` y falla con `PermissionDenied` si escapa. El trait
tiene **0 implementadores** en el repo y está marcado
`#[allow(dead_code)]`.

## Por qué NO es un hallazgo oculto (a diferencia de los anteriores)

El propio código lo declara explícitamente:

```rust
#[allow(dead_code)]
// Draft ADR-D foundation: trait is defined but not yet implemented.
// Retained to anchor the ADR and prevent drift during research phase.
```

Esto es **deuda declarada con intención**: el trait existe para anclar
un ADR en curso, no por descuido. `validate_xdg_output` sí tiene
implementación y tests; lo que falta es un writer que lo consuma.

## Por qué sigue siendo deuda

- El contrato "fail-closed XDG" está definido pero **no ejercido por
  ningún writer real**. Cualquier escritura que_no pase por
  `validate_xdg_output` no tiene la garantía, y nada lo fuerza.
- El `#[allow(dead_code)]` es el patrón habitual por el que la deuda
  declarada se vuelve invisible: el lint deja de señalar que nada usa
  el contrato, y el ADR puede quedar meses sin avance sin señal.

## Recomendación

- Cuando el writer exista, **quitar el `#[allow(dead_code)]`** en el
  mismo commit: el lint debe volver a vigilar el contrato.
- Considerar un test de arquitectura que afirme que todo writer de
  salida implementa `WriterXdgFailClosed` (o equivalente), para que la
  frontera se vuelva exigible en vez de declarativa.

## Estado

Abierto. Sin acción inmediata. Es deuda de proceso ligada al avance de
un ADR en curso, no un defecto de la implementación actual.
