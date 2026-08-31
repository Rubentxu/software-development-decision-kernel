# Herramientas de extracción por lenguaje

## Rust
- `cargo metadata` — estructura de crates.
- `cargo doc --no-deps` — API pública renderizada.
- **rust-analyzer** — índice de símbolos, go-to-definition, call hierarchy.
- `cargo expand` — expansión de macros (clave para entender derives).
- tree-sitter (`tree-sitter-rust`) — parsing estructural programable.

## Go
- `go doc` — documentación de paquetes.
- `gopls` — LSP, referencias y call hierarchy.
- `go callgrap` / `pprof` — flujos de ejecución.

## TypeScript/JavaScript
- **tsserver** / `tsc` — AST y API exportada.
- `madge` — grafo de dependencias de módulos.

## Python
- `ast` + ` jedi`/`pyright` — símbolos y referencias.

## Orden de preferencia
1. Indexación LSP / árbol de símbolos del lenguaje (preciso).
2. AST con tree-sitter (programable, multi-lenguaje).
3. `grep`/búsqueda textual solo como último recurso.
4. Búsqueda semántica (embeddings) **última**: útil para descubrir, nunca para afirmar estructura.

## Anti-patrón
Confiar en búsqueda semántica para afirmar "este módulo hace X". La estructura la determina el compilador/analyzer, no un embedding.
