# C2a-MsgFix — SCOPE-CONTRACT: improve --provider-bin error messages + doc comments

**Cycle:** C2a-MsgFix (UX honesty, no functional change)
**Baseline:** `main@383d112` (workspace v1.169.149)
**Opened:** 2026-09-22T08:56:00Z
**Owner:** orchestrator (direct execution, scope < 50 lines)
**Authority basis:** AGENTS.md §3 (gates preauthorized for ordinary continuity); C2a-RECEIPT §5 documented this gap; no new authority required.

## 1. Objective (falsable)

Replace the misleading literal `cognicode-mcp path` / `chronos-mcp path` mentions in
`crates/sddk-cli/src/verify_kernel_cmd.rs` user-facing strings with messages that accurately
describe what binary the operator should pass, **without changing the underlying adapter
contract or evidence URI scheme**.

## 2. Findings (OBSERVED, session-11)

| # | String | File:Line | Issue |
|---|---|---|---|
| F1 | `"verify: --domain static_provider requires --provider-bin <cognicode-mcp path>"` | verify_kernel_cmd.rs:172 | Hardcodes `cognicode-mcp` even on hosts without that binary (only `cognicode` CLI ships). Misleads operator. |
| F2 | `"verify: --domain runtime_provider requires --provider-bin <chronos-mcp path>"` | verify_kernel_cmd.rs:327 | Same shape; `chronos-mcp` does not ship. |
| F3 | `/// Static-provider mode: path to the real `cognicode-mcp` binary.` | verify_kernel_cmd.rs:64 | Doc comment shown in `--help`; hardcodes the binary name. |
| F4 | `provider_build: "cognicode-mcp/verify-cmd"` | verify_kernel_cmd.rs:204 | **Schema URI / evidence contract — DO NOT TOUCH** |
| F5 | `"cognicode-mcp://find_usages/{symbol}#usages={}"` | verify_kernel_cmd.rs:240, 265 | **Evidence URI — DO NOT TOUCH** |
| F6 | `"cognicode-mcp"` (origin tag) | verify_kernel_cmd.rs:252, 274 | **Evidence origin tag — DO NOT TOUCH** |
| F7 | Adapter `provider_build` (real binary version) | code_intelligence_port_mcp.rs:290 | **Contract — DO NOT TOUCH** |

**Decision**: only touch F1, F2, F3. Leave F4-F7 (evidence contract strings) untouched —
changing them is a breaking schema change requiring ADR.

## 3. Non-goals

- No release cut.
- No change to Authority/Storage boundaries.
- No change to evidence URI scheme or provider_build strings.
- No change to adapter behavior.
- No new test (this is a UX-honesty fix; existing tests + a new doc test are sufficient).

## 4. Surface area

- `crates/sddk-cli/src/verify_kernel_cmd.rs:64` (doc comment)
- `crates/sddk-cli/src/verify_kernel_cmd.rs:172` (error message)
- `crates/sddk-cli/src/verify_kernel_cmd.rs:327` (error message)

## 5. Test plan (scoped)

- T1: `cargo build -p sddk-cli` (compile after edit).
- T2: `cargo clippy -p sddk-cli --all-targets -- -D warnings` (lint clean).
- T3: `cargo test -p sddk-cli --lib` (existing tests still pass).
- T4: `cargo run -p sddk-cli -- verify --domain static_provider --subject foo` → expect new error message containing the literal "MCP server" instead of "cognicode-mcp".
- T5: same for `--domain runtime_provider` → "MCP server" instead of "chronos-mcp".
- T6: `cargo run -p sddk-cli -- verify --help` → doc comment should describe the binary without hardcoding a name.

## 6. STOP conditions

- If the new message accidentally contains the string "cognicode-mcp" or "chronos-mcp" (evidence contract strings must remain unchanged in the surrounding code) → STOP, revert.
- If any existing test fails → STOP, report regression.

## 7. Deliverables

1. Commit `fix(cli): use generic 'MCP server' in --provider-bin error messages and doc comments` (single concern).
2. `docs/roadmap/receipts/c2a-msgfix/{UAT-EVIDENCE.yaml,RECEIPT.md}` documenting the change.
3. SESSION-JOURNAL.md entry with SHA before/after.
4. CURRENT.md update if applicable.

## 8. Risks

- **R1**: An operator may already grep for the string `cognicode-mcp` in CLI output as a stable contract. Mitigated: the strings appear ONLY in the new doc comment / error message, which was previously a misleading one-shot, not a stable API.
- **R2**: Some downstream tooling may parse the old message. Unlikely (the old message was an error path, not a stable JSON schema).

## 9. Out-of-scope

- Changing evidence URIs (F4-F7) — would require ADR for schema change.
- Changing the adapter to point at a different protocol (TCP, etc.) — separate SCOPE.
- Removing `cognicode-mcp` mentions from doc comments **inside** the adapter itself — out of scope; the adapter legitimately targets `cognicode-mcp` per IPB-001..012.

## 10. Acceptance

- [ ] `cargo build -p sddk-cli` → clean.
- [ ] `cargo clippy -p sddk-cli --all-targets -- -D warnings` → clean.
- [ ] `cargo test -p sddk-cli --lib` → existing tests pass.
- [ ] T4/T5/T6 commands return strings that do NOT contain `cognicode-mcp` / `chronos-mcp`.
- [ ] Evidence URI strings at F4-F7 unchanged.
- [ ] No release / version bump (workspace 1.169.149 stays; this is a post-release housekeeping fix that can land on main without a new tag).
