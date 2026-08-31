# Architecture Decision Records

Short, immutable decisions that are hard to reverse. Each ADR is a timestamped record of a consequential design choice.

## Index

| # | Title | Status | Date |
|---|-------|--------|------|
| ADR-0001 | E2E Validation Sandbox | accepted | 2026-08-13 |
| ADR-0002 | Atomic Gate Receipt Seq Allocation | accepted | 2026-08-13 |
| ADR-0014 | Phase 1 ARCH Evaluators | accepted | 2026-08-14 |
| ADR-0015 | ARCH003 Composition Root Waiver | accepted | 2026-08-17 |
| ADR-0016 | Universal Evidence Model | accepted | 2026-08-17 |
| ADR-0017 | Tier-Based Model Resolution | accepted | 2026-08-17 |
| ADR-0018 | User Owns IDE Config | accepted | 2026-08-17 |
| ADR-0019 | Editor Adapter Trait | accepted | 2026-08-17 |
| ADR-0020 | Bash + Gum + TUI Shell | accepted | 2026-08-17 |
| ADR-0021 | Phase 1 Hexagonal Enforcement | accepted | 2026-08-17 |
| ADR-0022 | SDDK Testkit | accepted | 2026-08-17 |
| ADR-0023 | Event Export JSONL | accepted | 2026-08-17 |
| ADR-0024 | Evidence Redaction Rules | accepted | 2026-08-19 |
| ADR-0040 | BTreeMap Mandate for IR Collections | accepted | 2026-08-19 |
| ADR-0041 | Schema Version as `u32` Constant | accepted | 2026-08-19 |
| ADR-0042 | ARCH008 SDD-Agnostic Kernel Runtime + WV-0026 | accepted | 2026-08-19 |
| ADR-0043 | Compiler Determinista sin LLM | accepted | 2026-08-19 |
| ADR-0044 | Validator con 7 Gates en Short-Circuit | accepted | 2026-08-19 |
| ADR-0045 | GraphStore Port con 6 Métodos IR-Revision | accepted | 2026-08-19 |
| ADR-0050 | True-Concurrent Parallel with Pure-Return Contract | accepted | 2026-08-24 |
| ADR-0051 | `Box<dyn Operator>` → `Arc<dyn Operator>` Runtime Safety | accepted | 2026-08-24 |
| ADR-0052 | Concurrent Parallel Channel Design (cycle-20) | implemented | 2026-08-24 |
| ADR-0053 | Tick/Receiver-Map Design (cycle-20) | implemented | 2026-08-24 |
| ADR-0054 | OperatorContext field types: Arc<Mutex<T>> | accepted | 2026-08-24 |
| ADR-0060 | Evidence Contracts for the SDDK Prompt Layer | accepted | 2026-08-24 |
| ADR-0061 | Operator::Map stub scope (cycle-26, Phase 4 WU-1) | accepted (stub) | 2026-08-24 |
| ADR-0065 | Map Source Context Isolation Cross-Tick Replay | accepted | 2026-08-26 |
| ADR-0067 | Runtime Checkpoint Draining | accepted | 2026-08-26 |
| ADR-0068 | Bounded Execution | accepted | 2026-08-24 |
| ADR-0069 | Test-Tooling Ownership (Rust/Shell/Python/JS) | accepted (user-approved 2026-08-28) | 2026-08-28 |

## Adding an ADR

1. Create `ADR-XXXX-title-slug.md` in this directory
2. Use the template: `docs/sddk-2.0-architecture-consolidation/templates/ADR-TEMPLATE.md`
3. Add the entry to this index table in chronological position
4. Commit with `docs(adr): ADR-XXXX — short title`

## Conventions

- File name: `ADR-NNNN-slug.md`
- Status values: `proposed | accepted | deprecated | superseded`
- No ADR is deleted — deprecated/superseded ADRs are kept with updated status
- ADRs are immutable once accepted — new decisions get new ADRs
