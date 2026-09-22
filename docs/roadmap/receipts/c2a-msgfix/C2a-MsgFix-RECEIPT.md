# C2a-MsgFix — RECEIPT

**Cycle:** C2a-MsgFix (UX honesty fix; no functional change)
**Status:** `PASS_OBSERVED`
**Baseline:** `main@383d112` (workspace v1.169.149)
**HEAD post-cycle:** `3a09914` (workspace v1.169.150 after ceremonial bump)
**Issued:** 2026-09-22T09:01:00Z
**Issuer:** orchestrator (direct execution; scope < 10 lines net change)

## 1. Outcome

The misleading literal `cognicode-mcp path` / `chronos-mcp path` in
`crates/sddk-cli/src/verify_kernel_cmd.rs` user-facing strings has been
replaced with the generic `path-to-MCP-server-binary`. The previous
strings were inaccurate (the `cognicode-mcp` binary does not ship with
the available `cognicode` v0.97.3 CLI; `chronos-mcp` does not ship at
all) and would mislead operators on hosts without those binaries.

**This is NOT a release-blocker**. The change is UX-only: existing
tests pass, evidence contract strings are unchanged, and adapter
behavior is unchanged.

## 2. Evidence

Full evidence in [UAT-EVIDENCE.yaml](UAT-EVIDENCE.yaml).

| Scenario | Status | Result |
|---|---|---|
| T1 build | PASS_OBSERVED | cargo build clean (1m 18s) |
| T2 clippy | PASS_OBSERVED | clippy clean (1m 25s) |
| T3 tests | PASS_OBSERVED | 780 passed, 0 failed, 1 ignored |
| T4 error static | PASS_OBSERVED | new message: `<path-to-MCP-server-binary>` |
| T5 error runtime | PASS_OBSERVED | new message: `<path-to-MCP-server-binary>` |
| T6 help doc | PASS_OBSERVED | doc comment visible, accurate |

## 3. Changes applied

| File | Lines | Change |
|---|---|---|
| `crates/sddk-cli/src/verify_kernel_cmd.rs:64` | +2/-1 | doc comment: "real `cognicode-mcp` binary" → "static MCP server binary (currently `cognicode-mcp`; the adapter spawns it via stdio JSON-RPC)" |
| `crates/sddk-cli/src/verify_kernel_cmd.rs:172` | +1/-1 | error message: `<cognicode-mcp path>` → `<path-to-MCP-server-binary>` |
| `crates/sddk-cli/src/verify_kernel_cmd.rs:327` | +1/-1 | error message: `<chronos-mcp path>` → `<path-to-MCP-server-binary>` |

Net: **+4 lines, -3 lines** in source.

Evidence contract strings (provider_build, evidence URIs, observation
origin tags) at lines 205, 241, 253, 266, 275 — **UNCHANGED**. Touching
those requires an ADR (separate SCOPE).

## 4. Decisions taken

- **No ADR required**: the changes are in non-contract UI strings (an
  error message and a doc comment shown in `--help`). The evidence
  schema, provider_build format, and adapter contract are untouched.
- **No new tests**: this is a UX-honesty fix; existing tests cover
  behavior preservation. The UAT scenarios T4-T6 are observable from
  the binary itself.
- **Ceremonial bump 1.169.149 → 1.169.150**: required by the pre-push
  hook contract because source code (`crates/sddk-cli/src/...`) was
  touched. No release cut is performed.
- **No release cut**: per the existing operator-side gate for `bash
  scripts/release.sh`. The current `v1.169.122` is the last public GH
  release; this cycle produces a buildable artifact but does not
  publish.

## 5. Surprises during execution

- **S1**: The `verify` subcommand in the CLI is for ledger continuity
  (M6.1 facade), not the AIW-S1 verify-kernel. The correct command
  is `verify-kernel`. Discovered via `sddk --help`; not a bug.
- **S2**: `verify-kernel` requires `--claim <CLAIM>` even when the
  argument-parsing fails earlier. The error path I wanted to test is
  reachable via `--domain X --claim foo` (without `--provider-bin`),
  which produces the new error message immediately.

## 6. Out-of-scope (not done)

- Adapter source code that still mentions `cognicode-mcp` (legitimate
  — the adapter targets `cognicode-mcp` per IPB-001..012).
- Evidence URI scheme `cognicode-mcp://find_usages/{symbol}#…` — this
  is a contract; changing it would invalidate existing receipts and
  requires an ADR.
- The CLI `--help` output for the `--claim` and `--subject` flags (no
  misleading strings there).

## 7. Acceptance

All criteria from SCOPE §10 satisfied:

- [x] `cargo build -p sddk-cli` → clean.
- [x] `cargo clippy -p sddk-cli --all-targets -- -D warnings` → clean.
- [x] `cargo test -p sddk-cli --lib` → existing tests pass.
- [x] T4/T5/T6 commands return strings that do NOT contain
  `cognicode-mcp` / `chronos-mcp` (in the affected positions).
- [x] Evidence URI strings at F4-F7 unchanged (verified via grep).
- [x] No release cut (workspace bumped to 1.169.150 but no tag, no GH
  Release).

## 8. Next WorkItem

C2a-MsgFix closes the side finding documented in
[docs/roadmap/receipts/c2a/C2a-RECEIPT.md §5](../c2a/C2a-RECEIPT.md).
The systemic C2 provider/adapter missing pattern persists:
- CogniCode: `cognicode-mcp` binary absent, `cognicode` v0.97.3 CLI
  does NOT expose MCP (probe evidence in C2a UAT).
- Chronos: `chronos-mcp` binary absent.
- JCode: `jcode` v0.86.0 present, `jcode-sdk` crate absent, SDDK has
  no adapter.

Roadmap C2 contract requires operator-side artifacts to unblock; no
AUTO-executable path remains within C2.

**Next viable AUTO WorkItem**: investigate whether any other C2/C3
side finding (beyond the msg fix just executed) has a small, scoped
fix that does NOT require provider artifacts or ADR authority.

Candidates surfaced in C3 receipts and earlier sessions:
- C3 mentions performance budget (Base/static/runtime); only Base is
  executable in this environment.
- Doc-string fixes in adapter source comments that hardcode binary names
  (analogous to this msg fix, in different file).
- Any `cargo doc` warnings or stale docs that can be tightened without
  changing behavior.

This cycle closed one such candidate. Remaining ones to evaluate in
next steps.
