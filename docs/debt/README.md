# Debt Documentation

This directory contains the canonical contracts for the durable debt remediation framework (ADR-0047).

## Files

- **[SEVERITY.md](./SEVERITY.md)** — Severity taxonomy (`critical | high | medium | low`). Intrinsic technical impact, independent of scheduling.
- **[PRIORITY.md](./PRIORITY.md)** — Priority taxonomy (`P0 | P1 | P2 | P3`). Remediation scheduling, distinct from UAT priority namespace.
- **[debt-report.schema.json](./debt-report.schema.json)** (v1.0.0, draft-07) — JSON Schema for the per-cycle debt report.
- **[INCIDENCE-TEMPLATE.md](./INCIDENCE-TEMPLATE.md)** — Template for `INC-NNN-{slug}.md` cross-cycle records.
- **[INC-DEBT-023](./INC-DEBT-023-lints-advisory-no-expansion-cycle.md)** — Lints advisory sin ciclo de expansión programado (low/P3, closed; decisión registrada 2026-09-20: advisory by design, expansión orgánica, sin ciclo que programar).
- **[INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH](./INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH.md)** - Un commit de metadata derivada (`MANIFEST.sha256`) no tiene camino admisible de push sin quemar un bump (medium/P2, closed; regla (C) generated-only en `githooks/pre-push`, cycle inc-derived-metadata-push-path 2026-09-20).
- **[INC-FINDING-A5-2-DW-RUNTIME-003-CLOCK-SKEW](./INC-FINDING-A5-2-DW-RUNTIME-003-CLOCK-SKEW.md)** - Commentario de anclaje DW-RUNTIME-003 en restart_survival.rs desalineado con lo que el test hace (low/P3, closed; cycle a5-2-2026-09-17).

### Session-16 (2026-09-27) — auditoría con ejecución real del e2e

Todos los hallazgos verificados ejecutando el instalador y la suite e2e en contenedor, no por lectura de código.

- **[INC-DEBT-021-MUSL-ASSET-NAME-LIE](./INC-DEBT-021-MUSL-ASSET-NAME-LIE.md)** (high/P1, open) — El asset `sddk-<tag>-sddk-linux-x86_64-musl.tar.gz` contiene un binario **glibc**, no musl. `release.sh` compila con `cargo build --release` del host y lo empaqueta con nombre musl. Un usuario con glibc < 2.39 no puede ejecutarlo. Pineado por el check 8 de `tests/test_install_asset_contract.sh`.
- **[INC-DEBT-022-INSTALLER-ASSET-NAME-404](./INC-DEBT-022-INSTALLER-ASSET-NAME-404.md)** (critical/P1, **closed**) — `install.sh` pedía el asset `sddk-linux-x86_64-musl`, que el release nunca publicó (el asset real es `sddk`). HTTP 404 y abort antes de enlazar nada, en la ruta que se toma sin `gh` en PATH. Corregido en session-16 y protegido por `tests/test_install_asset_contract.sh`.
- **[INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY](./INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY.md)** (high/P1, open) — **Opción (a) re-evaluada**: session-14 la estimaba como "coste bajo, reutiliza infra existente". Falso. `install.sh` no menciona cosign, `release.sh` no firma nada, ningún release publica asset de firma, y el e2e exigía una cadena que nadie imprimía. Cerrar (a) es trabajo desde cero, con trust root por decidir.

### Session-14 (2026-09-27) — auditoría basada en código

Detalle y evidencia de reproducción en cada fichero.

- **[INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY](./INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY.md)** (high/P1, open) — El `.sha256` que valida el bundle se descarga del mismo origen que el bundle: integridad sí, autenticidad no. La opción (b) (guard de traversal en `dev/update`) quedó **cerrada** en session-14; la firma out-of-band sigue abierta.
- **[INC-AUDIT-S14-TEST-PORTS-UNCONSUMED](./INC-AUDIT-S14-TEST-PORTS-UNCONSUMED.md)** (medium/P2, open) — 9 traits del SPI de SPEC-043 §4 con implementadores internos pero sin consumidor externo. **Revisado** desde high/P1: la recomendación original de borrar queda anulada.
- **[INC-AUDIT-S14-NO-STRUCTURED-LOGGING](./INC-AUDIT-S14-NO-STRUCTURED-LOGGING.md)** (medium/P2, open) — 0 `tracing` en `sddk-cli`; sin correlación de `cycle_id` ni `request_id` en fallos.
- **[INC-AUDIT-S14-RELEASE-FORCE-VERSION-ERGONOMICS](./INC-AUDIT-S14-RELEASE-FORCE-VERSION-ERGONOMICS.md)** (low/P2, open) — El override de versión funciona pero su señal queda enterrada en un `warn` de un log de 400 líneas. No es un bug: el flag está plumbéado (`release.sh:361-363`).
- **[INC-AUDIT-S14-PACK-UAT-NO-CRATE-CONSUMERS](./INC-AUDIT-S14-PACK-UAT-NO-CRATE-CONSUMERS.md)** (low/P3, open) — `sddk-pack-uat` tiene 0 consumidores como crate, pero es un pack de ADR-0104, no código muerto. No borrar.
- **[INC-AUDIT-S14-WRITER-XDG-TRAIT-UNIMPLEMENTED](./INC-AUDIT-S14-WRITER-XDG-TRAIT-UNIMPLEMENTED.md)** (low/P3, open) — `WriterXdgFailClosed` exportado con 0 implementadores; deuda ya declarada en el propio código.

## Source of truth

- [ADR-0047 — Remediación durable y priorizada de deuda técnica](../adr/ADR-0047-durable-debt-remediation.md)

## Status

- cycle-7a: ratified status, severity+priority taxonomies published (this directory).
- Cycle-7a + 7b: runtime contracts live.

## LOC policy

The project's LOC budget policy is documented in [ADR-0048](../adr/ADR-0048-loc-budget-policy-reformulation.md). The policy uses **total-module-sum budgets** (implementation + boilerplate + test fixtures) rather than per-file targets. Per-file targets are deprecated as of cycle-10.
