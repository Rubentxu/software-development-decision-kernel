# RECEIPT — session-48: C3i VERIFIED (CTX-UAT-002/003) + UAT caducado reparado

**Fecha (UTC):** 2026-09-30 · **Baseline al inicio:** `66110595` (v2.3.1 publicado, cierre session-47)
**Workspace al cierre:** `2.3.2` — **PUBLICADO** (run de CI 36751152773 success, 27 assets, no draft/prerelease, publishedAt 2026-09-30T17:32:49Z)

## Alcance y priorización (pre-flight)

Criterio del operador: regresiones → deuda técnica severa reciente → ciclos de roadmap.
Verificación de deuda (regla: "deuda" sin criterios vigentes no es deuda):

| Candidato | Severidad | Por qué NO entra |
|---|---|---|
| INC-AUDIT-S14-TEST-PORTS-UNCONSUMED | medium/P2 | Generalidad especulativa ya re-severizada a medium tras detectar consumidores internos reales |
| INC-AUDIT-S14-NO-STRUCTURED-LOGGING | medium/P2 | Observabilidad de amplio alcance, no defecto reciente ni regresión |
| INC-AUDIT-S14-RELEASE-FORCE-VERSION-ERGONOMICS | low/P2 | Ergonomía de session-14; el pipeline ya deriva la versión correctamente |

**Ninguna deuda severa reciente vigente.** INC-DEBT-041/038 ya resueltas en session-47.
→ Siguiente WorkItem por prioridad: **C3i**, cuyas dos UAT llevaban NOT_RUN con un bloqueo
supuestamente humano. **La premisa del bloqueo se verificó y era falsa**, lo que convirtió el
trabajo en algo más valioso que cerrar un NOT_RUN: destapó evidencia caducada.

## Hallazgo principal — evidencia UAT caducada que nadie detectó

`tests/uat_ctx_002_context_bootstrap.sh` estaba **ROJO** contra el binario actual mientras
`UAT-MATRIX.md` lo declaraba **PASS** desde session-40.

- **Síntoma OBSERVED:** `FAIL: bootstrap sin ciclo exited non-zero` con **stderr vacío**.
- **Diagnóstico:** el comando sí persistía adopción, binding y JSON tipado. INC-DEBT-042
  (session-46) cambió el contrato: sin capsule que reconstruir, `context bootstrap` sale con
  **código 4** y `status: no_capsule_source` — la degradación honesta de no reclamar `complete`.
  El script asumía exit 0.
- **Causa raíz (la importante):** **ningún job de CI ejecuta los scripts UAT de context**. Una
  premisa obsoleta pudo pudrirse durante dos sesiones sin que nada la notara. Un UAT que nadie
  ejecuta no es evidencia, es decoración.
- **Segunda herida:** el script emitía el literal `UAT CTX-002: PASS` mientras la fila
  **CTX-UAT-002** de la matriz seguía NOT_RUN — invitaba a leer un PASS donde no lo había.
  Literal cambiado a `UAT context bootstrap`.

## Trabajo entregado

| Commit | Tipo | Contenido |
|---|---|---|
| `0c167a6b` | fix(uat) | exit 4 aceptado como contrato válido; literal sin colisión. 4/4 PASS |
| `45087a17` | test(c3i) | `tests/uat_ctx_004_cycle_inference.sh`: CTX-UAT-002 + CTX-UAT-003 + recovery |
| `146e349b` | ci | UAT de context se ejecutan en el job espejo `shell-contracts`; timeout 15→25 min |
| `f6723cf5` | docs | Matriz UAT 002/003 → PASS con evidencia; C3i → VERIFIED (historia conservada) |
| `617e9981` | chore(release) | bump 2.3.1 → 2.3.2 (SEMVER derivado: 1 fix → PATCH) |
| `fd44a146` | chore(release) | BUNDLE.toml 2.3.2 (el guard `test_dev_install_source_guard` predijo el fósil exacto) |

## Release v2.3.2 (OBSERVED)

- **Orden respetado** (lección de session-47): push de commits → verificar `HEAD == origin/main`
  → **solo entonces** taggear. Tag anotado `309238c9`, peel `4952e88c` == `origin/main`.
- Run de CI **36751152773 completed success**. Release: `isDraft=false`, `isPrerelease=false`,
  `publishedAt 2026-09-30T17:32:49Z`, **27 assets**.
- **Gate 9b:** 27/27 assets HTTP 200, 0 fallos, sin esperas de CDN.
- **Gate 9c:** sha servido `364adbe0ff2d5ac2…` == sha256 del binario descargado;
  `cosign verify-blob` **Verified OK** (identity `release.yml@refs/tags/v2.3.2`).
- **Pasos 10-13:** `install.sh --version v2.3.2 --editor all` exit 0; `sddk 2.3.2`;
  `framework/current → 2.3.2`; doctor `content.manifest: present`,
  `binary.bundle_coherence: present`, `all_present: true`; prune removed 2.3.1.

## UAT cerrados con evidencia OBSERVED

**CTX-UAT-002** (un lease activo, inferencia sin `--cycle`): dos ciclos en ledger aislado, solo
uno nace con lease (`cycle start --lease-owner`). El bootstrap sin `--cycle` devuelve
`state: resolved` apuntando al que tiene la lease, `context_source: compiled`, `status: complete`,
y **no menciona** el ciclo sin lease. La asimetría lease/no-lease es el discriminante real: si el
runtime contara ciclos en vez de leases, devolvería `ambiguous`.

**CTX-UAT-003** (dos leases): segunda lease vía `lock acquire` → `state: ambiguous` con 2
`candidates` que traen `owner` + `expires_at_ms`, y **sin `cycle_id`**: el runtime declara la
ambigüedad en lugar de elegir.

**Recovery:** `lock release` con su `--fencing-token` devuelve la inferencia a `resolved` — la
ambigüedad es estado de runtime, no un fallo permanente.

**CTX-UAT-004** re-verificada: 4/4 PASS con el contrato exit 0/4 corregido.

## Falsabilidad (RED→GREEN observado)

Con el guard de ambigüedad neutralizado en `cycle.rs` (`SDDK_UAT_FORCE_GUESS=1`, revertido tras
la prueba con `git checkout --`): el UAT 004 **falla con 4 aserciones y exit 1**. Revertido, PASS.
Sin ese falsador el PASS sería decorativo.

## Contratos descubritos al escribir el UAT

- `cycle start` deriva el `cycle_id` del nombre (`CycleId::from_parts`) y **no** acepta `--cycle`;
  ofrece `--lease-owner` para crear el ciclo con lease en una sola operación.
- `--timestamp` es **RFC 3339**, no epoch: un entero da `a character literal was not valid`.
- `lock release` exige `--fencing-token`, que se lee de la salida del acquire (nunca hardcodeado).
- `context bootstrap` sale 4 con `no_capsule_source`; 0 solo cuando hay capsule reconstruida.

## Verificación

- `shellcheck -S style` limpio en ambos scripts.
- `python3 tests/test_workflow_contract.py`: **508/508**.
- YAML de `ci.yml` parsea con los 10 steps esperados.
- `tests/test_dev_install_source_guard.sh`: coherente (bundle 2.3.2 = workspace 2.3.2).
- `cargo fmt --all -- --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: 0 diagnósticos.
- `cargo test --workspace --locked`: **exit 0 — 268 suites, 5180 passed / 0 failed / 19 ignored**
  (log íntegro capturado; el primer intento con `grep` dio un total parcial de 30 suites y se
  descartó para no consignar un número inventado). Nota: la primera invocación murió por el
  timeout del wrapper (600 s), no por un fallo de test; repetida con margen, completa.
- `tests/test_release_state_pointer.sh`, `test_release_pipeline_consistency.sh`,
  `test_dev_install_source_guard.sh`: PASS.
- El ledger del operador **nunca** se toca: sandbox con `HOME`/`SDDK_STATE_HOME`/`XDG_DATA_HOME` propios.

## Incidentes propios (registrados, no escondidos)

- Tipos contaminados en la primera versión del script (`cycle_id싱`, `假设`, `递增`) y un conteo
  de aserciones inventado ("12/12" siendo 14). Corregidos **antes** de commitear.
- Dos `edit` fallaron por `old_string` no exacto (incluido un `SDDBASH` typo mío); el
  comportamiento fail-loud evitó escrituras parciales.
- **Un PASS FALSO detectado y descartado:** el primer falsador no compilaba (el bloque devuelve
  `ResolvedCycleContext`, no `Option`), y el UAT dio PASS contra el binario viejo. Se descartó ese
  resultado; el falsador válido es el guard neutralizable por variable de entorno.
- El primer `git commit` de la sesión falló con "Another git process..." sin lock file presente;
  el reintento salió. Sin efecto en el árbol.

## NOT_RUN / sigue abierto

- **CTX-UAT-005** (skill resume contra runtime 0/1/N) y **MIG-UAT-001** (migración skill vieja vs
  nueva): automatizables con el patrón ya establecido.
- INC-AUDIT-S14-*, INC-DEBT-039 (low/P3), INC-MATRIX-LINT: sin cambios.
- C3j: no abierto (C3i ya VERIFIED → se desbloquea).
- 6 advisory `surface.briefness.*` del doctor: sin cambios.

## Siguiente paso

Dos cosas, en este orden y por razones distintas:

1. **Pushear `441c8d46`** (commit solo-comentarios con la nota del orden
   `push → HEAD==origin/main → tag` en `githooks/pre-push`). El pre-push lo rechaza porque
   `githooks/` no está en la allowlist documental y el bump 2.3.2 ya está en `origin/main`.
   **No** se fuerza con `--no-verify` (el gate tiene razón sobre su pregunta) ni se inventa un
   2.3.3 por un comentario. Sale con el próximo bump real.
2. **CTX-UAT-005 y MIG-UAT-001** (las dos UAT de C3i que siguen abiertas), automatizables con el
   patrón ya establecido. C3i está VERIFIED con 003/004 cerrada; cerrarlas lo deja sin huecos y
   desbloquea C3j.
