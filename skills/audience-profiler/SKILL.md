---
name: audience-profiler
description: "Trigger: perfil de lector, público objetivo, audience, nivel del libro, a quién va dirigido. Define el lector objetivo, conocimientos previos, profundidad y tecnologías conocidas antes de diseñar el currículo o el índice."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **antes** de `curriculum-designer` y `book-outline-architect`. Es la primera fase después de inicializar el proyecto. Define el contrato de quién es el lector.

No lo uses si el perfil ya existe en `planning/audience-profile.yml` y no ha cambiado.

## Hard Rules

- El perfil debe ser **específico y falsable**, no marketing ("desarrolladores" no sirve; "dev con 2+ años en Python, sin experiencia en Rust, que conoce git" sí).
- Debe declarar explícitamente qué **se asume** y qué **se enseña** en el libro.
- Idioma de salida: castellano por defecto, salvo que el libro sea en otro idioma.

## Execution Steps

1. Si existe `planning/audience-profile.yml`, leerlo y proponer diferencias.
2. Entrevistar al autor (1 ronda, máx. 6 preguntas) cubriendo:
   - Experiencia previa relevante (lenguajes, frameworks, tooling).
   - Conocimiento del dominio (ECS, sistemas distribuidos, etc.).
   - Objetivo del lector al terminar (¿construir un proyecto? ¿aprobar? ¿profundizar?).
   - Plataforma/OS de referencia (Windows/Linux/macOS).
   - Tolerancia a teoría vs. hands-on.
   - Restricciones de versión (¿book para 0.18? ¿0.19? ¿ambos?).
3. Generar `planning/audience-profile.yml` con el esquema de `assets/audience-profile.schema.yml`.
4. Devolver un resumen de 1 párrafo + el archivo.

## Schema de salida (resumen)

```yaml
audience:
  persona: "Desarrollador con experiencia en lenguajes gestionados..."
  assumed_knowledge: [git, POO, tipos básicos]
  taught_knowledge: [ownership de Rust, ECS, scheduling]
  not_in_scope: [rendering low-level, shaders]
  target_version: "Bevy 0.19 / Rust 1.95"
  platforms: [linux, windows, macos]
  theory_hands_on_ratio: "30/70"
  exit_goal: "Construir un juego 2D completo y mantenible"
```

## Output Contract

- `planning/audience-profile.yml` creado/actualizado.
- Resumen ejecutivo de 1 párrafo.
- Confirmación de que `curriculum-designer` puede consumir el archivo.

## References

- `assets/audience-profile.schema.yml` — esquema validable del perfil.
