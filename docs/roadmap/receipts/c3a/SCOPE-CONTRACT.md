# C3a — SCOPE-CONTRACT: Authority hardening (T19, T20)

**Cycle:** C3a (Resilience/Authority adversarial)
**Baseline:** `main@ec86423` (workspace v1.169.142)
**Opened:** 2026-09-22T08:09:00Z
**Owner:** orchestrator (delegates to `sddk-apply` for test-first implementation)
**Authority basis:** AGENTS.md §3; ROADMAP.md §C3 + §3; AGENTS Anexo §2 (T1–T3 incremental)

## 1. Objective (falsable)

Demonstrate that the Authority admission ticket bus enforces the documented invariants under adversarial sequencing:

- **T19**: Deny + policy swap between issue and consume → ticket issued at policy_digest `D1`, policy swapped to digest `D2`, ticket consumption returns `Err(PolicyChanged { ticket_digest: D1, current_digest: D2 })`. No side effects post-swap.
- **T20**: Two CLI instances emitting tickets with different policy_digests concurrently → ticket from instance A with digest `DA` is **not** accepted by instance B whose current state has digest `DB`. No cross-policy acceptance, no global ordering assumption.

Both with real test execution, observed PASS or observed FAIL — never PASS_BY_CODE_READING.

## 2. Findings pre-investigación (OBSERVED, this session)

| # | Hallazgo | Archivo / línea | Implicación |
|---|---|---|---|
| F1 | `consume()` ya implementa `PolicyChanged` check at line 293-298 | `crates/sddk-engine/src/authority_admission_ticket.rs:293` | T19 base contract coded |
| F2 | 8 unit tests in module: T1 happy, T2 PolicyChanged (error variant), T3 FenceExpired (via forged ticket), T4–T8 others | `authority_admission_ticket.rs:444-642` | T19 base covered; **side-effect-on-reject NOT covered** |
| F3 | `t2_policy_changed_between_issue_and_consume_refuses_with_policy_changed` (line 467) asserts the error but **does NOT assert** that `consumed.insert` did NOT happen — the bus could be inserting on the error path silently | `authority_admission_ticket.rs:486-498` | T19 side-effects gap |
| F4 | No test exercises two-process concurrent issuance with divergent policy_digest | `grep -rn "two.*cli\|concurrent.*issuance\|cross.*policy" crates/` empty | T20 is unimplemented |
| F5 | No test exercises concurrent double-consume on the same ticket | `grep -rn "concurrent.*consume\|arc.*barrier" crates/sddk-engine/` empty | T20 atomicity gap |
| F6 | `AdmissionTicketBus::consume` line 287-291 checks `consumed.contains` BEFORE any state change. line 293-298 `PolicyChanged` returns `Err` BEFORE line 317 `consumed.insert`. So the order is safe — but **the test must verify it**. | `authority_admission_ticket.rs:285-319` | Test is verification, not patch |
| F7 | `AdmissionTicketBus` is `Clone` with `Arc<Mutex<FenceState>>` interior; perfect for multi-thread test | `authority_admission_ticket.rs:188-191` | T20 multi-thread feasible |
| F8 | `AuthorityNow` is public with `pub` fields, no constructor required | `authority_admission_ticket.rs:172-176` | test construction easy |

## 3. Non-goals

- No tocar Authority engine más allá de los símbolos necesarios para hacer los tests pasar.
- No tocar Authority policy evaluation (esos son C3 o C1 histórico).
- No abrir sub-crate ni reorganizar Authority en módulos nuevos.
- No tocar Authority docs fuera de los doc-comments del módulo.
- No bump de versión (v1.169.142 estable hasta operator release).

## 4. Surface area

- `crates/sddk-engine/src/authority_admission_ticket.rs` — añadir tests en `mod tests` (T19 ya en scope, T20 nuevo)
- `crates/sddk-engine/tests/` — opcionalmente añadir integration test si T20 no cabe en unit tests
- Sin cambios al módulo `authority_engine/`
- Sin cambios a `process_service`

## 5. Test plan (TDD-first incremental)

Per AGENTS.md Anexo §2 (T1 unit tests focalizados, no `cargo test --workspace` aquí):

| Fase | Acción | Por qué |
|---|---|---|
| T0 | `cargo fmt --all -- --check && cargo clippy -p sddk-engine --all-targets -- -D warnings` | baseline sin cambios |
| T1 RED | Añadir `#[test] fn t19_policy_swap_records_no_side_effects()`. Pattern: build bus, issue ticket with policy A, mutate `now.current_policy_digest` to B, consume with old ticket (expect `Err(PolicyChanged)`); then construct `now` back to A with `current_seq = 11`, attempt consume again with same ticket, and **expect `Ok(())`** (proves the rejected consume did NOT poison the bus's consumed-set). | T19 zero-side-effects (closes gap F3) |
| T1 RED | Añadir `#[test] fn t20_two_buses_with_divergent_policy_digests_dont_cross_accept()`. Two `AdmissionTicketBus` instances, each cloned. Bus-A bound to policy A digest, Bus-B bound to policy B digest. Issue from each in parallel using `std::thread::spawn` + `Arc<Barrier>`. Cross-attempt consume on the other bus's ticket. Assert `Err(PolicyChanged)` from both sides — never `Ok(())`. | T20 cross-policy |
| T1 RED | Añadir `#[test] fn t20_concurrent_double_consume_only_one_succeeds()`. Two `std::thread` aligned by `Arc<Barrier>`, both calling `bus.consume(same_ticket, &now)`. Assert: exactly one `Ok(())`, exactly one `Err(TicketAlreadyConsumed)`. Use `JoinHandle::join` for both threads, count outcomes. | T20 atomicity |
| T1 GREEN | Run `cargo test -p sddk-engine --lib authority_admission_ticket` until all 3 new tests pass. | T1 verification |
| T2 | `cargo test -p sddk-engine --lib` para asegurar no hay regresión en otros módulos del crate | regression local |
| T2b | `cargo test -p sddk-engine --test a6_0_admission_tickets` | integration regression |

**No** se ejecuta `cargo test --workspace` — eso es C4. La batería del módulo + integración cercana es suficiente para T19/T20.

## 6. STOP conditions

- Si `AdmissionTicketBus` no permite construir dos instancias con policy digests distintos en paralelo (e.g. algún global estático) → STOP, escalar como gap de diseño, no parchear con `lazy_static` o equivalente.
- Si los 4 tests no se pueden escribir sin cambiar la API pública → STOP, registrar como ADR-pending y NO implementar.
- Si `cargo clippy -p sddk-engine --all-targets -- -D warnings` introduce warnings en código no tocado → STOP, reportar regresión upstream.
- Si al añadir los tests T20 el bus demuestra race conditions reales (no solo ausencia de test) → emitir `BLOCKED` con diagnóstico, no parchear el bus para que "pase".

## 7. Deliverables

1. Commit funcional bajo `feat(c3a): T19+T20 — policy-swap no-side-effects + cross-policy and atomic concurrency tests`.
2. 3 tests nuevos (1 para T19 gap, 2 para T20), todos verdes.
3. `docs/roadmap/receipts/c3a/<sha>/UAT-EVIDENCE.yaml` con campos del contrato.
4. `docs/roadmap/receipts/c3a/<sha>/C3a-RECEIPT.md` con status PASS_OBSERVED o BLOCKED.
5. Entrada de SESSION-JOURNAL.md.

## 8. Risks

- **R1**: Los tests T20 (concurrencia) pueden ser flaky en CI por timing. Mitigación: usar `std::sync::Barrier` o `std::sync::Arc<Barrier>` para sincronizar; si aún flaky en local, registrar como honesto y NO usar `sleep` ni timing magic.
- **R2**: La cobertura existente podría revelar que `consume` con policy_changed SÍ tiene side effects no intencionados (e.g. el `consumed.insert` ocurre ANTES del check). Mitigación: verificar el orden de checks en `consume()` antes de escribir el test; si está mal, el test RED lo demostrará, y entonces el fix es legítimo (no bypass).
- **R3**: `AuthorityNow` puede no exponer API para cambiar `current_policy_digest` sin reissue. Verificar; si no, construir un constructor manual o helper de test.

## 9. Out-of-scope for this WorkItem

- Performance benchmarks (pertenecen a C3d).
- Storage adversarial (C3b, separado).
- Security canarios (C3c, separado).
- Schema resilience (C3e, separado).
- Cualquier cambio en Authority engine bridge o runner.

## 10. Acceptance

Este WorkItem se considera cerrado cuando:

- [ ] 3 tests nuevos pasan en verde bajo `cargo test -p sddk-engine --lib authority_admission_ticket`.
- [ ] Sin clippy warnings nuevos.
- [ ] Sin cambios a APIs públicas.
- [ ] UAT-EVIDENCE y RECEIPT commiteados con status real (PASS_OBSERVED / BLOCKED).
- [ ] SESSION-JOURNAL.md tiene entrada con SHA antes/después.

Si algún criterio falla → status `BLOCKED` con acción de recuperación; no PASS.
