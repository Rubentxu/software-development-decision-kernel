---
id: INC-DEBT-020
title: "prune re-apunta current a dev-link rompiendo bundle_coherence"
slug: "INC-DEBT-020-prune-reapunta-current-a-dev-link-rompiendo-bundle-coherence"
status: resolved
resolved_by: kernel-cycle-53-frontier-advisor (v1.68.0)
resolved_by_commits:
  - "4df6240 fix(cli): prune re-apunta current a la versión más nueva (INC-DEBT-020)"
  - "f76ba5f test(cli): regresión prune — current symlink post-condición (INC-DEBT-020)"
  - "669d7dc fix(engine): kinds desconocidos de Requirement::Structured = unmet en frontera (OVG-01)"
resolution: "Opción A — prune re-resuelve current → newest kept version después de eliminar stale dirs"
severity: high
priority: P1
fingerprint: "d4a7c3e91f8b62e7c9b6d8a2f5e4c1b3d9a7e6f8a2b4c5d6e7f8a9b0c1d2e3f"
fingerprint_aliases: []
cluster_id: CL-20
created: 2026-09-02
created_by: sddk-archive
closed_by: sddk-archive (cycle-53 archive)
owner: next-tooling-cycle
affected_versions: ["1.67.0"]
discovery_context:
  command: "sddk dev update --prune-only --keep 1"
  environment: "dev-link mode (framework/current → framework/)"
  symptom: "binary.bundle_coherence flips to missing after prune"
  trigger: "post-release step 12 (doctor after prune)"
  affected_path: "~/.local/share/sddk/framework/current"
---

# INC-DEBT-020 — prune re-apunta current a dev-link rompiendo bundle_coherence

> Durable record for debt finding INC-DEBT-020. Found in cycle-52 release step 12
> (doctor post-prune). See ADR-0047 §3.2.

## Context

El comando `sddk dev update --prune-only --keep 1` elimina todas las versiones
instaladas excepto la más reciente (1.67.0). Sin embargo, tras ejecutar el prune,
el symlink `~/.local/share/sddk/framework/current` apunta a `framework/` (el
directorio base dev-link) en lugar de apuntar a `1.67.0/` (el directorio de la
versión más reciente).

Esto provoca que `sddk dev doctor` reporte:
```
binary.bundle_coherence: missing
all_present: false
```

El bundle coherence check espera que `current` sea un directorio de versión
(`1.67.0/`) pero recibe `framework/` — un symlink al bundle source en dev
checkout.

## Problem

1. `sddk dev update --prune-only` en dev-link mode no debería tocar `current`
   cuando el target de `current` es el dev checkout (`framework/`), no un
   directorio de versión.
2. El prune elimina la versión anterior (1.66.6) y actualiza `current` como efecto
   colateral, pero el nuevo target no se resuelve correctamente.
3. El resultado es que `binary.bundle_coherence` queda en `missing` hasta que
   alguien ejecuta manualmente: `rm current && ln -s 1.67.0 current`.

## Rationale

| Atributo | Valor | Justificación |
|----------|-------|---------------|
| severity | high | Rompe la garantía de install coherence (el gate fundamental de `sddk dev install/update`) |
| priority | P1 | Puede afectar a cualquier usuario tras un prune; se resuelve en el ciclo actual o siguiente |
| cluster_id | CL-20 | Cluster de tooling de install/update/prune |
| owner | next-tooling-cycle | El ciclo que implemente la corrección |

**Impacto**: every prune operation in dev-link mode leaves the installation in
a broken coherence state. The user must manually fix the symlink after every
prune.

**Workaround**: `rm current && ln -s <version> current` (where `<version>` is the
remaining version after prune).

## Fix Direction

Dos opciones documentadas para resolver:

1. **Opción A**: prune debe re-resolver `current` → newest remaining version
   después de eliminar stale dirs. El `current` symlink debe apuntar siempre al
   directorio de versión más reciente幸存 (no al dev checkout).

2. **Opción B**: refuse dev-link mode cuando se invoca `--prune-only` fuera de
   un dev checkout. Si `current` es un symlink a `framework/` (dev checkout),
   el prune no debe modificarlo y debe emitir un warning en su lugar.

**Recomendación inicial**: Opción A (re-resolve after prune) porque es más
robusta y no cambia el comportamiento de prune en el caso normal.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-02 | sddk-archive | created | cycle-52 release step 12 doctor output |
| 2026-09-02 | sddk-archive | status: open | not yet fixed |
| 2026-09-02 | sddk-archive | status: resolved | cycle-53 archive — v1.68.0 dogfooded |

## Closure Evidence

### Fix Implementation (commits 4df6240 + f76ba5f)

**Commit `4df6240`** introduces `repoint_current_to_newest(framework_dir, newest_version: &str)` at `crates/sddk-cli/src/dev/update.rs:127-136` — an atomic-swap helper that symlinks `current` to the newest kept version directory. Called from both prune branches:

- `update.rs:315` — normal `--prune` path
- `update.rs:344` — `--prune-only` path

**Commit `f76ba5f`** adds regression tests:
- `update_prune_only_repoints_current_to_newest_dev_link_mode` — asserts `current` points to `1.65.0/` after pruning to keep 1 in dev-link mode
- `update_preserves_current_in_normal_mode` — asserts current is preserved when root≠"."
- `s_dev_link_preserved_without_prune_flags_keeps_dev_link_target` — asserts dev-link block guard preserves `current → framework/`

### Dogfooding (v1.68.0 release — INC-DEBT-020 dogfooded)

```
# PRE-PRUNE STATE (v1.68.0 installed, 1.67.0 also present)
$ ls ~/.local/share/sddk/framework/ | grep -E '^[0-9]'
1.67.0
1.68.0
$ readlink ~/.local/share/sddk/framework/current
/home/rubentxu/.local/share/sddk/framework/1.68.0

# RUN PRUNE (the shipped INC-DEBT-020 fix)
$ sddk dev update --prune-only --keep 1 --root ~/.local/share/sddk/framework
prune-only: removed 1 stale bundle(s); kept 1.68.0
  removed: 1.67.0
  current -> 1.68.0

# POST-PRUNE STATE
$ readlink ~/.local/share/sddk/framework/current
/home/rubentxu/.local/share/sddk/framework/1.68.0
$ ls ~/.local/share/sddk/framework/ | grep -E '^[0-9]'
1.68.0

# POST-PRUNE DOCTOR (the dogfood check)
$ sddk dev doctor --prefix $HOME/.local/bin
binary.bundle_coherence: present
all_present: true
```

**Verdict**: INC-DEBT-020 fully fixed and dogfooded. The prune operation removed `1.67.0`, kept `1.68.0`, and repointed `current → 1.68.0` automatically via `repoint_current_to_newest`. No manual `ln -sfn` required. Doctor confirms `bundle_coherence: present` post-prune.

### Regression Tests (update.rs)

The fix is guarded by 3 regression tests (all pass on v1.68.0):
- `prune_repoint_current_after_prune_keeps_valid_target` — exercises `repoint_current_to_newest` directly
- `prune_repoint_current_skips_nonexistent_version_dir` — edge case: newest dir doesn't exist
- `s_dev_link_preserved_without_prune_flags_keeps_dev_link_target` — non-prune paths preserved

## References

- Release receipt: `release-receipt.json` (cycle-artifacts/kernel-cycle-52-context-inference/)
- Archive manifest: `archive-manifest.md` (cycle-artifacts/kernel-cycle-52-context-inference/)
- docs/RELEASING.md § step 12 (prune verification)
- INC-DEBT-017 (cluster CL-17, same install-coherence area)
- cycle-53 archive manifest: `cycle-artifacts/kernel-cycle-53-frontier-advisor/archive-manifest.md`
- cycle-53 GH Release: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.68.0
