# A6-1 Receipt — `framework_bundle` migration to `AdmissionTicketBus`

> Cycle: `p-63676b11dc0ef88f/a6-1-framework-bundle-migration`
> Gates evidence produced:
> - **R4-B** (decide-then-act TOCTOU) — **migration step**. The
>   `framework_bundle` High-band unguarded surface is the first to opt
>   into the `AdmissionTicketBus` wrapper. The wrapper issues a ticket
>   immediately after the existing R4-A `auth.validate(...)` short-circuit
>   and consumes it before any `std::fs` write. If the consume refuses
>   (PolicyChanged / FenceExpired / TicketAlreadyConsumed), the body
>   never runs.
> - **G16** (A4 preserved) — zero A4 semantic change. Wrapper reuses
>   A6-0's primitive.
> Risks touched: **R4-B migration step 1 / 2**.
> Hard constraints honoured: **strangler**, no new authority, no A4
> re-interpretation, no change to `AuthorityEngine` source.

## Identity

| Field | Value |
|---|---|
| `project_id` | `p-63676b11dc0ef88f` |
| `workspace_id` | `w-2e7853aadc28217a6649e309` |
| `cycle HEAD at receipt` | `ffd472e` (A6-0 closing SHA at start) |
| `release HEAD` | post-cycle bump (target v1.169.77) |
| `cycle commits` | `44f82f1` (A6-0 feat), `ffd472e` (A6-0 bump), `9b83046` (A6-0 handoff) were the A6-0 baseline; A6-1 introduces a new feat+bump pair |
| `adrs promoted` | `ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES` (`proposed → accepted` in-cycle) |
| `incidents opened` | 0 |
| `incidents closed` | 0 (INC-R4 was closed at A6-0 as TESTED_BOUNDARY; this cycle is the first migration step toward FULLY_MIGRATED) |
| `findings` | 1 — see §6 |

## §0 Falsification first

| # | Defect | Pre-fix | Post-fix |
|---|---|---|---|
| 1 | `framework_bundle` install path has no R4-B re-validation. A policy flip between `auth.validate(...)` and the actual `atomic_write` could authorise a stale effect. | `crates/sddk-cli/src/dev/install.rs::run_dev_install` ran `auth.validate(WritableSurface::FrameworkBundle)` but went straight into the body without an `AdmissionTicketBus` boundary. | The body is now wrapped in `with_framework_bundle_ticket`. The `AdmissionTicketBus::issue` happens immediately after the R4-A validate; `consume` happens before `body()`. Pinned by `crates/sddk-cli/src/dev/framework_bundle_ticket.rs::tests` (4 tests). |
| 2 | A6-0's T2 (PolicyChanged) coverage was at the engine primitive only. There was no CLI-side witness for the same contract. | No CLI-side test exercised the consume anchor. | New CLI-side unit tests `fence_t1`/`t4`/`t5`/`t6` exercise the wrapper end-to-end against the same primitive. T6 pins policy_digest anchor mismatches at the wrapper. |

## §1 What landed

| Surface | Change | Rationale |
|---|---|---|
| `docs/architecture/adrs/ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES.md` | New ADR; defines the surface-migration pattern. | A6-2 (github_releases) consumes this verbatim. |
| `docs/architecture/a6/A6-1-PLAN.md` | Plan + scope budget + excluded surfaces. | Cycle record. |
| `crates/sddk-cli/src/dev/framework_bundle_ticket.rs` | New module: `with_framework_bundle_ticket`, `framework_bundle_policy`, `FrameworkBundleTicketError`. 4 unit FENCE tests. | The wrapper per ADR-0131. |
| `crates/sddk-cli/src/dev/mod.rs` | `pub(super) mod framework_bundle_ticket;` | Wire-up. |
| `crates/sddk-cli/src/dev/install.rs` | `run_dev_install` body now wrapped in `with_framework_bundle_ticket`. R4-A `auth.validate(...)` preserved. | The actual migration: every `std::fs::write`/`atomic_write`/`copy_tree` is now gated by a ticket consume. |

Total: ~390 lines net new (helper + tests + ADR + plan + receipt). `AuthorityEngine` source unchanged.

## §2 Gate evidence

| Gate | Evidence | Class |
|---|---|---|
| **FENCE matrix T1 (happy path)** | `framework_bundle_ticket::tests::fence_t1` GREEN | OBSERVED |
| **FENCE matrix T4 (one-shot)** | `framework_bundle_ticket::tests::fence_t4` GREEN | OBSERVED |
| **FENCE matrix T5 (Deny ⇒ no ticket)** | `framework_bundle_ticket::tests::fence_t5` GREEN | OBSERVED |
| **FENCE matrix T6 (verdict-anchored consume)** | `framework_bundle_ticket::tests::fence_t6` GREEN | OBSERVED |
| **G16 (A4 preserved)** | `cargo test --workspace` 0 failures; A4 milestone corpus untouched. | OBSERVED |
| **Strangler** | `auth.validate(...)` preserved; `AuthorityEngine::admit` source unchanged. | OBSERVED |
| **`no_new_root_level_context_module_without_adr`** | Passes — `framework_bundle_ticket` is `dev/`, not root level; no ADR registration required. | OBSERVED |
| **full workspace green** | `cargo test --workspace` 0 failures; `cargo clippy --workspace --all-targets -- -D warnings` 0 errors; `cargo fmt --check` 0 diffs. | OBSERVED |

## §3 Findings discovered during the cycle

1. **`sddk_engine::authority::AuthorityContext::for_cli` lives in `sddk_domain::ActorKind` while the wrapper requires `sddk_engine::authority_engine::ActorKind`.** The two enums are similar but distinct; bridging them required a small explicit mapping in `run_dev_install`. **Cluster**: `CL-CROSS-CRATE-TYPE-ALIGNMENT`. **Disposition**: closed in-cycle (mapping in `install.rs` lines 39–50 of the wrapped body).
2. **`AuthorityEngineRunner::admit_surface` does not return the registered `PolicySnapshot`.** This forced A6-1 to construct its own engine inside the wrapper rather than reuse the runner. ADR-0131 names A6-3 as the upgrade cycle for threading the live snapshot. **Cluster**: `CL-MIGRATION-COST`. **Disposition**: deferred to A6-3.

## §4 Honesty markers

- **T2 (PolicyChanged via `policy_digest`) is NOT directly pinned for the `framework_bundle` call site.** The wrapper anchors the ticket to a `framework_bundle_policy()` it constructs, and the consume uses the same anchor. The engine primitive's T2 is pinned inside `crates/sddk-engine/tests/a6_0_admission_tickets.rs`. The CLI helper pins T6 (verdict-anchored consume) instead. A6-3 closes this gap.
- **`AuthorityEngine::admit` source is byte-equal.** Diff inspection: zero changes to `crates/sddk-engine/src/authority_engine.rs`.
- **`AuthorityContext::for_cli` uses `sddk_domain::ActorKind`, not the engine's.** A small explicit adapter closes the gap in `run_dev_install`; A6-3 may improve it by aligning the types directly.
- **`seq = 0` is the A6-1 honest limit** for `issued_at_seq`/`current_seq`. There is no canonical ledger sequence for a `sddk dev install` at this scope; the runtime is single-threaded, so the contract is `Ok(())` today. A6-3 will thread the real `seq`.

## §5 Hard non-goals still honoured

- **No `github_releases` migration in this cycle.** A6-2 owns it; ADR-0131 covers the pattern verbatim.
- **No Medium/Low band migration.** Same as A6-0.
- **No A4 semantic reinterpretation.** Wrapper reuses A6-0 primitive + R4-A auth-validate.
- **No ledger-schema migration.** Same as A6-0.

## §6 Follow-ups deferred

1. **`github_releases` migration** (A6-2) — ADR-0131 pattern, applied to `dev release.rs`.
2. **Medium-band migration** (`plan_item`, `evidence_attachment`, `decision_record`, `dependency_edge`) — separate future cycle.
3. **A6-3 (or next migration):** thread live `PolicySnapshot` from `AuthorityEngineRunner` into wrappers; close the T2-at-call-site gap; align `sddk_domain::ActorKind` with `sddk_engine::authority_engine::ActorKind` (or document the cross-crate bridge).

## §7 INC-R4 update

INC-R4 (`INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md`) is currently closed as
`TESTED_BOUNDARY` (A6-0). The migration step in this cycle advances it
toward `FULLY_MIGRATED`. The INC remains `closed` at TESTED_BOUNDARY
because not all high-band unguarded surfaces have migrated yet (only
`framework_bundle`); the closure criterion from the INC's body
(requirement 3 — "migrate High-band unguarded surfaces first") is
**partially met**: `framework_bundle` migrated; `github_releases` is A6-2.

If a future audit wants to record the step, add an `evidence_links:` entry
in the INC's frontmatter pointing at `docs/architecture/a6/A6-1-RECEIPT.md`.

## Reproduce tomorrow

```bash
cd ~/Proyectos/agentesIA/sddk-framework
cargo test -p sddk-cli --lib framework_bundle_ticket
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
~/.local/bin/sddk --version    # expect 1.169.77 post-release
```
