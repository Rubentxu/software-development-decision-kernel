---
name: code-example-verifier
description: "Trigger: verificar código del libro, compilar workspace, ejecutar tests, fmt check, lint check, validar ejemplos compilables, stack-agnostic, multi-stack. Compila y prueba los ejemplos del workspace del libro ejecutando la cadena de verificación del stack-profile (fmt/check/test/lint). Es la puerta de calidad técnica: el código mostrado es el mismo que se prueba."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.2"
---

## Activation Contract

Úsalo **siempre** después de `code-example-generator` y **antes** de que un capítulo se publique. Sin verificación verde, el capítulo está bloqueado. Es la puerta técnica que impide el problema central del libro Bevy (código que no compila) y que se generaliza a **cualquier stack**.

No lo uses para coherencia prosa↔código (`code-prose-coherence-checker`): esa valida el significado; esta valida que compile y pase.

## Hard Rules — el artefacto único

> El código **mostrado** en el libro y el código **probado** en el workspace deben ser la misma región (`tag::` o equivalente del lenguaje).

- Ejecuta la **cadena exacta del `stack-profile`**, nunca una hardcoded.
- Lee `planning/stack-profile.yml` (de `book-stack-detector`) para los comandos exactos.
- Stack-agnostic: sirve para Rust, Python, Go, JS, Java, C++... Lo que diga el stack-profile.
- Un ejemplo que no compila o no pasa tests **bloquea** su capítulo.
- Registra el resultado por ejemplo en `build/verify-report.jsonl`.
- Si hay stack secundario (multi-stack), ejecuta la cadena de cada uno.

## La cadena de verificación

NO está hardcoded en esta skill. Sale de `stack-profile.yml`:

```yaml
# ejemplo stack-profile.yml (Rust)
stack:
  primary:
    fmt_tool: "cargo fmt --all --check"
    lint_tool: "cargo clippy --workspace --all-targets --locked -- -D warnings"
    test_runner: "cargo test --workspace --locked"
    build_tool: "cargo check --workspace --all-targets --locked"
```

El script `assets/run-stack-verify.sh` lee el stack-profile y ejecuta la cadena del stack activo. Funciona para Rust, Python, Go, JS, etc. (cada lenguaje tiene su bloque de comandos en el script).

## Execution Steps

1. Leer `planning/stack-profile.yml`.
2. Determinar el alcance: ¿workspace entero, o un ejemplo concreto?
3. Ejecutar la cadena del stack-profile (delegar a Bash con `assets/run-stack-verify.sh`).
4. Para cada fallo, clasificar:
   - `COMPILE_ERROR` / `BUILD_ERROR` → `code-example-generator`.
   - `TEST_FAILURE` → `code-example-generator` (o `chapter-writer` si la lógica está mal).
   - `LINT_WARNING` → `code-example-generator` (corregir antes de proseguir).
   - `FMT_ERROR` → formateo automático.
   - `VERSION_MISMATCH` (dependencia externa vs declarada) → `source-researcher`.
5. Emitir `build/verify-report.jsonl` (una entrada por ejemplo: path, stack, status, errores).
6. Estado global: `ALL_GREEN` o `BLOCKED`.
7. Si hay stack secundario, ejecutar su cadena también y agregar resultados.

## Decision Gates

| Fallo | Acción |
|-------|--------|
| Ejemplo no compila/build | Devolver a `code-example-generator` |
| Test falla | Devolver a `code-example-generator` con el fallo exacto |
| Lint con `warnings as errors` | Corregir (no es opcional) |
| Dependencia externa con versión incorrecta | `source-researcher` verifica la real |
| Ejemplo requiere GPU/externo (no headless) | Aceptable como `stub` si compila (verificar solo build) |

## Output Contract

- `build/verify-report.jsonl` (una entrada por ejemplo: path, stack, status, errores).
- Estado: `ALL_GREEN` o `BLOCKED`.
- Si `BLOCKED`, lista exacta de fallos + skill responsable.
- `book-builder` y capítulo en `done` requieren `ALL_GREEN` para los ejemplos que referencian.

## References

- `~/.zcode/skills/book-stack-detector/` — produce el stack-profile.
- `~/.zcode/skills/BOOK-REPO-CONTEXT.md` — plantilla tech-agnostic (la instancia concreta vive en `book-context/CONVENTIONS.md`).
- `assets/run-stack-verify.sh` — script genérico que ejecuta la cadena del stack-profile.

