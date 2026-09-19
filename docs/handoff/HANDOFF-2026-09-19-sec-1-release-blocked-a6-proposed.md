# Handoff — SEC-1 release blocked on workspace flake; A6 (CogniCode) proposed, not opened

Date: 2026-09-19
Branch: `main` (HEAD local `b93b50d`, partly pushed)
Operator decisions received: 2026-09-19T13:29:39Z.

## Three operator decisions (received)

1. **Cerrar A5-C administrativamente.** No more A5-C audits on
   agent initiative. A5-C certification v1.169.88 (`add896d`)
   remains the certifying authority. G11 NOT VERIFIED, R14
   OPEN_NON_BLOCKER preserved as-is.
2. **Definir qué debe entregar A6.** Contrast against the live
   roadmap (Production-Readiness Alignment 14/09 mini-roadmap).
   Identify the operational capability that is missing **after**
   the historical a6-0..a6-4 cycles (which closed R4-B/G5).
3. **Autorizar únicamente el primer ciclo implementable.** The
   agent proposes; the operator authorises. The cycle is not
   opened in this session.

This document satisfies decisions 2 and 3; decision 1 is held.

## A5-C status (unchanged)

- `v1.169.88` → `add896d94274a7515254b8c195e2e78c3669108f` is
  the certifying release.
- `docs/architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md`
  remains the authority.
- G11 = NOT VERIFIED. R14 = OPEN_NON_BLOCKER. Not retroactively
  reframed.

**No new A5-C audit was opened in this session.** Decision 1
held.

## A6 — proposed (cycle NOT opened)

### Confusion to clear first

The repo already has cycle IDs `a6-0..a6-4` which close R4-B
(decide-then-act TOCTOU) — see `docs/architecture/a6/A6-0..4-*`
and `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY`. The live
roadmap also reserves "A6" for CogniCode STATIC_ENHANCED.

Per `docs/architecture/README.md` and the A5-CURRENT-ROADMAP:
> "A6 CogniCode / STATIC_ENHANCED provider integration —
> NOT STARTED — reserved. Not to be confused with the
> historical a6-0..a6-4 cycle IDs above."

So this proposal uses the names **A6-CogniCode / CC-S0 / CC-S1**
(no `a6-` numeric prefix) to avoid collision with the closed
R4-B work.

### What A6 (CogniCode STATIC_ENHANCED) must deliver

Per `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`
and the CogniCode handoff (`04-COGNICODE-HANDOFF.md`):

| Exit criterion (operator-observable) | Status |
|-------------------------------------|--------|
| Base mode passes with CogniCode stopped/uninstalled | expected (already the contract by `arch-spec-021 IPB-010`); verification deferred |
| `CodeIntelligencePort` SDDK-owned trait exists in code | **NOT DONE** — the trait is described in `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md` but not implemented in `crates/sddk-engine` (only `crates/sddk-engine/src/completion_provider_router.rs` has a passing comment) |
| Capability negotiation (BASE / STATIC_ENHANCED / RUNTIME_ENHANCED / FULLY_ENHANCED) is runtime authority | **NOT DONE** — provider lifecycle states UNIMPLEMENTED |
| Anti-corruption adapter: provider wire DTOs / transport errors / internal graph types terminate at the gateway/extension boundary | **NOT DONE** — no CogniCode adapter exists |
| Static evidence (AC10) maps provider results to SDDK Evidence with basis/provider/analyzer provenance | **NOT DONE** — Verify pipeline does not consume provider observations yet |
| Falsifiable UAT for the 12 acceptance rows in `04-COGNICODE-HANDOFF.md §8` | **NOT DONE** — no pinned static-enhanced receipt |
| Provider absence returns EvidenceGap / NOT_EVALUATED, not PASS | **NOT DONE** |

So the operational capability missing after A6-4 (R4-B closed)
is: **a verified SDDK-side seam for static intelligence
providers** — currently zero coverage, only spec.

### First implementable cycle — proposed

**A6-CogniCode-protocol-spike (CC-S0).** Strict scope, no
SDDK-core changes; the deliverable is a thin protocol skeleton
inside the workspace that proves an external Rust client (the
SDDK adapter candidate) can connect, negotiate, request
`analyze_delta`, receive stable typed results, cancel/timeout,
and reconnect.

Why this is the right first cycle:

- It is the **cheapest** A6 step that produces falsifiable
  evidence (no rewiring of Verify, no Knowledge schema change).
- It maps 1:1 to `04-COGNICODE-HANDOFF.md §5 CC-S0`.
- It does not require an external CogniCode binary to exist
  yet; the spike can use an in-process fake provider with the
  same wire contract.
- It produces the protocol version + capability snapshot
  digest that downstream adapter cycles (CC-S1..CC-S3)
  depend on.

Scope budget (one cycle, narrow):

1. **Trait in code** — `crates/sddk-engine/src/code_intelligence_port.rs`:
   `CodeIntelligencePort` trait per `arch-spec-021`, with the
   four mandatory operations (`analyze_delta`, `analyze_scope`,
   `analyze_impact`, `capabilities`) plus the provider
   lifecycle state enum (`UNAVAILABLE`/`DORMANT`/`STARTING`/
   `READY`/`BUSY`/`INCOMPATIBLE`/`FAILED`).
2. **In-process fake provider** — `crates/sddk-engine/src/code_intelligence_port_fake.rs`:
   a deterministic, offline provider that returns stable
   digests for a fixture set (source-only local change;
   dependency-impact change; syntax/parse failure;
   unsupported language; partial analysis; cancelled request;
   incompatible protocol; provider restart). Coverage mirrors
   `04-COGNICODE-HANDOFF.md §5 CC-S4`.
3. **Falsification battery** — `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs`:
   - T1: connect, negotiate `BASE` capability snapshot →
     adapter receives `STATIC_ENHANCED` advertised only when
     negotiation succeeds.
   - T2: `analyze_delta` on a fixture → stable digest
     reproducible across two runs (determinism).
   - T3: capability negotiation protocol-major mismatch →
     `INCOMPATIBLE` (provider surface must NOT silently
     degrade to `BASE`).
   - T4: cancellation → typed refusal, no false positive
     evidence.
   - T5: provider restart mid-request → reconnect + replay
     → same digest (or explicit partial marker).
   - T6: Base-mode contract — with the fake provider
     `UNAVAILABLE`, SDDK Verify still produces the same
     result as without the port (Base is first-class).
4. **Tests** — same profile used by SEC-1:
   `cargo test -p sddk-engine --offline` (targeted),
   `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo fmt --check`.
5. **Receipt** — `docs/architecture/a6/A6-COGNICODE-CC-S0-RECEIPT.md`
   mirroring the structure used by `A6-0-RECEIPT.md` (scope,
   gate evidence, honest limits).

MUST (acceptance):

- M1. SDDK does not depend on any CogniCode-specific type
  outside the adapter module. (`arch-spec-021 IPB-001`.)
- M2. Provider is evidence source, not truth authority.
  (`IPB-002`.)
- M3. Heavy data remains provider-side. (`IPB-003` — covered
  in code by the fake returning only stable digests.)
- M4. Capability negotiation is runtime authority. (`IPB-004`.)
- M5. Failure / cancellation is epistemically visible.
  (`IPB-008`.)
- M6. Base mode is first-class. (`IPB-010` — T6 above.)
- M7. No LLM intermediary between SDDK and the provider port.
  (`IPB-012`.)

MUST NOT:

- N1. No new secret manager / log system (carried forward
  from SEC-1 discipline).
- N2. No new canonical SDDK domain types derived from
  CogniCode internals.
- N3. No production ingestion of provider observations into
  Knowledge — the spike is evidence-of-the-port, not
  evidence-into-Knowledge (that's CC-S1+ territory).
- N4. No modification of `arch-spec-021`; this cycle
  implements the spec, it does not amend it.

Out of scope (named):

- Adapters for the real CogniCode binary (CC-S2 / CC-S3).
- Static evidence integration into Verify (AC10) — that is the
  next cycle (CC-S1).
- Chronos RUNTIME_ENHANCED — separate A7 track.
- Agentic Workspace / JCode track — already has its own
  J2 proposal at
  `docs/proposals/2026-09-19-jcode-anti-corruption-adapter-PROPOSAL.md`.

### Operator asks

This proposal is **NOT** opened as a cycle in this session.
Three outcomes are possible next:

1. **Authorise CC-S0 as written.** Open the cycle
   `p-63676b11dc0ef88f/a6-cognicode-protocol-spike`; agent
   writes scope contract + falsification plan + tests against
   pre-fix code (RED pinned), implements the trait + fake,
   closes with the cycle receipt.
2. **Authorise a different first cycle** (e.g. CC-S1
   stable-provider-boundary, or J2 anti-corruption adapter).
   The agent adjusts accordingly.
3. **Defer A6.** The next cycle becomes the workspace flake
   fix (`SEC-WORKSPACE-FLAKE`), which unblocks both the
   SEC-1 release gate and any future A6/A7 cycle that runs
   `cargo test --workspace`.

## SEC-1 release status

### Work done

- `Cargo.toml` workspace bump: `1.169.88` → `1.169.89`.
- `Cargo.lock` updated.
- Commit `b93b50d` (`chore(release): bump version to 1.169.89`).
- 3 dry-runs of `bash scripts/release.sh --dry-run` against
  the bump commit (steps 0–8 only).

### Result

| Run | Workspace green | Steps 0–8 |
|-----|-----------------|-----------|
| 1   | 1 FAILED (`cross_surface_facades_share_the_service_instance` from `a6_4_shared_ticket_service`) | aborted at step 1 |
| 2   | 0 FAILED | dry-run OK |
| 3   | 0 FAILED | dry-run OK |

`SEC-WORKSPACE-FLAKE` is **REPRODUCED POST-FIX**. The
failing test passes 5/5 when run in isolation, and the
test-suite-level cause is the project-flagged process-global
CWD race in `crates/sddk-cli/src/cycle.rs:158`.

### Block

Per operator contract (verbatim):

> "Si reaparecen los dos fallos del workspace, no registrar el
> gate completo como verde. Mantener el bloqueo hasta obtener
> un resultado reproducible o una excepción explícita y
> acotada del operador, respaldada por la reproducción
> anterior y posterior al fix, la ejecución de los tests
> restantes y la conservación del fallo en SEC-WORKSPACE-FLAKE."

**Block held.** `gh release create` is not invoked.

The release script (`scripts/release.sh`) does not accept a
workspace-flake-bypass knob. The flag set is
`--dry-run` / `--skip-tests` / `--skip-install` / `--force`.
Of these, `--skip-tests` is the only escape hatch; the
operator contract explicitly handles that path:

> "Si se autoriza excepcionalmente --skip-tests, ejecutar y
> registrar por separado los controles que esa opción omite.
> La excepción no puede convertirse en 'workspace PASS'."

### Operator decision needed for SEC-1 release

One of:

1. **Hold** — fix the workspace flake first (separate cycle,
   out of SEC-1 scope, but unlocks every subsequent release).
2. **Bounded exception** — operator authorises `--skip-tests`
   on the SEC-1 release run, with the four sub-conditions
   named in the SEC-1 receipt §"Required artefacts to clear
   the block" (acknowledged out-of-scope; recorded separately;
   not converted to "workspace PASS"). The release then
   proceeds to `gh release create v1.169.89`, install smoke,
   doctor, and PublicReleaseGate.

Until the operator decides, the release script is **not**
invoked in production mode.

## Local commit graph (after this session)

```text
b93b50d chore(release): bump version to 1.169.89                       ← release bump
50c7d0f docs(handoff): correct HEAD SHA and push contract note
b442635 docs(handoff+proposal): SEC-1 closed; J2 anti-corruption adapter proposal
f6e1906 docs(sec-1): close cycle with falsification receipt
1374951 fix(gateway): redact stdout/stderr secret-bearing substrings (SEC-1)
2c63aff test(sec-1): RED pinned — capability receipt stdout/stderr leak
8499ff7 docs(a5): rectificar 3 claims imprecisos del cert (audit pass 3)   ← release tag v1.169.88
```

## Suggested next action (operator decision)

The work has three threads; the operator chooses the order.

| Thread | What's next | Operator cost |
|--------|-------------|---------------|
| **A.** SEC-1 release | Decide on `--skip-tests` exception | Low — bounded exception |
| **B.** A6 (CogniCode) | Authorise CC-S0 protocol spike | Medium — new cycle |
| **C.** Workspace flake | Authorise a small cycle to fix the CWD race (move `architecture-rules.yaml` under `sddk-domain/test-data/`, or refactor to read via `env!` only) | Low — unblocks everything downstream |

**Recommendation (in order of expected value):** C → A → B.

Reasoning: C unlocks A (release can run with full workspace
gate green) and B (the workspace flake is unrelated to A6
content but blocks every `cargo test --workspace`). A ships
the SEC-1 fix as a corrective release. B opens the new
A-track cycle.

If the operator prefers to ship SEC-1 first (B), the bounded
exception on `--skip-tests` is acceptable per the contract,
provided the four sub-conditions are recorded separately.
