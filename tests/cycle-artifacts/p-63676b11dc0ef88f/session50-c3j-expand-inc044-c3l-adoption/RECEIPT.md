# RECEIPT — session-50: C3j objetivo 4 (`context expand`) + INC-DEBT-044 + reenfoque de roadmap (C3l/C3m/C3n)

- **Fecha (UTC):** 2026-09-30T21:29Z → 2026-09-30T22:10Z
- **Baseline:** `origin/main = 6fbe1990` (peel de `v2.3.3`), árbol limpio al inicio
- **Proyecto SDDK:** `p-995939af668a53d8`
- **Hitos:** C3j objetivo 4 (en vuelo) + reenfoque de roadmap a la vía acceptance-truthfulness (`docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/`, adoptada por el operador en sesión: **C3l P0 paralelo, no reemplaza C3j**)
- **Binario de contraste:** release publicado `v2.3.3` (pre-feature) y build local `/var/home/rubentxu/cargo-targets/release/sddk`
- **Ledger del operador:** **intacto** (todos los escenarios en sandboxes `mktemp` con XDG/HOME propios)

---

## 1. Derivación del WorkItem (no supuesto)

Deuda abierta re-verificada: solo los 5 INC-AUDIT-S14 (medium/low, sin criterio de severa reciente); guard índice↔documento PASS. Regresiones: perfil 5182/0/19 en el SHA publicado. El hueco real de C3j: objetivos 1/2/3/5 implementados y verificados; **objetivo 4 (`context expand`) inexistente** — CTX-UAT-014/015 dependen de él. `uat_ctx_003` ya cubre 008–011 (no duplicado).

## 2. Trabajo realizado

1. **`sddk context expand` (feat(cli), `1ae2f6bf`)** — progressive disclosure mínima: sesión→binding→capsule→**ledger**; contenido real para `work-item:<id>`, decisión pelada, `cycle:<id>`; read log durable por sesión (`context/reads/<session>.json`, cap 100, sha256 del contenido); fail-closed tipado en todas las direcciones (sin binding, sin ciclo, ref desconocida con lista de disponibles, ref stale).
2. **INC-DEBT-044 (high/P1, resolved; `cb4ea598`)** — destapada por el UAT en su primera corrida: `FilesystemCapsuleStore::file_name_for` incrustaba el id de ciclo **con barra** en el nombre de fichero → `fs::write` a subdirectorio inexistente → `persist` tragaba el fallo → **para TODO ciclo real el bootstrap decía `compiled` con el directorio capsules VACÍO** (la basis apuntaba a una capsule inexistente; CTX-UAT-006 era imposible con ids reales). Nadie lo vio: **todos los tests de capsule usaban ids sin barra** — la misma lección de INC-DEBT-043 (un fixture que no usa la forma real del dato no distingue éxito de ficción). Fix: percent-encode (`%``/``:`) en `file_name_for`; lookups codificado-a-codificado; sin migración (los nombres rotos jamás llegaron a disco).
3. **Reenfoque de roadmap (directiva del operador)** — insertados **C3l (P0) / C3m / C3n** en `docs/roadmap/ROADMAP.md` con la regla de promoción obligatoria, y alta de **AT-UAT-001..026** en la matriz (NOT_RUN, con columna Boundary).

## 3. Evidencia observada

| Qué | Resultado |
|---|---|
| RED del store (unit) | `capsule_persists_and_recovers_with_slashed_runtime_run` FAILED antes del fix |
| Store tras fix | `durable_capsule_store` 5/5 · `sddk-engine --lib` **1352/0** |
| Unit expand | 5 nuevos; `context_cmd::tests` **30/0** |
| RED pre-feature (UAT) | contra `v2.3.3` publicado: exit 1 (el comando no existía) |
| UAT CTX-UAT-015 post-fix | **27 ok / 0 FAIL / exit 0**, 9/9 secciones |
| Falsador unitario | 2/2 RED (prosa de capsule en vez de ledger + read log suprimido); los 3 fail-closed siguen verdes (correcto) |
| Falsador UAT | **3 FAIL exit 1** contra binario mutado — exactamente los 2 asserts de contenido-ledger y el de read log |
| fmt / clippy | limpios (`-p sddk-engine`, `-p sddk-cli --all-targets -D warnings`) |
| Perfil completo | `cargo test --workspace`: consignado al cerrar (ver journal) |

## 4. Incidente de método propio (declarado)

**Pérdida y reconstrucción de la implementación por falsar sobre código sin commitear.** El primer intento de mutación no compilaba (referenciaba un binding fuera de scope); al revertir con `git checkout`, se deshizo **también toda la implementación de expand**, que aún no estaba commiteada. Reconstruida íntegramente desde el contexto de la sesión y verificada (30/0). **Lección operativa registrada: commitear el estado verde ANTES de aplicar mutaciones de falsificación** — la reversión de un falsador debe ser un `git checkout` que solo toque código desechable. Es la tercera entrada de la familia «la medición/operación puede destruir lo que dice medir» (con el PASS contra binario viejo de session-48/49 y el exit secuestrado por el trap).

## 5. Lo que este recibo NO declara

- No hay release en el momento de escribirlo: se publica tras el perfil completo; SemVer derivado (**feat → MINOR → 2.4.0**).
- CTX-UAT-014 queda **parcial**: la mitad observable (no-dump en el envelope) PASS; el presupuesto explícito de tokens no existe aún.
- `CapsuleStore::persist` sigue fire-and-forget por firma de trait (límite residual de INC-DEBT-044): un fallo de escritura por permisos/disco seguiría sin reportarse hasta tocar el contrato del trait.
- CTX-UAT-007..012: 008–011 cubiertos por `uat_ctx_003` a nivel de comportamiento, pero la correspondencia fila-a-fila sigue pendiente de confirmación del operador (nota previa de la matriz se mantiene).
- C3l.0 (re-clasificación del baseline) **no se ha abierto todavía** — es el siguiente WorkItem tras publicar.

## 6. Commits de este bloque

- `cb4ea598` fix(engine): capsules de ciclos reales (INC-DEBT-044)
- `1ae2f6bf` feat(cli): `sddk context expand` (C3j objetivo 4)
- _(docs/close: se consigna tras el perfil completo y el release)_

## 7. Artefactos

- `crates/sddk-engine/src/durable_capsule_store.rs` — encode + test RED-first
- `crates/sddk-cli/src/context_cmd.rs` + `crates/sddk-cli/src/lib.rs` — `context expand` + dispatch
- `tests/uat_ctx_007_context_expand.sh` — CTX-UAT-015
- `docs/debt/INC-DEBT-044-CAPSULE-PERSIST-SILENT-FAIL-SLASHED-CYCLE-ID.md`
- `docs/roadmap/ROADMAP.md` / `UAT-MATRIX.md` — C3l/C3m/C3n + AT-UAT + 014/015
