---
name: code-example-generator
description: "Trigger: crear ejemplo, generar ejemplo de código, crate de capítulo, proyecto de ejemplo, añadir ejemplo al workspace. Crea ejemplos como crates del workspace bevy-libro-examples (chapters/chapter-{NN}-{slug}/) con tests headless, respetando la convención del repo y la decisión pedagógica de la code card."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.1"
---

## Activation Contract

Úsalo cuando `code-integration-architect` ha identificado un gap (concepto sin crate) o `chapter-planner` necesita un ejemplo nuevo. El ejemplo es **un crate del workspace**, no un proyecto suelto.

No lo uses para verificar (`code-example-verifier`), escribir prosa (`chapter-writer`), ni para decidir *qué* ejemplo (`code-integration-architect` + `code-pedagogy-justifier`).

## Hard Rules

- Lee **`planning/stack-profile.yml`** (de `book-stack-detector`) antes de crear nada. Es la fuente de verdad del stack.
- Lee **`book-context/CONVENTIONS.md`** del proyecto (de `book-memory-keeper`) para las convenciones del repo concreto.
- El ejemplo vive en la **ruta y convención que dicta el stack-profile**, no en una hardcoded.
- **Naming y dependencias**: según la convención del stack (cargo workspace, npm workspace, monorepo...).
- **Tests headless cuando aplique**: respeta `stack-profile.headless`.
- Respeta el **principio pedagógico universal**: prefiere conceptos testables sobre APIs difíciles de probar en CI.
- Las regiones mostrables se etiquetan con el mecanismo del lenguaje (`// tag::` en Rust, `# region` en Python...).

## Execution Steps

1. Leer `planning/stack-profile.yml` y `book-context/CONVENTIONS.md` (cargados por el orchestrator al inicio de sesión).
2. Leer la code card de `code-pedagogy-justifier` (si existe).
3. Confirmar el número de capítulo y slug desde `planning/outline.yml`.
4. Crear el ejemplo en la ruta/convención del stack:
   - **Rust/Cargo**: `chapters/chapter-{NN}-{slug}/` con `Cargo.toml` workspace member.
   - **Python/uv**: paquete en `chapters/chapter_{nn}_{slug}/`.
   - **Go**: módulo o paquete en `chapters/chapter{NN}-{slug}/`.
   - **JS/TS**: paquete en `packages/chapter-{NN}-{slug}/` (workspace).
   - **Java/Maven**: módulo en `chapters/chapter-{NN}-{slug}/`.
   - Para stacks no listados, seguir la convención del repo concreto (ver CONVENTIONS.md).
5. Implementar la lógica + tests headless (cuando aplique).
6. Etiquetar regiones con el mecanismo del lenguaje.
7. Delegar a `code-example-verifier` (obligatorio) antes de dar por terminado.
8. Actualizar `examples/index.yml` del repo con el nuevo ejemplo.

## Plantillas por stack

El generador elige plantilla según `stack-profile.primary.language`:
- **Rust/Cargo**: ver `assets/crate-template.toml` (workspace member).
- **Python**: ver `assets/python-package-template/`.
- **Go**: ver `assets/go-package-template/`.
- **JS/TS**: ver `assets/js-package-template/`.
- **Java/Maven**: ver `assets/maven-module-template/`.
- **Otros**: el autor define la plantilla en `book-context/CONVENTIONS.md` y la usa.

## Decision Gates

| Necesidad | Acción |
|-----------|--------|
| Concepto requiere GPU/render | Marcar como `stub` (compila, tests mínimos) — ver chapter-14+ del repo |
| Concepto difícil de testear headless | `code-pedagogy-justifier` decide el enfoque (ej. conceptos subyacentes) |
| Necesita crate externo | `source-researcher` verifica versión real antes de añadirlo |
| Ejemplo muy complejo | `example-complexity-controller` |

## Output Contract

- Crate completo en `chapters/chapter-{NN}-{slug}/`.
- Regiones `tag::` documentadas para `code-integration-architect`.
- Resultado de `code-example-verifier` (debe ser verde).
- `code-integration-architect` actualiza el code-map.

## References

- `~/.zcode/skills/BOOK-REPO-CONTEXT.md` — convenciones del workspace (fuente de verdad).
- `assets/crate-template.toml` — plantilla de Cargo.toml workspace member.
- `references/testable-headless-patterns.md` — cómo escribir ejemplos GPU-free para CI.
