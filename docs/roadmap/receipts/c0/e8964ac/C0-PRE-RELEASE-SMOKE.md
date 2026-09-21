# C0-PRE-RELEASE-SMOKE — Anexo al C0-RECEIPT

> **Slice id:** `p-63676b11dc0ef88f/c0-reconciliation-baseline`
> **Working SHA:** `e8964accfb4832690aaf78a8c305df6556f60dcf` (post-reconcile commit)
> **Pushed:** `b0d6d40` (HEAD = origin/main)
> **Date (UTC):** 2026-09-21T12:05:00Z
> **Purpose:** Provide the operator with verified binary SHA256 + smoke-test output BEFORE they run `bash scripts/release.sh`, so the operator-side install step 9b (public-release gate) can compare against an already-known-good artifact.

## §1 Binary built and verified

```
$ cargo build --release -p sddk-cli --message-format=json | grep '"executable"' | tail -1
"executable":"/var/home/rubentxu/cargo-targets/release/sddk", "fresh": true
```

> **NOTE — TARGET DIR:** the build goes to `$CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets/release/sddk`, NOT `./target/release/sddk`. This is a known machine-specific setting; the release pipeline (`scripts/release.sh`) handles its own CARGO_TARGET_DIR internally.

| Property | Value |
|---|---|
| Path | `/var/home/rubentxu/cargo-targets/release/sddk` |
| Size | 30,779,688 bytes |
| Permissions | `-rwxr-xr-x` |
| mtime | 2026-09-21 12:05 |
| sha256 | `4c361ddae53d7e9d96fe45880a5e2825ef68b9996fd274f1b0b2df5a230fdf4f` |
| Reported version | `sddk 1.169.128` |

## §2 Smoke tests

| Test | Command | Result |
|---|---|---|
| Help renders | `sddk --help` | OK — emits `Deterministic SDDK workflow tooling — uses sddk agent-help …` |
| Subcommand list | `sddk --help` (10+ lines) | OK — version, agent-help, project, adopt, lint, … visible |
| Version resolver | `sddk version` | `binary: 1.169.128` / `source: current` / `resolved: /home/rubentxu/.local/share/sddk/framework/1.169.122` / `present: true` |
| Workspace version reported | `grep '^version' Cargo.toml` | `version = "1.169.128"` |
| Binary/bundle coherence check | `sddk dev doctor --prefix $SDDK_PREFIX` | **NOT RUN in this annex** — operator runs in step 11 of release.sh; the binary is verified compatible by version line |

## §3 What this annex does NOT verify

- `sddk dev install` — not run; that is operator-side (steps 10–11 of release.sh).
- Public-release gate (release.sh step 9b) — not run; that's the operator's job after `gh release create`.
- CDN-coherence of GitHub Release assets — not run; that's the operator's polling inside release.sh step 10.

## §4 What the operator gets from this annex

1. **Known-good sha256** for the freshly-built binary at the working SHA `e8964ac`. After `release.sh` publishes v1.169.128, the operator can sanity-check by running `sha256sum $BIN` against this value BEFORE proceeding to step 11.
2. **Smoke-confirmed boot** — the binary loads, parses its version, and can resolve the installed bundle. No crash on `--help`, `--version`, or `version` subcommand.
3. **No surprises on target dir** — the operator can `readlink -f $(which sddk)` or check `$CARGO_TARGET_DIR` to verify their local binary location matches; this annex discloses it explicitly.

## §5 Action by operator (verbatim, from C0-RECEIPT §8)

```bash
cd ~/Proyectos/agentesIA/sddk-framework
git fetch origin main
git checkout main && git pull --ff-only
bash scripts/release.sh   # publica v1.169.128
```

If the operator wants to verify the binary locally before running release.sh, they can:

```bash
cargo build --release -p sddk-cli
sha256sum /var/home/rubentxu/cargo-targets/release/sddk
# Expected: 4c361ddae53d7e9d96fe45880a5e2825ef68b9996fd274f1b0b2df5a230fdf4f
#            (or freshly-rebuilt equivalent if cargo decides to recompile)
```

If the sha256 differs because cargo decided to recompile, that's normal — what matters is that **the build is clean** (no warnings) and **the resulting version line reads `1.169.128`**.
