# ADR-0021 — Phase 1 Hexagonal Architecture Enforcement

**Status:** accepted
**Date:** 2026-08-19
**Supersedes:** ADR-0015 (partial)
**Trigger:** Roadmap SDDK 2.0 Phase 1 block

---

## Context

Phase 1 exit criteria require that `sddk-engine` does not depend on `sddk-storage` directly
in production code, enforcing the hexagonal port/boundary separation. The dependency audit
(2026-08-19) revealed:

```
sddk-engine/Cargo.toml:
  [dependencies]
  sddk-storage = { path = "../sddk-storage" }  ← PRODUCTION dependency

Production code (src/):
  uses: impl Ledger (trait from sddk-domain)  ✓ correct
  uses: &dyn Ledger                           ✓ correct
  NO direct Storage:: open or sddk_storage types  ✓ clean

Test code (adoption.rs:751,770):
  uses: sddk_storage::Storage::open directly  ← only in tests
```

`sddk-engine` production code is already hexagonally correct — it only uses the `Ledger`
trait from `sddk-domain`. The production dependency declaration is wrong; it belongs in
`[dev-dependencies]`.

---

## Decision

1. **Move `sddk-storage` to `[dev-dependencies]`** in `sddk-engine/Cargo.toml`.

   Production code has no direct imports of `sddk_storage`. The concrete `Storage` type
   is only needed in test code (`adoption.rs`). After this change:

   ```toml
   [dev-dependencies]
   sddk-storage = { path = "../sddk-storage" }
   ```

2. **Deprecate `Storage::open` in favor of a `LedgerFactory` port** in `sddk-domain`.

   To make the production dependency on the concrete `Storage` type explicit, add a
   `LedgerFactory` trait to `sddk-domain` that `SqliteLedgerAdapter` (or similar) implements:

   ```rust
   // sddk-domain/src/ports.rs
   pub trait LedgerFactory {
       type Ledger: Ledger;
       fn open_ledger(path: &Path) -> Result<Self::Ledger, StorageError>;
   }
   ```

   The CLI (which links everything) wires `SqliteLedgerAdapter::open_ledger` to satisfy
   `Engine<L>` where `L: Ledger`. This makes the composition root explicit and the
   dependency direction enforceable via `cargo tree --edges=dev`.

3. **Architecture lint: `ARCH003` — No production crate MAY depend on `sddk-storage` unless it
   also provides a `LedgerFactory` implementation.**

   This rule is checked by `archcheck` (ratcheted to fail).

---

## Consequences

- **Positive:** `cargo tree -p sddk-engine -e normal` shows zero `sddk-storage` entries.
  Hexagonal boundary is machine-enforceable.
- **Positive:** Test code in `adoption.rs` continues to work unchanged (dev-dependency).
- **Positive:** `sddk-cli` (the composition root) remains unchanged — it provides `Storage`
  which implements `Ledger` to `Engine<Storage>`.
- **Negative:** `sddk-engine` dev-dependencies now include `tempfile` (already present)
  and `sddk-storage`. Compile time for `cargo test -p sddk-engine` increases marginally.
- **Neutral:** Requires `cargo check -p sddk-engine -e normal` in CI to detect regressions.

---

## Implementation Plan (P1-FIX-001)

| Step | Description | File | Issue | Status |
|------|-------------|------|-------|--------|
| 1 | Move `sddk-storage` to `[dev-dependencies]` | `crates/sddk-engine/Cargo.toml` | P1-FIX-001 | ✅ DONE (37da426) |
| 2 | Add `LedgerFactory` trait to `sddk-domain/src/ports.rs` | `ports.rs` | P1-FIX-002 | ✅ DONE (050f4e0) |
| 3 | Implement `LedgerFactory for SqliteLedgerFactory` in `sddk-storage/src/lib.rs` | `lib.rs` | P1-FIX-003 | ✅ DONE (050f4e0) |
| 4 | Update `sddk-cli` composition root to use `LedgerFactory` (optional, additive) | `sddk-cli/src/lib.rs` | P1-FIX-004 | OPEN |
| 5 | Add `ARCH003` lint to `archcheck` | `tools/archcheck/` | P1-FIX-005 | OPEN |
| 6 | Verify: `cargo tree -p sddk-engine -e normal` shows no `sddk-storage` | CI gate | P1-FIX-006 | ✅ DONE |

---

## References

- Phase 1 exit criteria: ROADMAP.md §Phase 1
- ADR-0015 (composition root waiver — partial)
- `sddk-engine/src/lib.rs` line 793: `pub struct Engine<L: Ledger>`
- `adoption.rs:751,770`: only direct `sddk_storage::Storage::open` usages
