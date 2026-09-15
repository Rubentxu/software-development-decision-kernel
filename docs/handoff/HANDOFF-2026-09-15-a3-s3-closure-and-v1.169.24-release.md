# Handoff — A3-S3 closure + v1.169.24 release

> Status: cycle `p-63676b11dc0ef88f/a3-3-architecture-graph-overlay` **CLOSED**
> Release: **v1.169.24** published to GitHub Releases and installed locally
> Author of closure: orchestrator (2026-09-15)

## Result headline

AC2 (Architecture as SemanticGraphOverlay) is **shipped**. The
`ArchitectureGraphOverlay` is the canonical typed projection of
`ArchitecturalContract` + `ArchitectureClaim` + `SoftwareUnit` plus 14
overlay relations into the single shared `SemanticGraphProjection`
defined by ADR-0098.

## Cycle ledger

| # | Phase | Transition | Result |
|---|-------|------------|--------|
| 1 | explore | (pre-cycle) | `exploration-report.md` |
| 2 | specify | (cycle.start) | `arch-spec-A3-S3-*.md` (22 REQs) |
| 3 | apply | (impl) | `crates/sddk-engine/src/architecture_graph/` (1484 LoC, 5 files) |
| 4 | apply | phase.build.complete | `implementation-receipt.md` |
| 5 | verify | phase.verify.complete.a-min | `verification-report.md` (4 gates green) |
| 6 | release | release.complete | v1.169.24 published + installed |
| 7 | archive | archive.complete | `archive-manifest.json` (CLOSED) |

## Module added (AC2)

```
crates/sddk-engine/src/architecture_graph/
├── mod.rs         (45)  — entry; state-class doc; re-exports
├── types.rs       (425) — SoftwareUnit, UnitKind (7), DecisionRef/SpecRef/TestRef/UatRef/CompatibilityPathRef,
│                            ArchitectureOverlayNodeKind (8), ArchitectureOverlayRelationKind (14),
│                            ArchitectureClaimId, contract_overlay_node_id, convert_claim_evidence_to_universal
├── overlay.rs     (466) — ArchitectureGraphOverlay (delegates to InMemorySemanticGraph);
│                            add_unit, add_claim, add_relation, add_contract_metadata,
│                            find_units_contracted_by, find_contracts_for_unit,
│                            attach_claim_to_unit (outcome-dispatched),
│                            traverse_finding_to_software, traverse_decision_to_software,
│                            Query ADT (3 closed variants), query()
├── rebuild.rs     (67)  — RebuildInputs + rebuild (sorted iteration, REQ-AC2-006)
└── tests.rs       (481) — 16 acceptance + 4 anti-encroachment + 3 bonus + 5 types sub-tests
```

Total: 1484 LoC across 5 files (no file > 510 lines per the split policy).

## Companion substrate changes

- `crates/sddk-engine/src/lib.rs` — added `pub mod architecture_graph;` between `architectural_contract` and `authority`
- `crates/sddk-engine/src/architectural_contract/mod.rs` — `pub(crate) mod claim;`
- `crates/sddk-engine/src/architectural_contract/claim.rs` — `ClaimOutcome::canonical_tag` → `pub(crate)`; added `pub(crate) mod test_helpers` with `claim_with_outcome` constructor
- `crates/sddk-engine/src/architectural_contract/types.rs` — `DecisionRef::canonical_payload` and `SpecRef::canonical_payload` → `pub(crate)`
- `crates/sddk-cli/src/dev/arch_lint.rs` — `C4_LEGACY_ALLOWLIST_M1` line-shift (1167→1168, 1294→1295, 1354→1355; now the **3rd** instance of INC-A3-S1-C4-LINE-SHIFT)

## Anti-encroachment (compile-time pins, REQ-AC2-019..022)

- `anti_encroachment_no_a4_or_provider_imports` ✓
- `anti_encroachment_no_capability_or_authority_side_effects` ✓
- `anti_encroachment_no_markdown_parsing` ✓
- `anti_encroachment_no_second_digest_surface` ✓
- `ArchitectureOverlayNodeKind::ALL.len() == 8` ✓
- `ArchitectureOverlayRelationKind::ALL.len() == 14` ✓

## ADRs / specs

- **ADR-0113** `Architecture as a SemanticGraph Overlay` → **proposed → accepted** (accepted_at: 2026-09-15, accepted_by_cycle: p-63676b11dc0ef88f/a3-3-architecture-graph-overlay, 6 implementation_evidence lines)
- ADR-0113 mirrored to `~/.sddk-knowledge/sddk-framework/adrs/ADR-0113-*.md` (mirrored by `python3 scripts/mirror_adrs_to_vault.py`)
- Spec committed: `docs/architecture/specs/arch-spec-A3-S3-architecture-semantic-graph-overlay.md` (cycle-bounded, 22 REQs, 16 tests planned)

## Test outcomes

- `cargo test -p sddk-engine --lib architecture_graph::` → **23 passed, 0 failed**
- `cargo test -p sddk-engine --lib` (ac2 + adjacent surfaces) → **76 passed, 0 failed, 1 ignored**
- `cargo test --workspace` → **194 test result blocks green, 0 failed**
- `cargo fmt --check` → clean
- `cargo clippy -p sddk-engine --lib --tests -- -D warnings` → clean
- `cargo build --workspace --tests` → clean
- `bash tests/test_vault_adr_mirror_coverage.sh` → OK
- `cargo run --bin sddk -- dev check-architecture` → no new violations

## Carry-over debt (unchanged)

- `INC-A3-S1-C4-LINE-SHIFT` (severity P3) — now incremented to **3rd instance** (allowlist line numbers are brittle to `pub mod X;` insertions in `lib.rs`). The single canonical `arch_lint.rs` allowlist shifted from 1167/1294/1354 to 1168/1295/1355 to accommodate `pub mod architecture_graph;`. **Follow-up proposal:** replace the literal-line allowlist with an AST-based visitor that locates the line where `pub mod X;` is declared and re-derives the three source-of-truth sites from there.
- `surface.briefness.*` test fixtures still missing (pre-A3-S2, out of scope)
- `binary.bundle_coherence: missing` reported by `sddk dev doctor` post-install (release.sh step 11 reports present) — also pre-existing

## Git chain (in order pushed)

```
cca85dd docs(spec): cycle-bounded spec for A3-S3 (AC2 Architecture SemanticGraph Overlay)
54aaf33 feat(engine): A3-S3 architecture graph overlay (AC2)
66fe3a0 chore(uat): A3-S3 ADR-0113 acceptance + arch_lint line-shift
cc89901 chore(deps): refresh Cargo.lock (v1.169.23 workspace pins)
267013d chore(release): bump version 1.169.23 -> 1.169.24
8bc5cba chore(release): bump version marker for v1.169.24 (A3-S3 cycle closure)
```

`HEAD == origin/main == 8bc5cba` after `git push origin main`.

## Release v1.169.24

- GitHub Release: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.24
- Asset: `sddk-v1.169.24-sddk-linux-x86_64-musl.tar.gz`, `CHECKSUMS`, `sddk.sha256`, `sbom.json`
- Local install: `/home/rubentxu/.local/bin/sddk` reports version `1.169.24`
- Framework bundle: `/home/rubentxu/.local/share/sddk/framework/1.169.24/` plus symlink `current` updated
- Pruned: `1.169.23` (kept only 1.X.Y)

## Next steps for the operator

This is the planned **end of the A3-S3 / AC2 sub-cycle**. The next roadmap task after AC2 (per `docs/sddk-decision-kernel-architecture/03-adrs/ADR-042-M3-DRIVER-ROADMAP.md`) is **AC3: cross-engine coherence scenarios** (`find_findings_via_decision`, `find_units_in_finding_chain`, intent-aligned behavioral coherence lenses). To start it, repeat the explore→spec→tasks→apply→verify→release→archive loop with cycle name `a3-4-cross-engine-coherence` and base `8bc5cba`.

If you want to stop here, the local state is fully consistent:
- `sddk 1.169.24` on `PATH`
- framework at `current -> 1.169.24`
- ledger closed on 12 sequence-numbered events for this cycle
- ADR-0113 mirrored into the vault
- all 22 REQs traceable to the 23 module tests
