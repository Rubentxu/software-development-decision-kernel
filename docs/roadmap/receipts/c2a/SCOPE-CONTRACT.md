# C2a — SCOPE-CONTRACT: Real CogniCode integration (provider path)

**Cycle:** C2a (Static Enhanced path, T08–T11)
**Baseline:** `main@5f493ab` (workspace v1.169.142); release pública observada `v1.169.122`
**Opened:** 2026-09-22T07:57:00Z
**Owner:** orchestrator (delegates to `sddk-apply`)
**Authority basis:** AGENTS.md §3 (initiative preauthorizes ordinary continuity gates); ROADMAP.md §C2a + §3

## 1. Objective (falsable)

Reconciliar el adapter SDDK↔CogniCode con el binario real disponible en esta máquina (`cognicode` v0.97.3, comando `serve`) y demostrar T08–T11 del UAT-MATRIX con evidencia ejecutada, **no inferida por lectura de código**.

**Exit criterion:** T08, T09, T10, T11 cada uno con un test o ejecución real observable, recibo por escenario, y un único commit funcional verde. Sin `PASS_BY_CODE_READING`.

## 2. Findings pre-investigación (OBSERVED, this session)

| # | Hallazgo | Archivo / línea | Implicación |
|---|---|---|---|
| F1 | Binario real se llama `cognicode` (no `cognicode-mcp`); MCP server se arranca con `cognicode serve` | CLI `--help` ejecutado | Adapter necesita subcomando |
| F2 | `CogniCodeMcpAdapter::spawn` hace `Command::new(binary).arg("--cwd").arg(cwd)` directo | `crates/sddk-engine/src/code_intelligence_port_mcp.rs:74-80` | Falla contra binario real |
| F3 | CLI `sddk verify-kernel --domain static_provider --provider-bin` espera `cognicode-mcp` literal en mensaje de error | `crates/sddk-cli/src/verify_kernel_cmd.rs:172` | Mensaje user-facing a corregir |
| F4 | Tests `a6_cognicode_protocol_spike`, `a6_cc_s1_static_graph_completeness`, `aiw_s1_cognicode_real` ya referencian el binario | `crates/sddk-engine/tests/*` | Cobertura histórica; debe re-ejecutarse con binario real |

## 3. Non-goals

- No publicar release (`bash scripts/release.sh` queda operator-side).
- No modificar Authority/Storage/security boundaries (esos son C3).
- No introducir Capability negotiation protocol nuevo: solo respetar el que `cognicode serve` ya anuncia.
- No tocar JCode ni Chronos (C2b/c paralelos).
- No declarar `STATIC_ENHANCED` certificado: el scope es **ejecutar T08–T11**, no el perfil entero.

## 4. Surface area

- `crates/sddk-engine/src/code_intelligence_port_mcp.rs` — `spawn()` y posiblemente `AdapterConfig`/constructor
- `crates/sddk-cli/src/verify_kernel_cmd.rs` — mensajes de error + provider_build string
- `crates/sddk-engine/tests/a6_*.rs` + `aiw_s1_cognicode_real.rs` — regresión
- Doc: `crates/sddk-engine/src/code_intelligence_port_mcp.rs` cabecera (el módulo declara "cognicode-mcp" en su doc-comment)

## 5. Test plan (incremental, scoped al SUT)

Per AGENTS.md §2.3 + Anexo §2 (testing incremental):

| Fase | Acción | Por qué |
|---|---|---|
| T0 | `cargo build -p sddk-engine -p sddk-cli` | compilación focal |
| T1 | `cargo test -p sddk-engine --lib code_intelligence_port_mcp` | unitarios del adapter |
| T1b | Re-ejecutar `a6_cognicode_protocol_spike` y `aiw_s1_cognicode_real` con `--features required` si los hay, o sin | regresión histórica |
| T2 | E2E manual: `sddk verify-kernel --domain static_provider --provider-bin /home/rubentxu/.cargo/bin/cognicode --cwd <repo-root>` y capturar salida | T08 real |
| T3 | E2E con timeout / kill / restart del proceso | T10 real |
| T4 | Repetir comando y comprobar digest estable del manifest de evidencia | T11 |

**No** se ejecuta `cargo test --workspace` aquí — eso es scope C4. La cobertura local del adapter + tests regresivos basta para el cierre del WorkItem.

## 6. STOP conditions

- Si el binario `cognicode serve` no responde al handshake `initialize` JSON-RPC con `protocolVersion: 2025-03-26` → STOP, emitir `BLOCKED` con evidencia cruda (stdout/stderr), no parchear el adapter para mentir.
- Si los tests históricos `a6_*` requieren `cognicode-mcp` como binario literal y bloquean re-ejecución → STOP, decisión de mantenerlos con skip condicional (`#[ignore]` con motivo) o refactorizarlos a `cognicode serve` — pero esto último es alcance nuevo que requiere escalada.
- Si `cargo build` falla en crates no tocadas → STOP y reportar regresión, no parchear fuera de scope.

## 7. Deliverables

1. Commit funcional mínimo (cambio en `spawn()` + cli mensaje + doc comment) bajo `feat(c2a): wire cognicode serve subcommand into adapter`.
2. Ejecución observable de T08, T09, T10, T11 con stdout/stderr capturados.
3. `docs/roadmap/receipts/c2a/<sha>/UAT-EVIDENCE.yaml` con campos del contrato de [CERTIFICATIONS.md §5](../../CERTIFICATIONS.md).
4. `docs/roadmap/receipts/c2a/<sha>/C2a-RECEIPT.md` con status real (PASS_OBSERVED / BLOCKED / FAIL — no PASS_BY_CODE_READING).
5. Entrada de SESSION-JOURNAL.md con SHA antes/después.

## 8. Risks

- **R1**: Adapter cambia de `binary` a `binary serve` rompe consumidores que pasaban ya el binario completo. Mitigación: autodetección — si `binary` termina en `cognicode` o el basename contiene `cognicode` y no termina en `-mcp`, añadir subcomando `serve`. Si termina en `cognicode-mcp`, tratar como legacy (sin subcomando). Documentar en el commit.
- **R2**: Tests `a6_cognicode_protocol_spike` están pinned a v0.97.1 (cita Addendum 18) y el binario real es v0.97.3; pueden fallar por suposición de versión. Si pasa, documentar drift. Si falla, decisión de qué versión ancla la verdad.
- **R3**: `cognicode serve` puede tener flags distintos (`--cwd` puede no existir). Mitigación: ejecutar `cognicode serve --help` y ajustar args según output real antes de tocar el adapter.

## 9. Out-of-scope for this WorkItem

- CogniCode capabilities negotiation specifics (acepta el handshake y registra lo que el server anuncia).
- Detección de capabilities reales; basta con que `initialize` no falle y `serverInfo.version` se registre (lo que ya hace el adapter).
- Tests nuevos de performance o stress (pertenecen a C3 / T26).
- Recertificar `STATIC_ENHANCED` — eso es C4.

## 10. Acceptance

Este WorkItem se considera cerrado cuando:

- [ ] Compila sin warnings nuevos (`cargo clippy -p sddk-engine -p sddk-cli --all-targets -- -D warnings`).
- [ ] Tests del adapter + tests regresivos relevantes pasan en verde.
- [ ] T08, T09, T10, T11 cada uno con un resultado OBSERVED en `UAT-EVIDENCE.yaml`.
- [ ] Sin código nuevo que introduzca Authority/Storage side-effects.
- [ ] Sin releases ni bumps de versión (`v1.169.142` se mantiene hasta que el operador decida publicar).

Si algún criterio falla → estado `BLOCKED` honesto + acción de recuperación, no PASS.
