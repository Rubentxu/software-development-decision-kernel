# C0-PRE-RELEASE-SMOKE — Anexo al C0-RECEIPT (1.169.130)

> **Slice id:** `p-63676b11dc0ef88f/c0-reconciliation-baseline`
> **Working SHA:** `6fda463b481cd0c40d65f86ea0d61ed05d28e51c` (post-bump 1.169.130)
> **Date (UTC):** 2026-09-21T13:42:00Z
> **Purpose:** Verified binary sha256 + smoke-test output BEFORE the operator runs `bash scripts/release.sh`.

## §0 History of this annex (3 versions)

| Version | Workspace | Binary sha256 | Status |
|---|---|---|---|
| v1.169.128 (superseded) | 1.169.128 | `4c361ddae53d7e9d96fe45880a5e2825ef68b9996fd274f1b0b2df5a230fdf4f` | Replaced — superseded by bump to 1.169.129 |
| v1.169.129 (superseded) | 1.169.129 | `7c70ce5fcff86f09a2cd72e8f5bcffd9ed58639321f874636cdd1e43e3a955e1` | Replaced — superseded by bump to 1.169.130 |
| **v1.169.130 (current)** | 1.169.130 | **`45543f1f76b13af16bb7d22d7f46a91d4f57d21e0e86496ccda6e37fce4e0aa4`** | **Active — admission ACCEPT, ready for operator publish** |

## §1 Binary built and verified (1.169.130)

```
$ cargo build --release -p sddk-cli
   Compiling sddk-cli v1.169.130 (...)
    Finished `release` profile [optimized] target(s) in 3m 10s
```

> **NOTE — TARGET DIR:** the build goes to `$CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets/release/sddk`, NOT `./target/release/sddk`.

| Property | Value |
|---|---|
| Path | `/var/home/rubentxu/cargo-targets/release/sddk` |
| Size | 30,778,216 bytes |
| Permissions | `-rwxr-xr-x` |
| mtime | 2026-09-21 13:41 |
| sha256 | `45543f1f76b13af16bb7d22d7f46a91d4f57d21e0e86496ccda6e37fce4e0aa4` |
| Reported version | `sddk 1.169.130` |

## §2 Smoke tests

| Test | Result |
|---|---|
| `sddk --help` | OK — emits `Deterministic SDDK workflow tooling — uses sddk agent-help …` |
| `sddk version` | `binary: 1.169.130` |
| `grep '^version' Cargo.toml` | `version = "1.169.130"` |
| `release_admission_check HEAD` | **ACCEPT 1.169.129 -> 1.169.130** (exit 0) |
| `git ls-remote --tags origin 'v1.169.130'` | (empty) → tag available |

## §3 Why this is the third bump in a row

The C0 cycle produced 6 docs-only commits (b0d6d40..2397d71). The release admission invariant requires `head_version > parent_version`. Each docs-only commit advanced HEAD without changing Cargo.toml, breaking the monotonic chain. To restore admission, three real bumps were needed:

| Bump | Commit | Purpose | Outcome |
|---|---|---|---|
| 1.169.127 → 1.169.128 | `96f5366` | PR #7 AGENTS.md integration | ACCEPT initially; broken by subsequent docs |
| 1.169.128 → 1.169.129 | `aff68b8` | Restore after C0 docs broke admission | ACCEPT initially; broken by `39a4104` reconcile |
| **1.169.129 → 1.169.130** | **`6fda463`** | **Restore after reconcile docs broke admission** | **ACCEPT — current HEAD** |

**Lesson recorded in journal entry 2026-09-21T13:42:00Z:** do NOT add docs-only commits between bump and release.sh. Either do all docs work BEFORE the bump, or add another bump afterward.

## §4 Action by operator

```bash
cd ~/Proyectos/agentesIA/sddk-framework
git fetch origin main
git checkout main && git pull --ff-only
# Confirm HEAD is 6fda463
git rev-parse HEAD
# Admission sanity check before running release.sh
source scripts/lib/release_admission.sh && release_admission_check HEAD
# Expected: ACCEPT 1.169.129 -> 1.169.130
# Then publish
bash scripts/release.sh   # publica v1.169.130
```

The release admission gate will accept. No `--force`, `--skip-tests`, or ceremonial markers needed.
