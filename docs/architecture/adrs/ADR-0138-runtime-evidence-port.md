# ADR-0138: Runtime Evidence Port (Chronos MCP)

## Contexto

El VerifyKernel solo disponía de evidencia estática (grafo de arquitectura,
CogniCode static provider). AIW-S5 añade evidencia de ejecución real:
observaciones de runtime capturadas durante la ejecución de un programa
bajo un time-travel debugger.

## Decisión

- Nuevo módulo root-level `runtime_evidence_port` en `sddk-engine`:
  trait `RuntimeEvidencePort` con `RuntimeCaptureRequest/Result`,
  `ObservationSet` del port y `RuntimePortError`. Mismo envelope que el
  static provider (JSON en `content[0].text`, protocolo MCP 2025-03-26).
- Adapter `runtime_evidence_port_mcp::ChronosMcpAdapter`: spawn de
  `chronos-mcp`, handshake, probe_start→drain→stop→get_execution_summary,
  digest SHA-256 sobre material canónico. `exit_status` queda `None`
  deliberadamente: el summary no lo expone y no se infiere éxito.
- Nuevo dominio de verificación `runtime_provider` con claim falsable
  `RuntimeProviderClaim { subject_tag, contract_id, min_events }`:
  afirmar con eventos < umbral produce `Unknown(insufficient_runtime_capture)`,
  nunca `Verified`. Origin `StaticProvider` nunca satisface claims runtime.
- `ProviderKind` NO gana variante Chronos; conectar Chronos no declara
  capability enhancement (A7), igual que S1 no declaró STATIC_ENHANCED.

## Consecuencias

- El CLI expone `verify-kernel --domain runtime_provider --provider-bin …`.
- Tests EXT env-gated vía `CHRONOS_MCP_BIN` (skip si el provider no está).
- La captura consume una ejecución; la persistencia de observaciones
  runtime en el paquete publicado queda designada a AIW-S2.
