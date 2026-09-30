# RECEIPT — session-49: cierre de CTX-UAT-005 y MIG-UAT-001 (C3i) + INC-DEBT-043

- **Fecha (UTC):** 2026-09-30T18:55Z → 2026-09-30T19:05Z
- **Baseline:** `origin/main = 4952e88c` (peel de `v2.3.2`), `HEAD` local al inicio `13f39450`
- **Proyecto SDDK:** `p-995939af668a53d8` · workspace `w-92344bcb2666782c579618c4`
- **Ciclo/milestone:** C3i — Coherencia de bootstrap/adoption/cycle recovery
- **WorkItem:** W1 — cerrar las dos últimas UAT de C3i
- **Binario de contraste:** release publicado `v2.3.2` (`~/.local/bin/sddk`, 2.3.2) y build local `/var/home/rubentxu/cargo-targets/release/sddk`
- **Ledger del operador:** **intacto**. Todos los escenarios corrieron en sandboxes aislados (`SDDK_STATE_HOME`/`XDG_DATA_HOME`/`XDG_CACHE_HOME`/`HOME` bajo `mktemp -d`); nunca se ejecutó un comando mutante contra el ledger real.

---

## 1. Premisa de partida: dos filas `NOT_RUN` cuya razón era incorrecta

CTX-UAT-005 y MIG-UAT-001 llevaban `NOT_RUN` desde session-36 con la misma
razón: *«requiere dos versiones de skill conviviendo; sin release nuevo»*.
Verificada **antes** de escribir código, la premisa no se sostiene:

1. El enunciado de origen (paquete hypermedia,
   `06-uat/UAT-MATRIX.md:18`) es **«caller legacy pasa cycle ID explícito →
   explicit vence inference»**, no «dos skills conviviendo». Es un contrato
   del **runtime**, ejercitable contra un solo binario y un solo ledger.
2. Ya existe release publicado (v2.3.2), luego el binario real existe.

**Es la segunda vez en dos sesiones que una «puerta»_resulta ser un script que
faltaba** (la primera fue CTX-UAT-002/003 en session-48). Sin ejecutar nada
para comprobarlo, ambas filas habrían seguido figurando como deudaUAT.

## 2. Descubrimiento y corrección: INC-DEBT-043 (high/P1, resolved)

Al validar MIG-UAT-001 se observó que `--cycle` con un id **inexistente**
devolvía:

```json
{"status": "no_capsule_source",
 "cycle": {"state": "explicit", "cycle_id": "<id que no existe>"},
 "basis_revision": "empty", "capsule_id": null,
 "binding_written": true}
EXIT=4
```

`state: explicit` afirma que la referencia resolvió, y `binding_written: true`
**persiste un binding durable a un ciclo inexistente**. La superficie hermana
`sddk cycle status --cycle <desconocido>` sí fallaba cerrado
(`STORAGE_NOT_FOUND`, exit 1) — y ambas están documentadas por la skill en el
**mismo** envelope `cli_context`.

- **Clasificación honesta:** pre-existente desde `aff0a498` (session-40, cuando
  aterrizó la ruta explícita). **No** es una regresión de session-46.
- **Por qué nadie lo vio:** las dos fixtures explícitas usaban ids que nunca
  se insertaban en el ledger, así que no podían distinguir «enlaza una
  referencia» de «enlaza una ficción».
- **Resolución:** `ContextBootstrapError::CycleNotFound` + comprobación
  `Storage::cycle_exists` en la frontera del binding (paso 6 de `bootstrap`),
  después de resolver la basis → exit 1, sin envelope, sin binding. El ciclo
  **resuelto por inferencia** queda exento (el resolver ya lo leyó de una lease
  viva).

## 3. Evidencia observada

| Qué | Comando | Resultado |
|---|---|---|
| RED antes del fix (unit) | `cargo test -p sddk-cli --lib context_cmd::tests::explicit_nonexistent` | **2 FAILED**; payload = envelope fantasma con `binding_written: true` |
| GREEN (unit) | `cargo test -p sddk-cli --lib context_cmd::tests` | **25 passed / 0 failed** (2 nuevos + 2 fixtures reparados + 21 intactos) |
| Falsador unitario | `let exists = true` (variante de error presente) | **2/2 RED** → el check es load-bearing, no la firma del error |
| Detector pre-fix (e2e) | `uat_ctx_005` contra release publicado `v2.3.2` | **8 FAIL / exit 1**, de los cuales **4 exactamente** en la sección fail-closed; las otras 30 aserciones PASS |
| MIG-UAT-001 post-fix | `bash tests/uat_ctx_005_explicit_cycle_migration.sh` | **7 secciones, 34 ok, 0 FAIL, exit 0** |
| CTX-UAT-005 post-fix | `bash tests/uat_ctx_006_skill_runtime_alignment.sh` | **7 secciones, 36 ok, 0 FAIL, exit 0** |
| Falsador e2e | ambas mutaciones (adivinar ambigüedad + `cycle_exists=true`) | `uat_ctx_005` **8 FAIL exit 1**; `uat_ctx_006` **5 FAIL exit 1** (7/7 secciones, log completo) |
| fmt | `cargo fmt --check` | limpio |
| clippy | `cargo clippy -p sddk-cli --all-targets -- -D warnings` | exit 0 |
| shellcheck | `shellcheck tests/uat_ctx_005_*.sh tests/uat_ctx_006_*.sh` | limpio |
| índice de deuda | `bash tests/test_debt_index_coherence.sh` | **PASS=10 FAIL=0** |

### Observaciones que sostienen la corrección (no inferidas)

- **Explícito vence a inferencia, ya antes del fix:** con dos leases
  (`ambiguous`) + `--cycle alpha`, el binario publicado resolvía `alpha` con
  `status: complete` / `context_source: compiled`. El defecto **no** era la
  precedencia; era la falta de negativa ante una referencia rota. Por eso
  MIG-UAT-001 no era PASS antes: su criterio («sin pérdida de contexto») se
  incumplía en la entrada `cycle_id` inválido.
- **Identidad estable:** `project_id` y `workspace_id` idénticos entre el
  caller ambiguo y el explícito sobre el mismo ledger.
- **Sin pérdida de contexto, demostrado por comparación explícita:** la ruta
  ambigua entrega `capsule_id: null`; la explícita entrega capsule compilada
  desde facts reales. La comparación es una aserción del script, no una
  descripción.
- **La escalera de recovery de la skill es real:** desde 0,
  `sddk cycle start` (la acción legal que la skill escribe) devuelve el runtime
  a `resolved`; desde N, elegir **un candidate de la lista que emitió el propio
  runtime** resuelve a ese ciclo. Sin esto, una skill con un recovery inexistente
  pasaría los pines de texto de `test_workflow_contract.py`.

## 4. Errores de medición cometidos en esta sesión (declarados)

Dos, ambos de la familia que este repo ya documenta. Se registran porque un
recibo que solo lista los aciertos no es auditable.

1. **Falsador que no detectaba nada, por binario obsoleto.** La primera
   ejecución del falsador dio PASS en ambos scripts contra el binario mutado.
   La causa fue que **el build de la mutación había fallado** (`E0425`, luego
   `E0308`) y los tests corrieron contra el binario anterior. Se llegó a la
   conclusión opuesta —«mis tests no son load-bearing»— sin comprobar el build.
   Rehecha con mutación compilable: RED 8 FAIL / 5 FAIL, como se esperaba.
2. **Exit code contaminado por la limpieza.** El `trap` de `rm -rf "$SANDBOX"`
   devolvía el exit del `rm` (64), no el veredicto del script. Se habría
   reportado «exit 64» como si fuera el resultado de la UAT. Corregido en ambos
   scripts: el trap preserva el status y restaura `HOME` antes de borrar.

**Hallazgo de robustez derivado del falsador:** `uat_ctx_006` **abortaba a
media corrida** si el runtime mutado no devolvía `candidates` (`KeyError` bajo
`set -e`), ocultando el resto del log. Un UAT que aborta no informa: las
lecturas de JSON se hicieron tolerantes (`get` con default) y la corrida pasó a
reportar las 7 secciones con 5 FAIL.

## 5. Lo que este recibo NO declara

- **No hay PASS de C3j.** CTX-UAT-006..015 y HYP-UAT-001..004 siguen
  `NOT_RUN`/reservadas; C3j no se ha abierto. Solo se desbloqueó como
  dependencia satisfecha.
- **No se ejecutó el perfil completo del workspace** en el momento de escribir
  este recibo (sí `cargo test -p sddk-cli --lib context_cmd::tests`, fmt, clippy
  y shellcheck). El perfil completo se ejecuta antes del commit de integración
  y su resultado se consigna en el journal.
- **No se ha publicado release.** Sin tag, sin assets, sin verificación
  pública. La versión publicada sigue siendo `v2.3.2`.
- **Los dos commits documentales sin pushear de session-48**
  (`441c8d46`, `13f39450`) siguen sin pushear; el pre-push los rechaza porque
  `githooks/` no está en la allowlist documental y el bump ya está en
  `origin/main`. Salen con el próximo bump real, sin `--no-verify`.

## 6. Conocimiento negativo (lo que NO se sabe)

- La ruta explícita accepts un `--cycle` **de otro proyecto**: no se probó si
  `cycle_exists` filtra por proyecto o solo globalmente. Si solo fuera global,
  un id válido de otro proyecto resolvería y cruzaría contextos. Queda como
  pregunta abierta, no como defecto confirmado.
- No se midió el coste de la comprobación (una lectura extra de SQLite por
  bootstrap explícito). Se consideró irrelevante frente a la corrección, pero no
  se midió.

## 7. Commits de este bloque

_(se completan tras la ejecución del perfil completo y el bump)_

## 8. Artefactos

- `tests/uat_ctx_005_explicit_cycle_migration.sh` — MIG-UAT-001
- `tests/uat_ctx_006_skill_runtime_alignment.sh` — CTX-UAT-005
- `docs/debt/INC-DEBT-043-EXPLICIT-CYCLE-BOUND-TO-NONEXISTENT-CYCLE.md`
- `crates/sddk-cli/src/context_cmd.rs` — `CycleNotFound`, check `cycle_exists`,
  `plant_real_cycle`, 2 tests nuevos, 2 fixtures reparados
