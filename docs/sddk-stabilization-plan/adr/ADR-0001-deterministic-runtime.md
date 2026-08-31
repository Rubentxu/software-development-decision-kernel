# ADR-0001 — Runtime determinista Rust como autoridad del workflow

**Estado:** aceptada
**Fecha:** 2026-08-03

## Contexto

La lógica del workflow está repartida entre agentes, documentos y comandos shell. Las mismas reglas pueden existir con implementaciones distintas y no hay una única autoridad capaz de validar transiciones y efectos.

## Decisión

Crear un CLI Rust `sddk` y un módulo `sddk-engine` como únicos propietarios de:

- Estado y fase del ciclo.
- Gates.
- Locks.
- Idempotencia.
- Ejecución de capacidades.
- Confirmación de transiciones.
- Registro causal.

Los agentes producirán intenciones y resultados estructurados, pero no modificarán directamente el estado autoritativo.

## Consecuencias positivas

- Reglas verificables y testeables.
- Reducción de contradicciones.
- Recuperación tras fallos.
- Errores tipados.
- Evolución controlada.

## Consecuencias negativas

- Migración inicial considerable.
- Necesidad de mantener schemas.
- Parte de la flexibilidad informal de los prompts desaparece.

## Alternativas rechazadas

### Mantener scripts Bash como autoridad

No ofrece tipado, atomicidad ni reconciliación suficientes.

### Centralizar toda la lógica en un único prompt

Sigue dependiendo del comportamiento probabilístico del modelo.

## Criterios de cumplimiento

- Una transición no puede ejecutarse escribiendo texto en un resultado de agente.
- Todo efecto externo pasa por una capacidad declarada.
- La misma entrada y estado producen el mismo plan lógico.
