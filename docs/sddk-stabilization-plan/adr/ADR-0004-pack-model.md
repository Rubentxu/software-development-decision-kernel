# ADR-0004 — Packs declarativos, compilados primero

**Estado:** aceptada
**Fecha:** 2026-08-03

## Contexto

SDDK necesita modularidad para Git, testing, arquitectura, forge, release, gestión de especificaciones y otras capacidades, sin volver a concentrar toda la lógica en un orquestador monolítico.

## Decisión

Introducir packs con `manifest.toml` que declaren:

- Identidad y versión.
- Compatibilidad.
- Dependencias obligatorias y opcionales.
- Comandos.
- Eventos.
- Capacidades.
- Riesgo y consecuencia.
- Artefactos.
- Fixtures deterministas.
- Hash de contenido y bundle.

En v3.6 los packs estarán compilados dentro del binario. La carga dinámica se aplaza.

## Consecuencias positivas

- Superficie auditable.
- Fixtures por responsabilidad.
- Posibilidad de sustituir adaptadores.
- Documentación generable.

## Consecuencias negativas

- Más metadatos y validadores.
- Requiere evitar una fragmentación excesiva de crates.

## Regla

Un bundle es una lista de packs; no contiene lógica ni ontología propia.
