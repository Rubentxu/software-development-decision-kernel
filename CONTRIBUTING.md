# Contributing to SDDK Framework

> Conventions, workflow, and review process for contributing to this repo.
> Read this **before** opening a PR — the workspace separation rules and
> commit conventions are not negotiable.

---

## 1. Code of intent

`sddk-framework` is the **development repo** (NOT adopted). It contains
crates, docs, CI, releases, agents/skills/prompts **source**. Every change,
commit, push and release happens from
`~/Proyectos/agentesIA/sddk-framework/` (CWD).

The project **never writes inside other project repos** (rule "zero intrusion",
see `docs/responsibility-separation/SPEC.md`). The runtime bundle lives in
`$SDDK_DATA_DIR/framework/<version>/` (`~/.local/share/sddk/framework/<v>/`)
and is updated with `sddk dev install`.

---

## 2. Hard conventions (non-negotiable)

### 2.1. Namespace boundary

- Gentle AI SDD and SDDK are distinct systems. Their agents, skills, prompts
  and persistence contracts do not mix.
- The historical name "SDD-kernel" is normalized to **SDDK**.
- The active surface is `orchestrator`, `sddk-*` and `prompts/sddk/`. No aliases.

### 2.2. Commits — Conventional Commits in Spanish

```
feat(uat): …
fix(uat): …
chore(release): …
docs(adr): …
test(cli): …
refactor(domain): …
```

- **One concern per commit.** If a change touches docs + code, that's one
  commit with the concern explained in the body.
- **No `Co-Authored-By` or AI attribution.** Commits are the human's.
- Commits to `main` via `git push origin main` (no PRs — linear project with
  `vX.Y.Z` tags).

### 2.3. Branch model

- `main` is the single branch for development + releases.
- No `develop`, `release/*`, or hotfix branches.
- Any feature is committed directly to `main` (or squash-merged in external PRs).

### 2.4. Workspace

- `Cargo.toml` `[workspace.package] version` = current development version
  (may go ahead of the latest tag until `chore(release)`).
- `cargo test --workspace` green + `cargo clippy --workspace` 0 errors
  **before** committing (see checklist §6).

### 2.5. Memory + Engram

- Long sessions MUST close with `engram_mem_session_summary` (goal,
  discoveries, accomplished, next steps, relevant files). Survives
  compaction. Rules in `~/.config/opencode/skills/...`.

### 2.6. CI local-first, cloud async

- **The verification gate is LOCAL**: `cargo test --workspace` +
  `cargo clippy --workspace` + `cargo fmt --all -- --check` before
  committing. GitHub Actions cloud **does NOT block**: no required status
  checks, runs = asynchronous evidence.
- **Forbidden**: wait for cloud runs (`gh pr checks --watch`, delay
  push/merge for CI) or "fix CI" without reproducing locally first.
- **Workflows locally**: `act` v0.2.89 (`/usr/local/bin/act`) + podman;
  `ubuntu-latest` mapped to `catthehacker/ubuntu:rust-latest` via
  `~/.config/act/actrc`. Example:
  `act pull_request -W .github/workflows/<wf>.yml`.
- The GitHub free plan minutes are exhausted — the cloud may not even
  run; trust the local gate.

---

## 3. Workflow paths

SDDK classifies work into 4 paths. The orchestrator picks one during triage.

| Path | When | Phases |
|------|------|--------|
| **B-direct** | Hotfix, bounded task, C3 | Load skill → execute → light verify → release → archive |
| **A-min** | Simple change, C2 context | spec → apply → verify → debt-verify (smoke) → release → archive |
| **A-lite** | Bounded work, C1 context | propose → spec → apply → verify → debt-verify (standard) → release → archive |
| **A-full** | Architectural, new domain, C0 | explore → propose → spec ∥ design → tasks → apply → verify → debt-verify (deep) → release → archive |

Read `prompts/sddk/mcw.md` (Mandatory Complete Workflow) before modifying
any phase agent or the orchestrator.

---

## 3.5. Distribution (GitHub Releases + install.sh, asdf-vm style)

Since v1.28.0 SDDK distributes **pre-compiled binaries** via GitHub Releases.
Users install with a one-liner (rustup/mise model):

```bash
curl -fsSL https://raw.githubusercontent.com/Rubentxu/software-development-decision-kernel/main/scripts/install.sh | bash
```

The script `scripts/install.sh` (244 lines):
- Detects platform (`uname -s/m`) → asset `sddk-linux-{x86_64,aarch64}-musl`
  (Linux: **musl static**, runs on any distro regardless of glibc)
- Downloads binary + `sha256` from GitHub Releases
- Verifies SHA256 before installing (fails on mismatch)
- If `cosign` is available, verifies keyless signature (optional)
- Asks which editor to configure (opencode/zcode/claude/codex or all)
- Downloads `software-development-decision-kernel.tar.gz` (bundle: `agents/`, `skills/`,
  `prompts/sddk/`, `assets/`, `MANIFEST.sha256`) and extracts it into
  `$SDDK_DATA_DIR/framework/<v>/`
- Runs `sddk dev link --editor <X>` (symlinks the bundle to the editor dir)
- Prints `sddk dev doctor` (final verification)

**Supported platforms in v1.28.0:**
- ✅ Linux x86_64 (musl static)
- ✅ Linux aarch64 (musl static)
- ⏳ macOS x86_64 + arm64 (pending: `cargo-zigbuild` toolchain installed,
  binaries need to be generated and uploaded)
- ⏳ Windows x86_64 (pending: requires `#[cfg(unix)]` in code that uses
  `std::os::unix::*` — see `crates/sddk-cli/src/dev_cmd.rs`)

**Local-first release (manual):** the tag is pushed first (`git tag vX.Y.Z &&
git push origin vX.Y.Z`), then the binary is uploaded to GitHub Releases.
Workflow `.github/workflows/release.yml` is in `workflow_dispatch` mode
since 2026-08-10 (CI minutes exhausted); today's operational path is:

```bash
# 1. Tag + push (local)
cargo build --release --target x86_64-unknown-linux-musl -p sddk-cli --locked
git tag vX.Y.Z && git push origin vX.Y.Z

# 2. Stage assets (Linux x86_64 + aarch64)
./target/x86_64-unknown-linux-musl/release/sddk release dist \
  --prefix dist-amd64 --channel release --commit "$(git rev-parse HEAD)"
cp dist-amd64/dist/sddk sddk-linux-x86_64-musl
cp dist-amd64/dist/{checksums.txt,sbom.json,attestation.json} \
   sddk-linux-x86_64-musl.{CHECKSUMS,sbom.json,attestation.json}
sha256sum sddk-linux-x86_64-musl > sddk-linux-x86_64-musl.sha256
# (repeat for aarch64)

# 3. Framework bundle
tar czf software-development-decision-kernel.tar.gz agents skills prompts/sddk assets MANIFEST.sha256
sha256sum software-development-decision-kernel.tar.gz > software-development-decision-kernel.tar.gz.sha256

# 4. gh release create
gh release create vX.Y.Z --repo Rubentxu/software-development-decision-kernel \
  --target <commit> --title "vX.Y.Z" --notes "..." \
  sddk-linux-x86_64-musl sddk-linux-x86_64-musl.{sha256,CHECKSUMS,sbom.json,attestation.json} \
  sddk-linux-aarch64-musl sddk-linux-aarch64-musl.{sha256,CHECKSUMS,sbom.json,attestation.json} \
  software-development-decision-kernel.tar.gz software-development-decision-kernel.tar.gz.sha256
```

The E2E smoke test lives in `.github/workflows/release.yml:170-217` and runs
automatically when CI is available.

---

## 4. Layout (asdf-vm inspired)

Inspired by `asdf-vm` (tool versions, per-version shims, `path:` override).
Canonical spec: `docs/responsibility-separation/SPEC.md`.

### 4.1. Three separated roles

| Role | Location | Contents | Adopted | Linked |
|------|----------|----------|---------|--------|
| **Development repo** | `~/Proyectos/agentesIA/sddk-framework/` (CWD) | `crates/`, `docs/`, `agents/`, `skills/`, `prompts/`, CI, releases | NO | NO |
| **Runtime bundle** | `~/.local/share/sddk/framework/<v>/` | Snapshot: `agents/`, `skills/`, `prompts/`, `workflows/`, `assets/` | — | YES → `$HOME/.config/{opencode,claude,kilo,codex}/` |
| **Usage workspace** | User repos | Project + optional `.sddk-versions` | YES | NO |

### 4.2. Version resolution (lookup in order)

1. `$PWD/.sddk-versions`
2. `.sddk-versions` in parent directories up to the root
3. `$SDDK_DATA_DIR/framework/current` (global symlink)

Format (managed by the developer, NEVER by the framework):
```text
sddk 1.5.3
sddk current         # follows the global symlink
sddk path:../..      # dogfooding (CWD = sddk-framework)
sddk system          # system installation
```

### 4.3. Zero intrusion

| Operation | Before (bad) | Now (good) |
|-----------|--------------|-----------|
| Adoption | `workflow/workflow.yaml` planted in repo | Receipt in `~/.local/share/sddk/projects/<id>/` |
| Cycle artifacts | `sddk/{change}/...` in repo | `~/.local/share/sddk/projects/<id>/cycle-artifacts/{cycle_id}/` |
| Generated docs | `docs/generated/` in repo | `~/.local/share/sddk/projects/<id>/generated/` (or `--in-repo` for dogfooding) |
| Telemetry | `~/.sddk-shared/uat-results.sqlite` | Always XDG, never in repo |

---

## 5. Golden rules

### 5.1. ALWAYS work from the CWD (`sddk-framework/`)

- ✅ `cd ~/Proyectos/agentesIA/sddk-framework && git … && cargo …`
- ❌ `cd ~/.sddk-shared/ && …` — violates the "single source of truth in
  CWD" rule. **Do not create new checkouts in `~/.sddk-shared/`.**

### 5.2. The runtime bundle lives in `~/.local/share/sddk/framework/<v>/`

- Updated via `sddk dev install` (or `sddk dev update`).
- **Not a git checkout.** It is a published snapshot.
- **Do not edit `~/.local/share/sddk/...` directly** — it is overwritten
  on the next install.

### 5.3. The runtime bundle is NOT a checkout of the repo

- `agents/`, `skills/`, `prompts/` are **copies**, not symlinks. `bootstrap.sh`
  symlinks them to each editor's directories.

### 5.4. Design decisions have one canonical surface per scope

- `docs/adr/` (this repo) — public-project ADRs.
- `docs/sddk-decision-kernel-architecture/03-adrs/` — active target-architecture
  ADRs; its roadmap lives in `02-roadmap/`.
- `~/.sddk-knowledge/<project>/adrs/` — adopted-project ADRs.
- Plan specs live in `~/.sddk-knowledge/<project>/specs/`.

---

## 6. Pre-commit checklist

```text
[ ] cargo fmt --all -- --check                    # formatting clean
[ ] cargo build --release -p sddk-cli             # compiles
[ ] cargo test --workspace                        # all green
[ ] cargo clippy --workspace                      # 0 errors
[ ] If you touched assets/: sddk dev install      # runtime bundle updated
[ ] After release: sddk dev doctor | grep bundle_coherence (binary == bundle)
[ ] If you touched the model TUI: bash tests-e2e/tui/run.sh
[ ] git status                                    # clean
[ ] git diff                                      # review what you'll commit
[ ] commit message: feat(uat): … o fix(uat): …
[ ] git push origin main                          # push
```

### 6.1. One-time setup after clone

After cloning the repository, activate the pre-push hook that enforces the
apply/release split discipline:

```bash
git config core.hooksPath githooks
```

This hook rejects any `git push` to `refs/heads/main` unless the push contains
at least one commit with a subject matching `^chore\(release\): bump version`.

---

## 7. Process for proposing changes to phase agents or the orchestrator

Phase agents (`sddk-explore`, `sddk-propose`, `sddk-spec`, `sddk-design`,
`sddk-tasks`, `sddk-apply`, `sddk-verify`, `sddk-debt-verify`,
`sddk-archive`, `sddk-release`) and the orchestrator are the **heart of the
framework**. Changes here cascade to every adopted project.

Before modifying any of them:

1. **Read the canonical spec**: `prompts/sddk/mcw.md` (MCW) and
   `prompts/sddk/orchestrator.md` (orchestrator).
2. **Open an issue** describing the change with motivation, alternatives
   considered, and impact on existing cycles.
3. **Write or update the relevant ADR** in `docs/adr/` for current-runtime
   behavior, or in `docs/sddk-decision-kernel-architecture/03-adrs/` for the
   target architecture.
4. **Test locally** with at least one full A-lite cycle end-to-end
   (sddk new → verify → debt-verify → release → archive).
5. **Tag a release** (`chore(release): vX.Y.Z`) only after the change has
   landed on main and been verified by a real cycle.

---

## 8. Releasing

Releases follow `prompts/sddk/phases/release.md` and `MCW Phase 3`.

1. Update `Cargo.toml` workspace version, `manifest.toml`, and
   `crates/sddk-pack-uat/pack-uat.toml` in a single `chore(release):`
   commit.
2. Run `sddk release apply --route local` (or `sddk release plan` first
   to inspect).
3. Push the annotated tag: `git push origin vX.Y.Z`.
4. Run `sddk dev install --prefix ~/.local --source .` to refresh the
   local runtime.
5. Run `sddk dev doctor` — expect `all_present: true` and
   `binary.bundle_coherence: present`.

---

## 9. Summary in one line

> **The project is the CWD** (`sddk-framework/`). The runtime bundle
> lives in `~/.local/share/sddk/framework/<v>/` (installed via
> `sddk dev install`). All code changes go to the CWD; all publishable
> content is copied to the bundle with `sddk dev install`.

---

## 10. See also

- Regression history: `docs/history/AGENTS-history.md`
- Current handoff: `docs/handoff/HANDOFF-2026-08-13-sddk-framework.md`
- Architecture roadmap:
  `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
- Conventional Commits:
  <https://www.conventionalcommits.org/en/v1.0.0/>
- Keep a Changelog: <https://keepachangelog.com/en/1.1.0/>
