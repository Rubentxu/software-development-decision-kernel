# RECEIPT — AIW-S5 chronos runtime evidence port

## Scope cumplido
- Port `RuntimeEvidencePort` + `ChronosMcpAdapter` (runtime_evidence_port.rs / runtime_evidence_port_mcp.rs).
- Dominio de verificación `runtime_provider` con claim falsable `RuntimeProviderClaim { subject_tag, contract_id, min_events }`.
- CLI: `sddk verify-kernel --domain runtime_provider --provider-bin <chronos-mcp> --subject <program> --claim <contract>:<min-events>`.
- Tests unitarios del dominio (4) + tests EXT env-gated `CHRONOS_MCP_BIN` (3).

## Evidencia real (OBSERVED)
- `cargo test -p sddk-engine --lib adapter_runtime` → 4 passed; 0 failed.
- `CHRONOS_MCP_BIN=... cargo test -p sddk-engine --test aiw_s5_chronos_real` → 3 passed; 0 failed (contra chronos-mcp v0.1.0 real, protocolo 2025-03-26).
- E2E CLI real: `verify-kernel --domain runtime_provider --provider-bin /var/home/rubentxu/Proyectos/rust/chronos/target/release/chronos-mcp --subject /tmp/chronos-s5-target/t --claim e2e-s5:1` →
  `capture: session=07602559-… events=86` → `verdict: Verified`, exit=0.
- Corrida previa con min_events=10: events=35 → Verified, exit=0.
- Gate scoped: `cargo fmt -p sddk-engine -p sddk-cli -- --check` limpio; `cargo clippy -p sddk-engine -p sddk-cli --all-targets` → 0 warnings/errores.

## Comportamiento falsable (verificado)
- affirm + events >= min → Verified.
- affirm + events < min → Unknown { Custom("insufficient_runtime_capture: …") } (nunca Verified).
- deny → Contradicted. Origin StaticProvider nunca satisface claim runtime.
- Capture sin eventos (flake del runtime) → Unknown fail-closed, no éxito falso.

## Limitaciones conservadas
- `exit_status: None` a propósito: el execution summary de chronos no lo expone; no se infiere éxito.
- `ProviderKind` sin variante Chronos: A7 "RUNTIME_ENHANCED" NO se declara por conectar Chronos.
- Flake observado una vez: probe capturó 0 eventos (session válida, duration 0). El dominio respondió Unknown.

## No hecho (fuera de scope S5)
- Persistencia de observaciones runtime en el paquete publicado (designado a S2).
- S6/A8 (requieren S5+S1): disponibles como siguiente slice.
