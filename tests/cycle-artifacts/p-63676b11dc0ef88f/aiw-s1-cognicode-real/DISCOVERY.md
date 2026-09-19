# DISCOVERY — cognicode-mcp v0.97.1 (evidencia real, 2026-09-19)

## Handshake observado

- `initialize` OK: `serverInfo: cognicode 0.97.1`, protocolo `2025-03-26`, capacidades `resources` + `tools`.
- Transporte stdio; JSON-RPC por línea en stdout; los logs (INFO tracing) van por **stderr** (confirmado, no contamina el canal).
- Nota de integración: el servidor es lento en arrancar el reader — el adaptador debe esperar respuesta por id con timeout, no asumir bloqueo lineal.

## Catálogo (primera página: 20 tools; el total paginado reporta ~54k entradas: el cursor repite, verificar paginación en adaptador — sospecha de bug del cursor, no bloqueante para S1)

`build_graph, get_file_symbols, get_call_hierarchy, analyze_impact, find_usages, get_complexity, get_entry_points, get_leaf_functions, trace_path, export_mermaid, get_hot_paths, query_symbol_index, build_call_subgraph, get_per_file_graph, get_symbol_code, go_to_definition, hover, find_references, read_file, search_content`

## Llamadas reales ejecutadas sobre este repo (OBSERVED)

| Llamada | Resultado |
|---|---|
| `build_graph {strategy: lightweight}` | `{"success":true,"symbols_found":41607,"relationships_found":36548,...}` (~35 s con grafo frío) |
| `build_graph {strategy: full}` | OK (precondición de las tools de grafo) |
| `find_usages {symbol_name:"CodeIntelligencePort"}` | JSON con `usages[{file,line,column,context,is_definition}]` — **relación observable elegida para S1** |
| `analyze_impact {symbol_name:"build_operator"}` | `impacted_files[], risk_level, summary` con latencia real en `summary` |
| `analyze_impact {symbol_name:"CodeIntelligencePort"}` (grafo lightweight) | `impacted_files: []`, `risk_level: "low"` — resultado plausible pero vacío: **la calidad depende del grafo**; el normalizador debe distinguir vacío-por-grafo de vacío-real |

## Decisión de relación (S1)

**Relación:** `find_usages` de un contrato arquitectónico elegido (claim: «el trait
CodeIntelligencePort solo se usa desde el engine y su fake/null; ningún módulo del
CLI lo consume»), complementada con `analyze_impact` para riesgo de cambio.

**Formato de respuesta:** JSON de texto dentro de `content[0].text` — NO
`structuredContent`. El normalizador parsea ese JSON con fail-closed.

**Basis acreditable:** cwd del proveedor (repo+rev), versión del servidor
(0.97.1), nombre de herramienta, estrategia de grafo (lightweight/full),
latencia reportada, timestamp. La claim y su negación son comprobables con
`grep`+`find_usages`.
