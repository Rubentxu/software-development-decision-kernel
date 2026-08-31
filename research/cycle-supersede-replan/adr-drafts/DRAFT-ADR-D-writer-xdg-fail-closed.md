# DRAFT-ADR-D — WriterXdgFailClosed contract

> **Status**: DRAFT (not accepted). Awaiting cycle-50+ adoption.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-50
> **Amends**: none (additive)
> **Supersedes**: none
> **Authority target**: `crates/sddk-cli/src/vault_cmd.rs` + cross-cutting

---

## Pre-decision: context, decisions, alternatives, compatibility, authority, migration

### Context

The XDG fail-closed discipline is **partially** enforced today:

- `crates/sddk-vault/src/repair.rs` (line 172): atomic temp + rename for
  RepairReceipt queue.
- `crates/sddk-vault/src/validate.rs`: `normalize_cycle_target` rejects
  `..`, `/`, leading/trailing/double hyphens.
- `crates/sddk-cli/src/cycle.rs:90-94`: `resolve_xdg_paths` resolves via
  `environment.xdg()` (which enforces `XDG_DATA_HOME must be absolute`).
- `crates/sddk-cli/src/knowledge_cmd.rs:300`: writes
  `xdg_profile_path` (correctly XDG-resolved).
- `crates/sddk-cli/src/uat.rs:1677`: writes UAT storage via
  `uat_storage_root` (correctly XDG-resolved).

**Gap**: `crates/sddk-cli/src/vault_cmd.rs:443` (`vault export`) takes a
user-supplied `--output <path>` and writes there with `std::fs::write`
without validating that the path is not inside the workspace root. An
agent could pass `--output ./v2/vault/foo.html` and pollute the repo.

The principles are scattered across 5 sites. There is no reusable
contract; each writer re-implements its own guard.

### Decision (proposed)

Introduce a `WriterXdgFailClosed` trait in a new module
`crates/sddk-cli/src/writer.rs` (or co-located in
`crates/sddk-engine/src/paths.rs`):

```rust
pub trait WriterXdgFailClosed {
    /// Resolves the absolute XDG path for the requested artifact.
    /// Returns Err iff the resolved path is not under XDG_DATA_HOME
    /// or XDG_STATE_HOME or XDG_CONFIG_HOME.
    fn resolve_xdg_path(
        &self,
        kind: ArtifactKind,
        sub_path: &RelativePath,
    ) -> Result<PathBuf, XdgWriterError>;

    /// Writes atomically (temp + rename).
    fn write_atomic(
        &self,
        path: &Path,
        contents: &[u8],
    ) -> Result<(), XdgWriterError>;
}
```

Plus a **validation helper** that rejects workspace-relative paths for
user-supplied `--output` flags:

```rust
pub fn validate_xdg_output(
    output: &Path,
    workspace_root: &Path,
) -> Result<(), XdgWriterError>;
```

Apply this validator at `vault_cmd.rs:443` (vault export) and any other
site that accepts user-supplied output paths.

### Alternatives considered

| # | Alternative | Why rejected |
|---|---|---|
| D1 | Validate each `--output` flag at CLI parse time | Some flags legitimately target the workspace (e.g., `--in-repo` for dogfooding); a CLI-only validator cannot distinguish |
| D2 | Add a global pre-write hook | Hidden behavior; agents cannot reason about it |
| D3 | Centralize all writes via a single writer singleton | Too invasive; the goal is documentation + targeted validation, not centralization |
| D4 | Use `dirs` crate only (no XDG enforcement) | `dirs` already resolves XDG; but does not enforce "not under workspace root" |

### Compatibility with current ledger

- **Trait is documentation/refactor aid, not a new requirement** for
  existing XDG-resolved sites (knowledge, UAT, cycle artifacts).
- **`vault export --output <path>` gains a new validation**: `--output`
  MUST NOT be inside the workspace root. An alternative `--xdg-output
  <kind>` flag resolves to XDG (no input path).
- **No existing command's signature changes** (additive flag).
- **No ledger digest change** (writers are CLI concern, not ledger).
- **VAULT003 / RepairReceipt (v1.65.6) are unchanged**: the trait wraps
  `append_receipt`'s atomic-write behavior without modifying its logic.

### Authority limits

- The trait does not enforce schema or content; it enforces **where**
  writes go, not **what** is written.
- The trait does not bypass capability policy (`check_vault_capability`
  is still called first per `vault_cmd.rs:120-140`).
- **Symlink escape is detected**: canonicalize the resolved path; reject
  if canonicalization differs from input (defense against symlink-based
  bypass).

### Migration path

1. **Phase 1 (this research)**: ADR-D drafted; blueprint drafted.
2. **Phase 2 (cycle-50 candidate, A-min)**: implement trait, validate
   `vault_cmd.rs:443` site. RED test first.
3. **Phase 3 (cycle-51+)**: extend to UAT wizard temp HTML writes.
4. **Phase 4 (cycle-52+)**: refactor existing XDG-resolved sites to use
   the trait (no behavior change; documentation gain).

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| XDG env not set | low | medium | Already handled by `CliEnvironment::xdg()` fallback to `HOME/.local/share` |
| Symlink escape | low | high | Canonicalize; reject if canonicalization differs |
| User-injected `--output` | medium | medium | Validate at CLI parse + at write site (defense in depth) |
| Breaking change for `vault export --output` users | medium | medium | New flag `--xdg-output` is preferred; `--output` still works if outside workspace |

---

## Post-decision: formal sections (for adoption)

### Status

(pending acceptance)

### Date

(pending acceptance)

### Consequences

(pending)

### Implementation notes

- New module: `crates/sddk-cli/src/writer.rs`.
- New validator at `vault_cmd.rs:443`.
- New flag `--xdg-output` on `vault export`.

### Compatibility / migration

See Phase 1–4 above.

### Revisit trigger

Revisit when:

- A new writer site is added that needs XDG fail-closed.
- The XDG spec is updated (low frequency; last major 2024-08-17).

### Implementation trace

- **cycle-50** (target): implements trait + validates `vault_cmd.rs:443`.
  Refer to
  `research/cycle-supersede-replan/blueprints/writer-xdg-fail-closed.yml`
  and
  `research/cycle-supersede-replan/evidence-cards/ec-css-004-writer-xdg-foundation.yml`.

---

## References

- `crates/sddk-vault/src/repair.rs:172` (atomic write pattern)
- `crates/sddk-vault/src/validate.rs` (normalize_cycle_target)
- `crates/sddk-cli/src/vault_cmd.rs:443` (vault export — the gap)
- `crates/sddk-cli/src/cycle.rs:90-94` (resolve_xdg_paths)
- `crates/sddk-cli/src/dev/paths.rs` (XDG root resolution)
- AGENTS.md §4.1 (no crear nuevos checkouts en `~/.sddk-shared/`)
- XDG Base Directory Specification 2024-08-17 (`XDG_DATA_HOME` MUST be absolute)