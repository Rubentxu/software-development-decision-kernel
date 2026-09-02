---
id: INC-DEBT-020
title: "prune re-apunta current a dev-link rompiendo bundle_coherence"
slug: "INC-DEBT-020-prune-reapunta-current-a-dev-link-rompiendo-bundle-coherence"
status: open
severity: high
priority: P1
fingerprint: "d4a7c3e91f8b62e7c9b6d8a2f5e4c1b3d9a7e6f8a2b4c5d6e7f8a9b0c1d2e3f"
fingerprint_aliases: []
cluster_id: CL-20
created: 2026-09-02
created_by: sddk-archive
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

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-02 | sddk-archive | created | cycle-52 release step 12 doctor output |
| 2026-09-02 | sddk-archive | status: open | not yet fixed |

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

## References

- Release receipt: `release-receipt.json` (cycle-artifacts/kernel-cycle-52-context-inference/)
- Archive manifest: `archive-manifest.md` (cycle-artifacts/kernel-cycle-52-context-inference/)
- docs/RELEASING.md § step 12 (prune verification)
- INC-DEBT-017 (cluster CL-17, same install-coherence area)
