# Receipt — SDDK Configuration Model v1 (single resolver) + jcode integration

> Cycle: `p-63676b11dc0ef88f/config-model-v1` (jcode control-plane strand; **not** an
> A5 workstream — the A5 programme is untouched and A5-2 has not been opened).
> Contract of record: `arch-spec-049-sddk-configuration-model-v1` (status: `implemented`),
> ADR-0129.

## Identity (recorded separately)

| Field | Value |
|---|---|
| project_id | `p-63676b11dc0ef88f` |
| workspace_id | `w-2e7853aadc28217a6649e309` |
| released tag | `v1.169.73` |
| release SHA | `947984b880fd19b32cc550b33240c2e4626cdc5f` |
| workspace version at release | `1.169.73` |
| binary sha256 (prefix) | `f8fcd949e65c67da…` |
| bundle `manifest_sha256` | `608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f` |
| local framework bundle | `~/.local/share/sddk/framework/1.169.73` (`current` → `1.169.73`) |
| GitHub Release | `v1.169.73`, `isDraft=false`, `isPrerelease=false`, 9 assets |
| post-release main head | `6604117` (docs-only: receipt + A5 README pointer + INC registration) |

## §0 Falsification first (RED → GREEN)

| # | Defect | RED (observed) | GREEN (observed) |
|---|---|---|---|
| 1 | `prompts/sddk/orchestrator.md` edited without regenerating the committed manifest | `cli_dev_install_accepts_committed_manifest` → `manifest verification FAILED (1 mismatch(es)): prompts/sddk/orchestrator.md: hash mismatch` | `sddk dev manifest --verify` → `manifest OK`; step 1 → `✓ workspace green` |
| 2 | `sddk-mode` swallowed the resolver's failure cause (`2>/dev/null`) | reset the exact original logic in a scratch dir: `stderr_bytes=0` with `MODE=undeclared REASON=resolve-failed` | patched shim: `stderr_bytes=118`, `sddk-mode: config resolve failed: perfil no encontrado: ghost` |
| 3 | new code violated the engine's `#![deny(clippy::all)]` | `cargo clippy -p sddk-engine --lib` → `redundant field names in struct initialization` ×2; `-p sddk-cli` → `this if statement can be collapsed` ×2 | `cargo clippy --workspace --all-targets -- -D warnings` → clean |

Defect 1 and 2 are process/observability defects found **by running the full
pipeline**, not by reading the code. Both are now pinned (defect 1 in
`AGENTS.md` §5 checklist, defect 2 in `sddk-config-selftest` L8 case).

## §1 What was built

**Engine** — `crates/sddk-engine/src/orchestration_config.rs` (new):

- model keys with types/defaults across `autonomy.*`, `human_feed.*`,
  `workflow.*`, `parallelism.*`, `verification.*`, `personality.*`
- non-overridable laws: `git.{push,tag,release,history_rewrite}=human_gate`,
  `evidence.{unverified_green,invented}=forbidden`, `evidence.classes=mandatory`,
  `adoption.*=undeclared`; forbidden keys list
- adoption index v1 (`<id> <mode> [profile]`) with the closed reason vocabulary
  `index-absent` / `no-entry` / `declared:<scope>` / `invalid-value:<scope>` /
  `duplicate:<scope>`
- profile chain resolution (`extends`), cycle detection, missing-profile detection
- `upsert_declaration` (replaces, never duplicates — L5) and `remove_declaration`
  (returns the level to inheritance or `undeclared`)

**CLI** — `crates/sddk-cli/src/config_cmd.rs`: `sddk config resolve|laws|profiles|set|clear`.
`resolve` prints `mode/reason/profile/project/workspace` plus every key with its
SOURCE (`system-law`, `profile:<name>`, `builtin`); `--format json` emits the same
as `EffectiveOrchestratorConfig`. `sddk_data_root()` + `resolve_project_ids()`
moved to the crate root so resolver and writer share one identity source.

**Prompts / skills** — `prompts/sddk/orchestrator.md` gained a
"Configuration (single resolver)" section: the law that consumers MUST NOT
independently resolve SDDK configuration, the three prompt layers (core laws /
effective config / workflow context), the profile-governed key table, and
subagent scoping (workers do not receive onboarding, global personality, or
unrelated git policy).

**ADR / spec** — ADR-0129-SINGLE-CONFIG-RESOLVER (accepted, mirrored to the
vault); arch-spec-049 `implemented_by` / `adrs` updated.

**jcode-side (out-of-repo, `~/`)** — `~/.jcode/prompt-overlay.md` and
`~/AGENTS.md`: one vocabulary (`undeclared`, never `no-declarado`), the
single-resolver law, the three layers. `~/.jcode/bin/sddk-mode` and
`~/.jcode/bin/sddk-config` are pure delegation shims.

## §2 Evidence

| Claim | Evidence | Class |
|---|---|---|
| Resolver + writer behave as specified | `cargo test -p sddk-engine --lib orchestration_config` → 17 passed | OBSERVED |
| CLI surface renders value + source | `cargo test -p sddk-cli --lib config_cmd` → 11 passed (5 new) | OBSERVED |
| No workspace regression | `cargo test --workspace` → **4719 passed, 0 failed, 14 ignored** (A4 baseline: 4697) | OBSERVED |
| Lints clean | `cargo clippy --workspace --all-targets -- -D warnings` → clean; shellcheck scope clean | OBSERVED |
| Full release pipeline | 14/14 steps, exit 0 | OBSERVED |
| Public release honest | tag anchored at `947984b` via `git ls-remote`, `isDraft=false`, `isPrerelease=false`, 9-asset contract, 9/9 HTTP 200 | OBSERVED |
| Local install coherent | `sddk dev doctor` → `binary.bundle_coherence: present`, `all_present: true`; prune removed stale; distrib round-trip OK | OBSERVED |
| Shims delegate, do not resolve | `sddk-config-selftest` → **49 ok, 0 fail**; `sddk-mode-selftest` → **14 ok, 0 fail** | OBSERVED |
| spec/ADR consistency | `test_adr_promotion_format.sh` → 37 ADRs, 0 violations; `test_vault_adr_mirror_coverage.sh` → 36 mirrored | OBSERVED |

## §3 Findings discovered by running the FULL 14 steps

1. **`MANIFEST.sha256` is committed state and step 1 verifies it before step 4
   regenerates it.** A cycle that edits bundle assets (`prompts/`, `agents/`,
   `skills/`, `packs/`) and forgets the manifest fails closed at step 1. The
   gate is correct; the checklist was incomplete. **Fixed**: `AGENTS.md` §5 now
   requires `sddk dev manifest --root .` + `git add MANIFEST.sha256` **before**
   the gate.
2. **The push gate has no admissible path for derived-metadata commits.**
   `githooks/pre-push` accepts (A) a real `[workspace.package] version` change or
   (B) a non-empty range entirely under `docs/**` / `.sddk/followups/**`.
   A manifest-only fix (case 1) satisfies neither, so it can only be pushed
   **together with a bump**. Not amended: rewriting pushed history is a
   `git.history_rewrite` human gate.
3. **Consequence — version burn.** `1.169.72` was committed and pushed but never
   released (the release stopped at step 1). The release therefore shipped
   `1.169.73`; `1.169.72` exists only in history. Admission stayed monotonic
   (`ACCEPT 1.169.72 -> 1.169.73`) and nothing shipped under the burned number.
   **Process lesson**: run the full gate **before** creating the bump commit, or
   accept the burn.
4. **`Cargo.lock` was missing from bump `314dbd7`.** Earlier bumps include it.
   Corrected in the amended, still-unpushed `947984b` (`Cargo.lock` + `Cargo.toml`).

## §4 Claim → Evidence for the four orchestration concerns

| Concern | Where it lives now | Not conflated with |
|---|---|---|
| Knowledge | ADR-0129 + arch-spec-049 (model authority), vault mirror | not evidence of behaviour |
| Alignment | the model itself: which axis (adoption mode vs profile) owns which tension | not a verification result |
| Verification | the two selftests + the Rust pins; receipts are what verify/deb-verify leave behind | not a governance rule |
| Governance | laws in the resolver (`git.*=human_gate`, `evidence.*=forbidden`) — enforced, not documented-only | not advisory |

## §5 Deferrals (explicit)

- `sddk config get <key>` does not exist; consumers use `resolve` (piping the
  first matching row). Adding it changes CLI surface and was not needed for this
  cycle.
- A2/A3/A4 coverage of the profile matrix is partial: `cautious`/`manual` are
  listed and loadable but their `autonomy.*` interaction has no dedicated test.
- The overlay is jcode-side, so there is no repo test asserting the overlay and
  `sddk config resolve` agree; the agreement is by construction (overlay text
  names the CLI as the resolver) and by the two selftests.
- A5-2 (durability/rebuild/recovery) is **not started** and must not be
  auto-opened.

## §6 No new production abstraction

No new crate, no new trait indirection, no new persistence. The change is one
engine module (resolver + writer), one CLI command family, one prompt section,
one ADR, one spec update, and two out-of-repo shims.
