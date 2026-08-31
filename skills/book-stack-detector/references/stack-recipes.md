# Recetas por stack

Comandos y convenciones verificadas por stack. `code-example-verifier` usa la cadena del stack-profile, que se construye desde aquí.

## Rust (cargo)
- Workspace: `members = ["chapters/*"]`, crate `book-chapter-{NN}`.
- Tests headless: `default-features = false` cuando el framework lo permite (ej. Bevy).
- Cadena CI: `cargo fmt --all --check` → `cargo check --all-targets --locked` → `cargo test --workspace --locked` → `cargo clippy --workspace --all-targets --locked -- -D warnings`.

## Python (uv/pytest)
- Estructura: un paquete por capítulo o un monorepo con `chapters/chapter_{nn}/`.
- Entornos: `uv` (recomendado) o `venv` + `pip`.
- Cadena: `ruff check .` → `ruff format --check .` → `pytest`.
- Tipos: `mypy` o `pyright` si el libro enseña typing.

## Go
- Módulo único o múltiples módulos por capítulo.
- Cadena: `gofmt -l .` → `go vet ./...` → `go test ./...` → `golangci-lint run`.

## JavaScript/TypeScript
- Workspace npm/pnpm con un paquete por capítulo.
- Cadena: `prettier --check .` → `eslint .` → `vitest run` (o `jest`).
- Tipos (TS): `tsc --noEmit`.

## Java
- Maven o Gradle, un módulo por capítulo.
- Cadena: `mvn -q test` (+ `spotless:check`, `checkstyle:check`, `spotbugs:check`).

## C/C++
- CMake por capítulo o un CMake umbrella.
- Cadena: `cmake --build` → `ctest` → `clang-format --dry-run` → `clang-tidy`.

## Multi-stack
- Un libro fullstack (ej. "webapp con Go backend + React frontend"): bloque `secondary` con su propia cadena.
- `code-example-verifier` ejecuta la cadena de cada stack y agrega el resultado.

## Headless
- Si los tests requieren GPU, navegador, servicio externo: `headless: false`.
- `example-complexity-controller` y `code-example-verifier` marcan esos ejemplos como `stub` (compilan, tests mínimos) igual que el repo Bevy hace con los capítulos de rendering.
