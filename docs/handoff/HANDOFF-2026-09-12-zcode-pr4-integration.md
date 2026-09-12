# Handoff — PR #4 zcode-native-agents integration

## Status
**Integrated and verified.** Doctor: `all_present: true`. The zcode drift noted
in cycle-7's archive-manifest (`zcode.broken_agent_links: missing`) is closed.

## Origin

PR #4 (`feat/zcode-native-agents`, commit `b7fec35`) merged into main during
the v1.168.37 release flow. The merge happened asynchronously while the
release script was running, which caused a non-fast-forward push that
required manual `git merge --no-edit origin/main` + tag repointing during
cycle-7's release. The merge and its functional impact were noted as a
"side benefit" in the cycle-7 archive-manifest, but no standalone handoff
document was produced.

This handoff consolidates the integration evidence and verifies that the
fix is durable across the v1.168.38 and v1.168.39 releases that followed.

## What PR #4 fixed

### Symptom (pre-fix)
On a real install (`sddk dev install` with editor=zcode), 49 symlinks under
`~/.zcode/agents/` survived the first migration. These symlinks pointed into
the framework root at paths like `debt-overeng-cluster.md`, `jd-judge-b.md`,
`uat-guide.md`, etc. — agents that lack the `sddk-`/`sdd-`/`gentle-` prefix
that the previous ownership check required.

### Root cause
`register_subagent` in `crates/sddk-cli/src/dev/editor_adapters/zcode.rs`
gated the legacy-artifact replacement on `is_sddk_owned(&agent.name)`, which
returned `true` only for names carrying the `sddk-`/`sdd-`/`gentle-` prefix.
The bundle has 70+ agents without that prefix (`debt-*`, `jd-*`, `uat-*`,
`studio-*`, …), so symlinks pointing to those agents were treated as
"user-owned" and skipped during migration. The prune path had the same bug.

### Fix (3 changes)
1. **`register_subagent` (line ~148)**: a symlink under `agents/` is **always**
   ours to replace — users never symlink into the framework's namespace. The
   `is_sddk_owned` check now only guards `name`-less stale **regular** files
   (which is the original intended semantic).

2. **`prune_stale_artifacts` (line ~184)**: introduced `links_into_framework`
   closure that canonicalizes the symlink target and checks whether it
   resolves into `ctx.root` (the framework root). If yes, the symlink is
   ours to prune even if the agent name doesn't carry the prefix. The
   `/home ↔ /var/home` aliasing on this filesystem is handled by
   `std::fs::canonicalize` on both sides of the comparison.

3. **Regression test (`zcode_adapter_tests.rs:register_migrates_agent_map_era_artifacts`)**:
   updated to use a non-prefixed `gentle-bar` symlink as the regression case,
   with an `assert!(!foo.contains("old body"))` after the migration to
   confirm the symlink was replaced with native content.

## Verification (post-merge)

| Check | Command | Result |
|---|---|---|
| Regression test | `cargo test -p sddk-cli zcode::zcode_adapter_tests` | 11 passed (incl. `register_migrates_agent_map_era_artifacts`) |
| Doctor zcode | `sddk dev doctor --prefix /home/rubentxu/.local/bin` | `zcode.broken_agent_links: present` |
| Doctor zcode | `sddk dev doctor --prefix /home/rubentxu/.local/bin` | `zcode.agent_name_frontmatter: present` |
| Doctor overall | `sddk dev doctor --prefix /home/rubentxu/.local/bin` | `all_present: true` |
| Carry-forward to v1.168.38 | After vault-mirror cycle | drift stable, no regression |
| Carry-forward to v1.168.39 | After evidence-mapping cycle | drift stable, no regression |

## Cycle archive cross-references

- **Cycle 7** (`synthesis-dissent-runner-extension`, v1.168.37) — first release to ship PR #4 (merge happened mid-release). Archive-manifest gotchas §2 documents the non-fast-forward push recovery. Archive-manifest §"Doctor drift closeout" already documented that PR #4 + the v1.168.38 install re-bootstrap closed `zcode.broken_agent_links`.
- **Cycle 8** (`vault-mirror-accepted-adrs`, v1.168.38) — side benefit noted: "PR #4 closed the zcode half" of the doctor drift.
- **Cycle 9** (`planning-evidence-migration`, v1.168.39) — no zcode impact.
- **Cycle 10** (`transition-outcome-m9-2-closeout`, v1.168.40 — this cycle) — this handoff consolidates the integration evidence.

## Remaining drift (NOT closed by PR #4)

The doctor reports 19 `surface.briefness.*.md` items as `missing` after
v1.168.39. These are surface-level briefness stubs for skills that exist in
the framework but lack a corresponding `~/.zcode/skills/<name>.md`. The
`all_present` aggregate ignores these because they're treated as best-effort
(default `M*-Surface-Briefness` checks), not as blocking gates. Tracked as
cycle-12 (`surface-briefness-drift`).

## Carry-forward lessons

1. **Release script + concurrent PR merge is fragile** (cycle-7 §2 gotcha). The `scripts/release.sh` step 9 pushes the new tag **before** a possible rebase with concurrent PR merges. The result is a non-fast-forward push that the pre-push hook can reject mid-release. Workaround: `git merge --no-edit origin/main` + `git tag -f v<X.Y.Z> HEAD && git push origin v<X.Y.Z> --force`. Permanent fix is in scope for INC-RELEASE-TAG-FIX (cycle-11).

2. **The `is_sddk_owned` naming was misleading**. It really meant "name starts with sddk- prefix" — not actual ownership. PR #4's `ours = meta.is_symlink() || is_sddk_owned(&agent.name)` makes the test explicit: a symlink is ours by structural proof, a regular file is ours by name-prefix proof. The split is now in the code comment too.

3. **Canonical path comparison needs canonicalize on both sides**. The `/home ↔ /var/home` aliasing on this system (`/var/mnt/DiscoChino2-fast/Proyectos/agentesIA/sddk-framework` vs `/home/rubentxu/Proyectos/agentesIA/sddk-framework` per the SDDK CLI's path resolution) means `path.starts_with(&root_canon)` must use canonicalized forms on both sides, otherwise the symlink-resolves-into-framework check fails spuriously.
