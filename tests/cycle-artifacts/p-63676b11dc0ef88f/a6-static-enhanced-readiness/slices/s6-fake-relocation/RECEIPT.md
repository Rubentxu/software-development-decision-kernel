# RECEIPT — S6 — Fake relocation (NOT_PROCEED, audit-only)

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s6-fake-relocation`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Baseline (released):** `v1.169.95` → `9688ebb` (S1 close)
> **Cycle lead:** orchestrator (this session, auto-run)
> **Status:** **CLOSED — `NOT_PROCEED`**, push pending release flow.

## §1 Scope adherence

| Hard constraint | Status | Evidence |
|---|---|---|
| C1: no fabrication | ✅ | Audit reproducible; binary footprint measured empirically. |
| C2: no source modification without operator decision | ✅ | Zero source files modified. |
| C3: no scope drift from the original macro-cycle goal | ✅ | Decision recorded as `NOT_PROCEED` with reason; if API hygiene is the actual goal, an amended SCOPE is required. |
| C4: workspace test green; CC-S0/CC-S1/S1/S2/S3 still pass | ✅ | Untouched: same green state as S3 close (12/4/12/5/7 + 1 ignored). |

| STOP condition (per SCOPE §4) | Triggered? |
|---|---|
| Downstream production usage of the fake | **No** — zero hits outside `tests/`. |
| Move breaks CC-S0/CC-S1 tests | **N/A** — no move attempted. |
| **Material architectural change** requires operator alignment (AGENTS.md §2.7, §2.9) | **Yes** — introducing `test-support` feature changes the build matrix and the public surface. Reported. |

Per the operator's session rule, material architectural changes stop
the line until aligned. S6 closes in `NOT_PROCEED` state with the
audit and the proposed path forward.

## §2 Evidence

### 2.1 Audit summary

- 31 references to `FakeCodeIntelligenceProvider` / `NullCodeIntelligenceProvider` ...
- ... all in `crates/sddk-engine/tests/` (5 files).
- 0 references in `crates/sddk-cli`, `crates/sddk-gateway`, `sddk-domain`, `sddk-storage`.
- Module size: 376 LOC.
- `Cargo.toml` has `default = []`, `std = []` only; no `test-support`.

### 2.2 Empirical release-binary footprint

```
$ cargo build --release -p sddk-engine --offline
   Compiling sddk-engine v1.169.95
    Finished `release` profile [optimized] target(s) in 53.94s

$ strings target/release/libsddk_engine*.rlib | grep -c "FakeCodeIntelligenceProvider"
0

$ strings target/release/libsddk_engine*.rlib | grep -c "NullCodeIntelligenceProvider"
0
```

Both types are **stripped from the release rlib**. The macro-cycle's
stated rationale for the slice ("fake lives in release builds") is
already empirically moot: rustc dead-code elimination removes the
symbols because no consumer outside the crate references them.

### 2.3 What the relocation would still buy

The remaining valid motivation is **API hygiene** (making the fake
non-public), which is **scope drift** from the original S6 goal. This
change qualifies as a public-contract change and a material
architectural change; it requires operator alignment.

### 2.4 Alternative paths (for operator decision)

If API hygiene is the actual goal, three ordered options:

1. **`pub(crate)`**: zero build-matrix impact, no feature flag, no
   `cfg` gates. Only internal-to-crate consumers; the fake still
   compiles but is not re-exported. Lowest cost.
2. **`#[cfg(any(test, feature = "test-support"))]` without moving
   to `dev-dependencies`**: keeps the fake as a regular source
   module but gates its compilation. Feature flag added. Medium
   cost (changes `cargo doc` / IDE experience).
3. **Full relocation to `dev-dependencies` + `test-support` feature**
   (original SCOPE): pulls the file out of the source tree, makes
   it usable only by integration tests via a small adapter or a
   `tests/common/mod.rs`. Highest cost; only worth it if a
   downstream crate will legitimately use it in production.

## §3 Files changed by this slice

| Path | Δ | Role |
|---|---|---|
| `tests/cycle-artifacts/.../slices/s6-fake-relocation/SCOPE-CONTRACT.md` | nuevo | Audit + decision record. |
| `tests/cycle-artifacts/.../slices/s6-fake-relocation/RECEIPT.md` | nuevo | This file. |

**Zero source files modified. Zero tests added. Zero features added.**

## §4 Honest limits

1. **The empirical binary-footprint check is rlib-level.** It does
   not cover the case where `sddk-engine` is consumed as a
   `cdylib` or `staticlib` (neither is configured). For rlib and
   binary consumers, the dead-code elimination result is the same:
   no fake symbols reach the final artifact.
2. **The audit looked for `FakeCodeIntelligenceProvider` /
   `NullCodeIntelligenceProvider` by name.** A consumer that imports
   `sddk_engine::code_intelligence_port_fake::*` (module glob) was
   not separately grep'd; the test files all use explicit names, so
   this is not a real gap, but if a glob import exists it would be
   counted under the module path, not the type path.
3. **`cargo doc` was not run.** If `code_intelligence_port_fake` is
   visible in `cargo doc` output, that is a separate motivation
   for API-hygiene gating. This slice did not exercise that surface.

## §5 Next slices

- **S4 — durability** (`PR-UAT-024`). STOP anticipated; see S4 prep.
- **S7 — closeout integrated report.** Can proceed once S4 is
  resolved (NOT_PROCEED or accepted) and S5 closes (which it has,
  in `NOT_EVALUATED` state).
- **S6 itself** is **NOT_PROCEED** pending operator decision. If
  the operator wants it re-opened, the choice of path is in §2.4
  above and should be re-scoped as a new slice.
