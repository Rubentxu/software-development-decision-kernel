# Agent-asset baseline inventory

> **Snapshot:** 2026-09-09 (adoption cycle `p-63676b11dc0ef88f/architecture-adoption-m0-supersession`).
> Authoritative agent-facing asset baseline for M0 inventory and M7 migration.
> No behavior changes; counts only.

## Asset counts (current)

| Surface | Count | Path |
|---|---|---|
| Agent profiles | 70 | `agents/` |
| Skill files | 241 | `skills/` |
| Prompt files (sddk-only) | 43 | `prompts/sddk/` |
| Templates | 0 | `templates/` |
| Rules | 0 | `rules/` |
| Embedded CLI examples in `docs/` | TBD (M0 D5 sweep) | `docs/**/*.md` |
| Embedded prompt strings in Rust | TBD (M0 D5 sweep) | `crates/sddk-cli/src/`, `crates/sddk-engine/src/` |

The baseline in `propose-manifest.md` reported 69/204/41; the current count is
70/241/43 (drift between propose and apply). The discrepancy is recorded so
that M0 D5 (full inventory + smell detection) can start from a verified
baseline.

## Migration disposition classes

Per package `AGENT-ASSET-MIGRATION.md`:

- **KEEP** — asset is canonical, no change required.
- **MIGRATE** — asset must move to typed YAML/JSON contract.
- **SPLIT** — asset bundles multiple concerns; split into KEEP / MIGRATE / ABSORB.
- **ABSORB** — asset content is folded into a higher-level contract.
- **DEPRECATE** — asset is no longer needed.
- **REMOVE** — asset is removed after traceability is preserved (M9 only).

The full per-file classification belongs to M0 D5. This baseline only freezes
the asset surface.

## Smells to detect (M0 D5, deferred)

- Hard-coded deprecated CLI command names.
- Direct mention of internal SQLite/table/store paths.
- Vault described as authority or memory.
- Raw ledger reads prescribed to ordinary agents.
- Cycle runtime states used after Run consolidation.
- Instructions that grant/assume permission (Skill ≠ Capability violation).
- Duplicated architecture conventions across multiple skills.
- Free-form output where Contribution/ExecutionOutcome schema exists.
- Prompt text implementing workflow transitions.
- Undocumented provider-specific behavior.
- Command examples not present in CommandRegistry.

Detection results and per-asset disposition are deferred to M0 D5.

## Author

Adoption cycle: `p-63676b11dc0ef88f/architecture-adoption-m0-supersession`.
Next cycle: `p-63676b11dc0ef88f-m0-inventory-baseline` (D1..D7).