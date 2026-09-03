# SDDK Human-Agent Collaboration Evolution Pack

**Fecha:** 2026-08-28  
**Baseline auditada:** `sddk-framework` 1.50.0, `main@643180a21ab1c9e7a63758ad221d97ec1640ae5a`  
**Propósito:** dar continuidad al roadmap real de SDDK y convertir la interacción humano-agente en una capacidad arquitectónica de primer nivel sin degradar el kernel determinista, los gates, la evidencia ni la trazabilidad.

## Resumen ejecutivo

Este paquete propone **SDDK Companion / Human Interaction Plane** como el siguiente evolutivo transversal prioritario. No añade "más charla" a los prompts: introduce un protocolo explícito para que el usuario sepa siempre **dónde está el ciclo, qué se acaba de hacer, qué se ha descubierto, qué ha cambiado, por qué, qué viene después y cuándo necesita intervenir**.

La propuesta preserva las decisiones previas de SDDK:

- kernel Rust determinista y local-first;
- ledger operacional como autoridad de estado;
- grafo/vault como conocimiento y proyección, nunca como sustituto silencioso del runtime;
- evidencia, receipts, gates, capabilities y políticas como contratos verificables;
- cero intrusión en repositorios adoptados;
- UAT human-governed con `PASSED != ACCEPTED`;
- capa agent-first de CLI/facade con compresión semántica sin pérdida funcional;
- telemetría/F3 para mejora continua.

## Decisión de roadmap

Se introduce una nueva línea **HX — Human Experience / Human-Agent Collaboration**:

1. `HX0` — Canonical State & Baseline Reconciliation.
2. `HX1` — Human Interaction Domain + `CurrentRunView`.
3. `HX2` — Never Lost reporting + Resume.
4. `HX3` — Decision/Reframe/Assumption + risk-based HITL.
5. `HX4` — Audience + Personality + Autonomy renderers.
6. `HX5` — Preference Memory.
7. `HX6` — Friction Telemetry + F3.
8. `HX7` — UAT, dogfood y estabilización.

**Relación con UAT:** el core UAT F0–F11 puede evolucionar en paralelo. HX0–HX3 debe estar disponible antes de consolidar F12–F14 (Guided Runner/UX humano) para reutilizar el mismo protocolo de interacción y evitar dos modelos de UX incompatibles.

## Contenido

- `BASELINE.md` — estado actual y gaps.
- `PRD.md` — objetivo de producto, alcance, requisitos y no-objetivos.
- `ARCHITECTURE.md` — arquitectura propuesta.
- `ARCHITECTURE-EMERGENCE.md` — decisiones aplazadas, spikes y criterios para evolucionar.
- `ROADMAP-UPDATED.md` — roadmap actualizado y relación con milestones existentes.
- `MILESTONES-AND-EXIT-CRITERIA.md` — hitos medibles.
- `IMPLEMENTATION-PLAN.md` — orden de implementación y slices.
- `MIGRATION-AND-COMPATIBILITY.md` — migración sin ruptura.
- `PROMPT-CHANGE-MAP.md` — cambios concretos en prompts/skills.
- `TEST-STRATEGY.md` — tests deterministas, contract tests, golden tests y UAT.
- `TRACEABILITY-MATRIX.md` — objetivos → requisitos → ADRs → pruebas → milestones.
- `RISK-REGISTER.md` — riesgos y mitigaciones.
- `adrs/` — ADRs propuestos.
- `specs/` — especificaciones implementables.
- `roadmap/` — backlog y dependencias.
- `uat/` — plan UAT, escenarios y métricas.
- `schemas/` — schemas de contratos.
- `examples/` — ejemplos de configuración y salidas.

## Cómo integrarlo

1. Leer `BASELINE.md` y validar que el HEAD real no haya invalidado ningún supuesto.
2. Aceptar/rechazar los ADR `ADR-HX-*`; al integrar, reasignar numeración canónica si el repositorio lo exige.
3. Incorporar `ROADMAP-UPDATED.md` como delta del roadmap vigente.
4. Ejecutar `HX0` antes de implementar renderers o memoria.
5. Implementar HX1–HX3 en slices pequeños y dogfood.
6. Sólo después activar personalidad/memoria adaptativa.
7. Usar `uat/UAT-PLAN.md` como gate de aceptación del evolutivo.

## Principio rector

> **Semantic compression is not functional compression.**

El usuario y los agentes pueden utilizar una interfaz más simple, pero ninguna simplificación puede omitir reports, artifacts, gates, evidence, receipts, traceability, safety brakes o capacidad de depuración de bajo nivel.
