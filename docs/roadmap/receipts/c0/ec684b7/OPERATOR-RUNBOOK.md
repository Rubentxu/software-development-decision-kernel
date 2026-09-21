# OPERATOR-RUNBOOK — Cómo publicar v1.169.130 desde el estado actual

> **Slice id:** `p-63676b11dc0ef88f/c0-reconciliation-baseline`
> **Date (UTC):** 2026-09-21T13:48:00Z
> **Audience:** HUMAN OPERATOR who will run `bash scripts/release.sh`
> **Status:** STOP — orquestador no puede publicar (system-law `git.release`). El operador debe ejecutar este runbook.

## §0 TL;DR

El workspace está en `1.169.130` con admisión rota en HEAD por 3 commits docs-only sobre el último bump (`6fda463`). El operador debe ejecutar **uno** de los dos comandos abajo para llegar a un HEAD con admisión ACCEPT y luego correr `bash scripts/release.sh`.

## §1 Estado actual verificado

```text
HEAD               = ec684b7  (origin/main)  → admisión REJECT
last public release = v1.169.122 (2026-09-20)
workspace          = 1.169.130

SHAs con admisión ACCEPT (verificadas por `release_admission_check`):
  - 6fda463  chore(release): bump version a 1.169.130  ← ÚLTIMO publicable
  - aff68b8  chore(release): bump version a 1.169.129  ← también publicable
```

**Diagnóstico**: el invariante de admisión exige `head_version > parent_version`. Cada commit docs-only que se añadió sobre el bump (`a77ad2e`, `6668c3e`, `ec684b7`) rompió la cadena. El operador debe volver a un SHA que sea el bump en sí mismo.

## §2 Comando exacto (recomendado, opción A — operator side reset)

```bash
cd ~/Proyectos/agentesIA/sddk-framework

# 1. Verificar que no hay trabajo local sin pushear
git status -sb

# 2. Reset local al último SHA con admisión ACCEPT
#    (NO se pushea — es reset local del operador; main remota queda en ec684b7)
git reset --hard 6fda463

# 3. Verificar admisión antes de release
source scripts/lib/release_admission.sh && release_admission_check HEAD
#    Expected output: ACCEPT 1.169.129 -> 1.169.130

# 4. Publicar
bash scripts/release.sh
```

Tras la publicación:
- Tag `v1.169.130` existe local y remoto.
- GitHub Release `v1.169.130` está publicada con 9 assets.
- Install local en `~/.local/share/sddk/framework/1.169.130/`.
- `sddk version` reporta `binary: 1.169.130, resolved: 1.169.130`.

Si el operador quiere re-trazar los commits docs (`a77ad2e`, `6668c3e`, `ec684b7`) tras la release, están en `git reflog` y pueden re-aplicarse con cherry-pick después de un nuevo bump ceremonial.

## §3 Comando exacto (opción B — checkout detached)

Si el operador prefiere NO mover `main`:

```bash
cd ~/Proyectos/agentesIA/sddk-framework
git checkout 6fda463
bash scripts/release.sh
```

Pero `release.sh` exige estar en `main` (step 0 preflight check):

```
$ grep "must be on main" scripts/release.sh
```

Por tanto la opción A es la única viable. El operador debe hacer `reset --hard` local.

## §4 Comando exacto (opción C — autorizar 4º bump)

Si el operador quiere que el orquestador aplique un 4º bump ceremonial (`1.169.130 → 1.169.131`) para restaurar admisión en HEAD sin mover main:

1. Operador me autoriza explícitamente con una nueva directriz.
2. Orquestador aplica bump ceremonial con cambio real en Cargo.toml.
3. Orquestador pushea.
4. Operador corre `bash scripts/release.sh` desde HEAD (admisión ACCEPT).

**Riesgo de la opción C**: el operador previamente desaconsejó "bumps ceremoniales sin publicar". Si la release falla de nuevo, el bump se queda sin publicar y suma marker. La opción A es estructuralmente más limpia.

## §5 Verificación post-release

Tras `bash scripts/release.sh` completar exitosamente, el orquestador debe:

1. Re-correr T01 (8 pasos: rev-parse HEAD, status, log, tag, gh release, version, workspace, STATE).
2. Marcar C0 CLOSED en `docs/roadmap/STATE.yaml`.
3. Re-aplicar cherry-pick de los commits docs (`a77ad2e`, `6668c3e`, `ec684b7`) si el operador lo desea — esos commits contienen: smoke annex actualizado, journal entries, STATE.yaml/CURRENT.md reconciliados.
4. Iniciar C1 SCOPE-CONTRACT + H01 fix.

## §6 Resumen de SHAs y binarios

| SHA | Tipo | Admisión | Binario sha256 | Notas |
|---|---|---|---|---|
| `ec684b7` | docs (journal stop) | REJECT | n/a | HEAD actual |
| `6668c3e` | docs (reconcile 1.169.130) | REJECT | n/a | |
| `6fda463` | **bump 1.169.130** | **ACCEPT** | `45543f1f76b13af16bb7d22d7f46a91d4f57d21e0e86496ccda6e37fce4e0aa4` | **Publicable** |
| `aff68b8` | bump 1.169.129 | ACCEPT | `7c70ce5fcff86f09a2cd72e8f5bcffd9ed58639321f874636cdd1e43e3a955e1` | También publicable |
| `96f5366` | bump 1.169.128 (PR #7 AGENTS) | REJECT ahora (padre también docs) | (binario v1.169.128) | Histórico |

## §7 Lo que NO hacer

- **NO** intentar `git push --force` (prohibido por AGENTS y por la directriz del operador).
- **NO** hacer un 4º bump sin autorización explícita del operador (la opción C requiere nueva directriz).
- **NO** modificar `scripts/release.sh` ni `scripts/lib/release_admission.sh` (prohibido por la directriz).
- **NO** saltar la release con `--skip-tests` u opciones equivalentes (prohibido).

## §8 Contacto

El orquestador esperará la publicación del operador. Cuando se complete:

1. Operador puede pedir al orquestador que re-corra T01 desde la sesión nueva.
2. Operador puede pedir cherry-pick de los commits docs preservados en reflog.
3. Operador puede iniciar C1 con un nuevo turno de directriz.

## §9 Estado de la iniciativa

- **C0**: cerrado en orquestador (PASS_OBSERVED_WITH_NOTES), pendiente de CLOSED formal tras release.
- **C1**: substance acumulada (3 preflights + 1 research notes), SCOPE-CONTRACT no emitido (bloqueado por C0 CLOSED).
- **Release**: pendiente de operador.
- **Bumps ceremoniales sin publicar**: 3 (96f5366 → 1.169.128, aff68b8 → 1.169.129, 6fda463 → 1.169.130). El operador publica uno de ellos (preferentemente 6fda463) y los 2 anteriores quedan históricos sin release tag asociada — esto es **legítimo** porque cada bump fue mecánico y la release consolida el último.

## §10 Lección registrada

NO añadir commits docs-only entre bump ceremonial y release.sh. El invariante de admisión es monotónico contra el primer padre de HEAD, no contra el último release público. Para futuros ciclos, el patrón canónico es:

```
[docs-only commits sin bump]
        ↓
[un solo bump ceremonial al final]
        ↓
[publicación inmediata]
        ↓
[post-release: nuevos docs admitidos por rule B del pre-push]
```
