# DISCOVERY — AIW-S5: proveedor Chronos runtime real

Fecha: 2026-09-19. Proveedor: `chronos-mcp` v0.1.0
(`/var/home/rubentxu/Proyectos/rust/chronos/target/release/chronos-mcp`).

## Caracterización (OBSERVED)
- Transporte: stdio JSON-RPC, protocolo **2025-03-26**, logs a stderr,
  respuestas de tools como JSON dentro de `content[0].text` — mismo
  envelope que cognicode-mcp (S1). Handshake verificado.
- Store: redb (`CHRONOS_STORE_PATH`); fallback a in-memory si el lock
  falla (WARN, no bloquea).
- 39 tools: probe_start/probe_drain/probe_stop (live probe), save/load
  session, query_events, get_execution_summary, inspect_causality,
  debug_*, tripwire_*.
- probe_start requiere `program`; las demás tools de sesión requieren
  `session_id` (devuelto en el JSON de probe_start).
- Escenario controlado y repetible verificado: binario C trivial
  (3 ticks, exit 0), probe → **68 eventos** (34 syscall_enter, 33
  syscall_exit, 1 custom), summary tipado con `duration_ns`,
  `event_counts_by_type`, `thread_count`.
- Caveat: `language: "Unknown"` en binarios sin debuginfo completo;
  `top_functions` vacío en este escenario.

## Elección de relación observable
`get_execution_summary` sobre probe de un binario objetivo: relación
**ejecutó/completó** (Affirms: el programa completó con N eventos y
exit code observado). Falsable: claim "program-completes" se contradice
si el summary muestra fallo o el probe no puede capturar el proceso.

## Limitaciones conservadas
- `probe_stop` sobre proceso ya terminado lo mata forzadamente tras
  timeout del loop (comportamiento del backend ptrace, anotado).
- S5 NO declara A7 enhanced: solo demuestra cadena vertical runtime real
  → observación → VerifyKernel, igual que S1 para estático.
