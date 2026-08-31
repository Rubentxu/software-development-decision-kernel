# Contexto del repositorio de ejemplos del libro — PLANTILLA

**Plantilla tech-agnostic.** Cada libro concreto instancia este documento en su `book-context/CONVENTIONS.md` (lo mantiene `book-memory-keeper`). El caso Bevy/Rust al final es un **ejemplo ilustrado**, no la regla.

## Propósito

Fuente de verdad para todos los skills que tocan código del libro. Antes de generar, verificar o mapear código, lee la instancia del proyecto (`book-context/CONVENTIONS.md`) o el `planning/stack-profile.yml` (producido por `book-stack-detector`).

## Campos a instanciar por libro

- **Ruta del repo de ejemplos**: dónde vive el código del libro.
- **Tipo de workspace**: Cargo workspace, npm/pnpm workspace, monorepo Go, monorepo Python...
- **Toolchain**: versión del lenguaje + herramientas (linter, formatter).
- **Framework principal + versión** (si aplica): ej. Bevy 0.19, Django 5, React 19.
- **Convención de paquetes**: cómo se llama cada paquete/crate/módulo de capítulo.
- **Cadena de verificación (CI)**: los comandos exactos.
- **Principio pedagógico del repo**: cómo se aborda lo testeable vs lo que requiere entorno.

## Principios universales (no dependen del stack)

1. **Nunca crear un paquete suelto**: siempre dentro del workspace del repo, siguiendo la convención.
2. **Nunca fijar versiones a mano**: usar el mecanismo del workspace (workspace dependencies, lockfile...).
3. **Nunca inventar comandos de verificación**: usar la cadena de la CI del repo.
4. **Distinguir ejemplo ↔ libro**: el código vive en el repo de ejemplos; la prosa en el libro. Se acoplan por el code-map.
5. **Antes de tocar código**: leer este documento + el `stack-profile.yml`.

## Principio pedagógico universal

> **Preferir conceptos testables en CI sobre APIs difíciles de probar sin entorno especial.**

Si una API requiere GPU, navegador, servicio externo o estado global difícil, el ejemplo enseña el **concepto subyacente** testable y declara el tradeoff en la code card (`code-pedagogy-justifier`).

---

## EJEMPLO ILUSTRADO — Libro "Patrones 2D y ECS con Bevy 0.19"

*(Esto es una instancia concreta, no la regla. Otros libros tendrán otras instancias.)*

- **Repo**: `<your-book-repo-path>/`
- **Workspace**: Cargo (resolver=3, edition=2024), miembros `chapters/*`.
- **Toolchain**: Rust 1.96.0 (`rust-toolchain.toml` + clippy + rustfmt).
- **Framework**: Bevy pinned `=0.19.0`, `default-features = false` (tests headless).
- **Convención**: `chapters/chapter-{NN}-{slug}/`, crate `bevy-book-chapter-{NN}`.
- **CI**: `cargo fmt --all --check` → `cargo check --workspace --all-targets --locked` → `cargo test --workspace --locked` → `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- **Decisión pedagógica ejemplar**: el chapter-12 enseña `ChildOf`/`Children`/`#[require]` (testable headless) en lugar de `bsn!` directo (requiere contexto de render). Declarado en la code card.
