# RECEIPT — C4 v1.171.0 release

**Release:** v1.171.0
**Date:** 2026-09-22T17:11:44Z (published) / 2026-09-22T17:12:01Z (installed locally)
**Tag SHA:** db1e2e44bc64034f44238b6cf250e6bddaa6addb
**Binary SHA256:** `5e9d5fbd17d94b8c53763cdca0a70435b8eabedcd72e521cce21f1c2335ef9f3`
**Pipeline:** 14/14 steps PASS via `bash scripts/release.sh`
**Override SemVer:** LIFTED (v1.171.0 = SemVer-correct minor for 1 feat + 3 test + 1 fix since v1.170.3)

## Audit trail

| Step | Action | Status | Evidence |
|------|--------|--------|----------|
| 0 | Preflight (main, clean, HEAD=bump commit, gh auth) | PASS | main@db1e2e44, on main, clean, gh logged in as Rubentxu |
| 1 | cargo fmt + clippy + test (workspace) | PASS | fmt --check OK, clippy -D warnings OK, cargo test --workspace exit 0 |
| 2 | Read version (workspace Cargo.toml) | PASS | 1.171.0 |
| 3 | Build binary (cargo build --release --bin sddk) | PASS | sddk 1.171.0 |
| 4 | Manifest (sddk dev manifest --root . + --verify) | PASS | verify_manifest OK |
| 5 | Bundle tarball (sddk-v1.171.0-...musl.tar.gz) | PASS | sha256 fc732ec01748733bf438c260e3790fbed8bac11517017a0929347aa43227ba62 |
| 6 | BUNDLE.toml (schema_version=2) | PASS | manifest_sha256 = 608c6d9c... (coherent) |
| 7 | Unified tarball (bin/sddk + framework/) | PASS | tar tvzf confirms -rwxr-xr-x on sddk |
| 8 | sha256 + CHECKSUMS + sbom (CycloneDX 1.5) | PASS | 3 artifacts emitted |
| 9 | gh release create v1.171.0 (6 assets) | PASS | https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.171.0 |
| 9b | Public-release gate (tests/test_release_public_gate.sh, 10 scenarios) | PASS | all 10 scenarios pass |
| 10 | Install from real URL (bash scripts/install.sh --version v1.171.0 --editor all) | PASS | exit 0, exec bit on bin/sddk |
| 11 | sddk dev doctor --prefix | PASS | all_present=true, binary.bundle_coherence=present |
| 12 | sddk dev update --prune-only --keep 1 | PASS | kept 1.171.0 |
| 13 | Final state | PASS | binary=bundle=current=1.171.0 |

## Features delivered (since v1.170.3)

- **FC-6** `sddk vault show <node-id>` — resuelve un nodo del vault por id (frontmatter `id` o file stem) y lo renderiza con metadata + body + backlinks. Caso de uso: leer ADR sin buscar el archivo. Verificado en binario real contra `~/.sddk-knowledge/sddk-framework` ADR-0142-RELEASE-SCRIPT-SEMVER-CORRECTNESS (resuelve id, kind, path, status, wikilinks, body).
  - `VaultCommand::Show(VaultShowArgs)`
  - `VaultShowOutput { node: VaultNode, backlinks: Vec<String> }` con `derive(serde::Serialize)` — JSON path funcional
  - `vault_show_text` rendering legible
  - Capability `vault.show` (risk: low, consequence: read) añadida a `workflow/workflow.yaml`
  - 6 vault_cmd tests pass (3 nuevos + 3 normalize_cycle_target_*)

## Fixes bundled

- **fix(release-bump):** el sed `s/^version = "$CURRENT"/.../` fallaba silenciosamente cuando workspace ≠ last tag. Ahora lee `WORKSPACE_VERSION` desde workspace Cargo.toml (filesystem, no git ref) antes del loop. Repro: desde v1.170.3 con workspace en 1.170.4, `release-bump.sh --force-version 1.170.5` actualiza Cargo.toml correctamente.

## Test counts

- `cargo test --workspace`: 5037 passed; 0 failed; 5 ignored (workspace stable since session-11)
- `cargo test -p sddk-cli --test cli`: 185 passed; 0 failed (incluye 1 nuevo `cli_vault_show_resolves_node_and_renders_json_and_text`)
- `cargo test -p sddk-cli --lib`: 784 passed; 0 failed; 1 ignored

## UAT-EVIDENCE

- `UAT-EVIDENCE-T29-v1.171.0.yaml`: 6 falsifiers, 0 triggered. FC-6 end-to-end en binario PATH.
- `UAT-EVIDENCE-T31-v1.171.0.yaml`: 6 falsifiers, 0 triggered. 5-way coherence (tag/HEAD/origin/asset/PATH).

## Override SemVer audit

Override activo desde v1.170.3 (operador: "saltar al 1.170.x" para no publicar v1.171.0 sin features suficientes). En esta sesión se extendió a v1.170.4/5/6/7 con feat+test+fix+regression. Reconociendo que la duración del override había superado lo razonable, **se levanta en v1.171.0** — algoritmo SemVer coincide con decisión: 1 feat(vault) = minor.

**Si la próxima release es solo fix:/test:/docs:/chore:, el algoritmo emitirá patch (v1.171.1) sin override**, completando el ciclo de retorno a SemVer-correct.

## Resolved desfase

El binario en `~/.local/bin/sddk` era v1.170.3 (pre-FC-6) al cierre de session-11. v1.171.0 cierra ese desfase — FC-6 ya está en PATH.

## Risk register

- C2 (cognicode-mcp / chronos-mcp / jcode-sdk) sigue NOT_EVALUATED — no resuelto en este ciclo. Binarios externos ausentes; UAT reales de C2 no posibles.
- Override SemVer historical drift: v1.170.3 fue publicado con override (deuda menor), pero el binario era coherente.
- Cycle 16 tail (INC-DEBT-006 cluster): apply-push-discipline related; las puertas mecánicas siguen activas.

## References

- UAT-EVIDENCE-T29: `docs/roadmap/receipts/c4-release-v1.171.0/UAT-EVIDENCE-T29.yaml`
- UAT-EVIDENCE-T31: `docs/roadmap/receipts/c4-release-v1.171.0/UAT-EVIDENCE-T31.yaml`
- GH release: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.171.0
- FC-6 inventory: `docs/roadmap/FEATURE-CANDIDATES.md`
- v1.170.3 RECEIPT (previous release, override baseline): `docs/roadmap/receipts/c4-release-v1.170.3/RECEIPT.md`
