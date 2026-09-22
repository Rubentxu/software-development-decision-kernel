# CURRENT — puntero de reanudación de SDDK

**Estado:** C1 cerrado; C2 NOT_EVALUATED (3 sub-cycles systemic); C3a/b/c/d/e/f/g/h PASS_OBSERVED. **C4 v1.171.0 — PASS_PARTIAL_OBSERVED** (release publicada y binario en PATH, pero certificación formal retractada por session-12 audit al esquema CERTIFICATIONS §5). Override SemVer LIFTED (v1.171.0 es SemVer-correct minor). Actualizado: 2026-09-22T18:11:00Z (session-12 reconciliation). Este puntero se revalida al comienzo de cada sesión.

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@a8e77f4b6c03276f62e7d8223ab50f293e3b4fff` (HEAD = docs commit post-release) — consultado 2026-09-22T18:11Z; **revalidar al comenzar cada sesión**. Tag `v1.171.0` → commit `db1e2e44bc64034f44238b6cf250e6bddaa6addb`. |
| Workspace en esa fotografía | `1.171.0` (Cargo.toml). Workspace == tag (cumple AGENTS.md §2.3). |
| Release pública comprobada en esa fotografía | `v1.171.0` (2026-09-22T17:11:44Z) — GH Releases Latest. Binary sha256 `5e9d5fbd17d94b8c53763cdca0a70435b8eabedcd72e521cce21f1c2335ef9f3`. PATH sha256 matches GH asset. |
| Hito activo | C1 CERRADO; C2 NOT_EVALUATED (systemic, provider MCP bridges ausentes); C3a-h PASS_OBSERVED; **C4 v1.171.0 PASS_PARTIAL_OBSERVED** (release OK; full certification deferred — see CERTIFICATION-RECEIPT.yaml). Override SemVer LIFTED. |
| Estado PRs abiertos | Ninguno. |
| Commits this session (session-11 close v1.171.0 + session-12 audit) | session-11: 9 nuevos desde v1.170.3 → `db1e2e4` tag. session-12: 1 docs commit `a8e77f4` (RECEIPT + UAT-EVIDENCE + CURRENT/STATE). Working tree: audit docs `CERTIFICATION-RECEIPT.yaml` + `AUDIT-session-12.md` + `FEATURE-CANDIDATES.md` corrections (NO commiteados, awaiting operator review). |
| Working tree uncommitted (session-12) | `docs/roadmap/receipts/c4-release-v1.171.0/CERTIFICATION-RECEIPT.yaml` (new, 236 lines, schema-compliant CERTIFICATIONS §5); `docs/debt/AUDIT-session-12.md` (new, 62 lines, INC closure evidence inventory); `docs/roadmap/FEATURE-CANDIDATES.md` (FC-3 + FC-5 marked DUPLICATED). FC-3 working tree code REJECTED — would duplicate `sddk ledger export --cycle`. |
| Tests verified this session | `cargo test -p sddk-cli --lib` → 787 passed (session-11: 784; +3 are from FC-3 working tree tests, not yet committed). `cargo test -p sddk-cli --test cli` → 186 passed (session-11: 185; +1 is FC-3 integration test, not yet committed). cargo fmt/clippy clean on FC-3 working tree (so it could be reverted without harm). |
| Real-provider binary availability | `cognicode` CLI v0.97.3 presente en PATH; `cognicode-mcp` AUSENTE (typer not installed for `mcp` CLI bridge). `chronos-mcp` AUSENTE. `jcode` v0.86.0 presente; `jcode-sdk` no publicado. **C2 sigue NOT_EVALUATED.** |
| Siguiente acción exacta | **Operator decision** sobre working tree de session-12: (A) commitear docs (CERTIFICATION-RECEIPT.yaml + AUDIT-session-12.md + FEATURE-CANDIDATES corrections) y descartar código FC-3 → release v1.171.1 (docs-only = patch auto, sin override); (B) descartar todo el working tree session-12 y volver al cierre session-11; (C) abrir nuevo ciclo C5 con feature genuina (FC-1 uat run --filter, valor confirmado; FC-4 uat replay --release, valor a confirmar). |
| Evidencia requerida para mover puntero | Para C4 PROMOTION a `CERTIFIED_BASE`: ejecutar C2 con providers reales (cognicode-mcp + chronos-mcp + jcode-sdk instalados), re-run T01-T35 contra el SHA de release, ejecutar `tests/clean_machine_uat.sh --tag v1.171.0` en podman. Para C4 PROMOTION a `PASS_OBSERVED` (manteniendo BASE): el CERTIFICATION-RECEIPT.yaml ya está schema-compliant. |
| Bloqueos y decisiones | **C2 sigue cerrado honesto NOT_EVALUATED** (receipts c2a/c2b/c2c). **C3a-h cerrado PASS_OBSERVED**. **Override SemVer LIFTED en v1.171.0**; próxima release con solo fix:/test:/docs:/chore: será patch (v1.171.1) sin override. **FC-3 RECHAZADO por duplicación con `sddk ledger export --cycle`** (audit finding session-12). J7/J8/J9/X08/R11 siguen DEFERRED. |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.

## Estado de certificación (al cierre de session-12 / 2026-09-22T18:11Z)

- **C1 (Base)**: cerrada con full profile 4998/0/15 sobre e7968f8; certificados H02/H05+H06/cycle-c.
- **C2 (Integraciones reales)**: **NOT_EVALUATED_PROVIDER_MISSING (C2a, C2b) / NOT_EVALUATED_ADAPTER_MISSING (C2c)**. Cierre honesto documentado en `docs/roadmap/receipts/c2a/`, `c2b/`, `c2c/`. Status systemic; recovery requires operator decision (instalar `cognicode-mcp` con typer, instalar `chronos-mcp`, publicar `jcode-sdk`).
- **C3a-h**: TODOS **PASS_OBSERVED** — Authority hardening (C3a), Storage adversarial (C3b-c), Performance baseline (C3d), Schema resilience (C3e), Migration re-application safety (C3f, closes C3e-F1 via ADR-0141), Performance budget harness (C3g), Supply-chain audit + remediation (C3h, 0 vulns).
- **C4 (Release y certificación de producto)**: **v1.171.0 PASS_PARTIAL_OBSERVED** — release publicado en GH como Latest (2026-09-22T17:11:44Z). CERTIFICATION-RECEIPT schema-compliant emitido en `docs/roadmap/receipts/c4-release-v1.171.0/CERTIFICATION-RECEIPT.yaml` (236 líneas, schema_version=1 per CERTIFICATIONS.md §5). Resumen de gates:
  - 4 gates **PASS_OBSERVED** (T28 full verify, T29 install, T31 SHA coherence, T33 provider-absence preserved Base).
  - 12 gates **HISTORICAL_CARRY_OVER** (G0..G6, G9, G10, G12..G15) con respaldo en `cargo test --workspace` 5037/0/5.
  - 1 gate **NOT_VERIFIED** (G11 security/secrets — R14 OPEN_NON_BLOCKER since A5-C).
  - 4 UAT scenarios **NOT_RUN** (T08-T18 C2 providers, T23 adversarial, T26 performance, T30 cross-version, T32 failed-gate adversarial).
- **C5 (Evolución condicionada)**: pendientes P2/P3 (J7/J8/J9/X08/R11), ninguno activo.
- **Session-12 audit findings** (operador: "CLOSED != CERTIFIED"):
  - RECEIPT.md original (session-11) era un release receipt, NO un CERTIFICATION-RECEIPT (schema §5). Ahora reemplazado por CERTIFICATION-RECEIPT.yaml.
  - 40 INCs declaradas `status: closed` fueron auditadas; todas tienen evidencia de cierre (commit refs, test names, exit codes, matrix case counts). NO hay cierres paperwork.
  - FC-3 working tree RECHAZADO: duplica `sddk ledger export --cycle X --output <file>`. La extensión propuesta es extender `LedgerExportArgs` con `--format text|json|jsonl` y `--output opcional` (FUTURO, no en este ciclo).
  - FC-5 working tree NO implementado pero marcado DEFERRED-DUPLICATED con `sddk fork diff` y `sddk memory diff`.

**Distinción importante**: workspace v1.171.0 == tag v1.171.0 publicado (alineado per AGENTS.md §2.3). El binario instalado en `~/.local/bin/sddk` y el asset GH coinciden (sha256 `5e9d5fbd...`). Próximo release: si solo fix:/test:/docs:/chore:, el algoritmo calculará patch (v1.171.1) sin override — retorno automático a SemVer-correct.
