---
name: book-stack-detector
description: "Trigger: detectar stack del libro, qué lenguaje usa el libro, configurar verificación, Python book, Go book, JavaScript book, Rust book, multi-lenguaje. Detecta el stack tecnológico del libro (lenguaje, build tool, test runner, package manager) y genera el stack-profile que configura generator, verifier y convenciones. Hace que el sistema sea agnóstico: sirve para cualquier libro técnico."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo en **A2c**, justo después de `book-project-initializer` (A1), en paralelo con `audience-profiler` y `editorial-voice-designer`. Es lo que hace que el sistema sirva para **cualquier libro**: detecta el stack y produce la configuración que `code-example-generator`, `code-example-verifier` y `code-integration-architect` consumen.

No lo uses para redactar, investigar ni revisar. Es **configuración del entorno**.

## Hard Rules

- Un libro puede ser **mono-stack** (solo Rust) o **multi-stack** (frontend JS + backend Go).
- El stack-profile es la **fuente de verdad** para todos los skills de código: nadie asume un lenguaje fijo.
- La detección se hace inspeccionando manifests reales (Cargo.toml, package.json, go.mod, pyproject.toml...), no preguntando al modelo.
- Si el repo de ejemplos no existe aún (libro nuevo), el perfil se deriva del `book-config.yml` + preguntas mínimas al autor.
- El perfil es **persistente** (`planning/stack-profile.yml` + Engram) para no redetectar cada sesión.

## Execution Steps

1. Si existe repo de ejemplos, inspeccionar manifests en la raíz y subdirectorios.
2. Si no existe, leer `book-config.yml` (campo `primary_tech`) y preguntar lo mínimo necesario.
3. Para cada lenguaje detectado, identificar:
   - `language` + `version`/`edition`.
   - `package_manager` (cargo, npm, pip/uv, go mod...).
   - `build_tool`.
   - `test_runner` + comando exacto.
   - `lint_tool` + comando exacto.
   - `fmt_tool` + comando exacto.
   - `workspace_convention` (cómo se organizan los ejemplos).
4. Generar `planning/stack-profile.yml` (esquema en `assets/stack-profile.schema.yml`).
5. Mapear a `code-example-verifier`: la cadena de verificación sale de aquí, no hardcoded.
6. Persistir vía `book-memory-keeper` (topic_key=`stack-{libro}`).

## Stack-profile (resumen)

```yaml
stack:
  primary:
    language: rust
    version: "1.96"
    package_manager: cargo
    build_tool: cargo
    test_runner: "cargo test --workspace --locked"
    lint_tool: "cargo clippy --workspace --all-targets --locked -- -D warnings"
    fmt_tool: "cargo fmt --all --check"
    workspace_convention: "chapters/chapter-{NN}-{slug}/, crate bevy-book-chapter-{NN}"
  secondary: []    # para multi-stack
  notes: "default-features = false para tests headless"
```

## Ejemplos por stack (referencia, no exhaustivo)

| Lenguaje | package_manager | test_runner | lint | fmt |
|----------|-----------------|-------------|------|-----|
| Rust | cargo | `cargo test` | `cargo clippy -D warnings` | `cargo fmt --check` |
| Python | uv/pip | `pytest` | `ruff check` | `ruff format --check` |
| Go | go mod | `go test ./...` | `golangci-lint run` | `gofmt -l` |
| JavaScript/TS | npm/pnpm | `vitest`/`jest` | `eslint` | `prettier --check` |
| Java | maven/gradle | `mvn test` | `spotbugs`/`checkstyle` | `spotless` |

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Multi-stack detectado | Un bloque `secondary` por lenguaje; verifier ejecuta cada cadena |
| Stack no listado | Definir manualmente con el autor y añadir a la tabla |
| Repo sin manifests | Preguntar y generar perfil desde book-config |
| Tests requieren GPU/externo | Marcar `headless: false` (afecta a complexity-controller y verifier) |

## Output Contract

- `planning/stack-profile.yml`.
- `code-example-generator`, `code-example-verifier` y `code-integration-architect` cargan este perfil.
- Persistido en Engram para no redetectar.

## References

- `assets/stack-profile.schema.yml` — esquema validable.
- `references/stack-recipes.md` — recetas detalladas por stack (comandos, convenciones, headless).
