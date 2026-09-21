# SCOPE-CONTRACT — AIW-S0/S1: A6 productor real → observación → Verify

**Base:** HEAD `d3194ff` (incluye paquete AIW). Roadmap: A6 `NOT STARTED`,
CC-S0 seam shipped (ADR-0137), A5-C v1.169.88 certificado. No se reabren A5-C,
SEC-1 ni CC-S0.

## Pre-flight S0 (hecho, 2026-09-19)

| Check | Resultado |
|---|---|
| Proveedor CogniCode REAL | **OBSERVED**: `cognicode-mcp v0.97.1` arranca en stdio (`--cwd <repo>`, MCP handshake standalone in-memory). Sandboxes podman por lenguaje (rust/python/java/ts/go/js) activos como systemd user units. |
| Transporte | stdio local. El endpoint remoto configurado en el editor (`127.0.0.1:9847/mcp`) NO tiene listener — usar stdio, no asumir HTTP. |
| Consumidor real | `verify_kernel_cmd::run_verify` (ObservationSet vacío hoy) — es el path a poblar. |
| Símbolos confirmados | `CodeIntelligencePort` (trait, `code_intelligence_port.rs:337`), Fake+Null providers, `DigestSha256` FNV-64 provisional (`016x`, línea 49), `arch-spec-021`/IPB-009 vigente, `Operator` IR en engine. |
| WorkItem | A6 (`CogniCode / STATIC_ENHANCED`), subestado: seam shipped, enhanced NOT delivered. |

## Alcance de AIW-S1 (propuesto, pendiente de admisión del operador)

1. Adaptador `CogniCodeMcpAdapter` que implementa `CodeIntelligencePort` sobre
   el binario real vía stdio (negotiate/analyze/cancel según el trait).
2. Normalizador `result → observation::SoftwareObservation` con basis completa
   (revision/scope/config/analyzer/provider/complete) y digest SHA-256 real
   (sustituyendo el FNV provisional para refs durable de A6; el FNV queda
   identificado como compat de spike).
3. `verify_kernel_cmd` consume ObservationSet NO vacío y produce veredicto
   justificado sobre UNA claim elegida (contrato arquitectónico concreto del
   propio repo SDDK como fixture).
4. Estados tipados `Unknown/Partial/Unavailable/Incompatible`; Base green sin
   proveedor (UAT-A09).

**Fuera de alcance:** importación de AST, segundo store (S1b solo si UAT-A10
demuestra carencia),parsers multi-familia, handoff (S3), expansión (S4),
cualquier cambio a A5-C/SEC-1/CC-S0.

**UAT rector:** A01 (EXT+E2E), A03 (NEG), A05 (IT), A06 (digest), A09 (REG) de
`docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md`.
Estado inicial: todas NOT_RUN.

## Condiciones de fallo

- Sin proveedor ejecutable en el entorno de test → S1 queda BLOCKED por
  capacidad; se registra y no se declara enhanced.
- Si el normalizador no puede garantizar basis completa con el resultado real
  del MCP → fail-closed, se documenta el hueco de shape (posible entrada a S1b).
