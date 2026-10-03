# RECEIPT — REL-2.5.5

Cycle: `p-63676b11dc0ef88f/c3n-production-boundary-certification`
Path: A-full · phase en el ledger: `design` · status: `OPEN`
Release: `v2.5.5` · commit del tag: `886e47cc0e2f4cffcc59054f28e44667ab844f08`
Publicado: `2026-10-03T22:56:09Z` · draft `false` · prerelease `false`
URL: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v2.5.5

**Este es el primer release de este repo que corre el pipeline `0`..`13` completo con
`EXIT=0`.** Las dos releases anteriores (`v2.5.3`, `v2.5.4`) se publicaron con el
pipeline a medias: la `2.5.3` murio en el 9c y la `2.5.4` en el 10.

---

## 1. Resultado por paso, transcrito sin reinterpretar

La cuenta pertenece a la corrida que la produjo (`/tmp/rel255d.log`), no a un
documento que la menciona.

| # | Paso | Resultado |
|---|---|---|
| 0 | preflight | `ACCEPT last-publish=2.5.4 -> 2.5.5` · aviso no fatal de subject |
| 1 | fmt + clippy + test workspace | verde |
| 1b | shell contract tests | 33 shell + 10 python verdes |
| 1c | sync HEAD to origin/main | ya sincronizado, sin push |
| 1d | EXT auto-activacion | sin vars EXT · tests `#[ignore]` |
| 2 | read version | `2.5.5` |
| 2b | changelog coverage | verde (primera vez que pasa en ruta) |
| 3 | build release | ELF 64-bit static-pie, musl, `sddk 2.5.5` |
| 3b | reconciliacion del artefacto | `PASS=16 FAIL=0` |
| 3c | autofalsacion de 3b | `PASS=10 FAIL=0` |
| 3d | estados de `binary.build_identity` | `PASS=16 FAIL=0` (O6 `NOT_RUN` declarado) |
| 3e | receipt de frontera exigible | `PASS=14 FAIL=0` |
| 3f | politica de nombres | `PASS=9 FAIL=0` |
| 3g | autofalsacion de 3f | `PASS=8 FAIL=0` |
| 3h | citas de spec ancladas | autofalsacion `PASS=6 FAIL=0 SKIP=0` |
| 4 | manifest | regenerado, 395 ficheros |
| 5 | bundle tarball | 679476 B · staging identico al manifiesto |
| 6 | `BUNDLE.toml` | dentro del tarball, `version=2.5.5` |
| 7 | unified tarball | 12595281 B · exec bit OK |
| 8 | sha256 + CHECKSUMS + sbom | binario `bb753428b56998f…` |
| 8b | vault ADR mirror | 60 de 64 aceptados, `created: 0` (idempotente) |
| 8c | cosign | **`SDDK_SKIP_SIGNING=1` — publicado SIN FIRMAR** |
| 9 | `gh release create` | publicado |
| 9b | public-release gate | tag SHA anclado · 9/9 assets HTTP 200 en CDN publico |
| 9c | autenticidad | **postura `UNSIGNED` · declarado `NOT_RUN` con su motivo** |
| 10 | install desde URL real | CDN sirvio el sha correcto tras 10 s · instalo |
| 11 | `dev doctor` | `binary.bundle_coherence: present`, `all_present: true` |
| 12 | `dev update --prune-only --keep 1` | bundles stale podados |
| 13 | re-install desde URL | round-trip OK, binario y bundle coherentes |

`EXIT=0`.

## 2. Estado local verificado despues de publicar

Medido sobre la instalacion, no heredado del log:

| Comprobacion | Valor |
|---|---|
| `sddk --version` | `sddk 2.5.5` |
| `sha256` del binario instalado | `bb753428b56998f75310f77ed964456c5ef7f156673034706f43563434cbaebc` |
| symlink `framework/current` | -> `/home/rubentxu/.local/share/sddk/framework/2.5.5` |
| versions presentes tras el prune | solo `2.5.5` |
| `BUNDLE.toml` instalado | `schema_version=2`, rango cerrado `2.5.5`..`2.5.5` |
| `dev manifest --verify` | `manifest OK` (0 mismatches) |
| `dev doctor` | `all_present: true`, `binary.build_identity: present` |
| ficheros en el bundle instalado | 397 = 395 del manifiesto + `MANIFEST.sha256` + `BUNDLE.toml` |

El `sha256` del binario local **coincide con el que el paso 8 declaro y con el que
el paso 10 recibio del CDN**: el artefacto que se publico, el que se descargo y el
que se esta ejecutando son el mismo fichero.

## 3. Lo que esta release NO demuestra

Se declara porque un cierre que solo lista lo verde es una certificacion que no
se sabe que mide.

1. **Autenticidad NO verificada.** Sin firma (`SDDK_SKIP_SIGNING=1`) y con **cero**
   ficheros de firma presentes. Los dos instaladores exigiran `SDDK_ALLOW_UNSIGNED=1`.
   El 9c lo declaro `NOT_RUN` con su motivo, y el cierre del 9b lo **califica**
   en vez de declarar PASS a secas. La release esta publicada y es
   instalable, pero **nadie puede afirmar de donde viene**.

2. **El ancla de confianza sigue siendo el placeholder**
   `@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@`, en los dos sitios que tienen que
   coincidir (`assets/trust/release-verify-key.pub` y `SDDK_RELEASE_VERIFY_KEY_BODY`
   en `scripts/install.sh:235`). `test_install_asset_contract.sh` pasa **porque
   comprueba que coincidan, no que sean reales**. **No se fabricara una clave ni
   un ancla**: eso es fabricar una credencial.

3. **3h reporta 7 citas de spec ambiguas de un unico ID.** La autofalsacion pasa
   (`PASS=6`), luego el gate no esta roto, pero la ambiguedad sigue ahi. Es
   `SPEC-012` sin autoridad declarada.

4. **La firma sigue siendo una decision de tres vias, todas externas** (KMS /
   fichero local / `SDDK_SKIP_SIGNING`). La via usada aqui es la tercera, y es la
   unica que no necesita nada fuera del repo. Es una decision *deliberada*, no un
   No es un olvido: instala con opt-out explicito, nunca en silencio.

## 4. Discrepancia abierta que este receipt deja declarada

`docs/roadmap/CURRENT.md` (session-68) declaraba para este ciclo:

> Artefactos: `exploration-report.md`, `specification.md`, `closeout.md`

**Esos tres ficheros no existen.** El directorio
`tests/cycle-artifacts/p-63676b11dc0ef88f/c3n-production-boundary-certification/`
no existia hasta que este receipt lo creo. Medido con `glob` sobre
`tests/cycle-artifacts/**/*boundary*`: sin coincidencias.

Es la misma clase de defecto que este ciclo (`c3n-production-boundary-
certification`) existe para
arrancar: **una autoridad que declara algo que no esta**. Un puntero que afirma
artefactos inexistentes entrena a la siguiente sesion a buscarlos, y cuando no los
encuentra tiene dos lecturas posibles — "se perdieron" o "nunca se escribieron" —
y la distincion es exactamente lo que el receipt no puede restaurar.

**No se fabrican.** Los tres artefactos de design no se reconstruyen a posteriori
desde memoria: un `exploration-report.md` escrito tres sesiones despues de la
exploracion seria una narracion, no una evidencia. Lo que si es restaurable y se
restaura aqui es **lo medido**: la release. Si el ciclo necesita esos artefactos
para cerrarse, hay que reabrir la fase que los produce.

Nota de gobernanza: el ledger tiene este ciclo en `OPEN`/`design` y **sin lease
viva** — `cycle artifacts-dir` responde `no active cycle found for project`, que
es como se presenta una lease caducada, no un ciclo inexistente. Escribir el
receipt **a disco** no requiere lease (es un fichero del repo, camino
`tests/`, que es donde este proyecto escribe sus artefactos); transicionar el
ciclo en el ledger si la requiere.

## 5. Defectos que este tramo reparo, y donde

| Defecto | Commit | Guard |
|---|---|---|
| 8c resolvia el binario donde no vivia | `5e310011` | `test_release_sign_artifacts.sh` (12) + mutacion (7) |
| 9c exigia firma que la via sin firmar no produce | `dc8a5f8f` | `test_release_authenticity_posture.sh` (8) + mutacion (8) |
| `release-bump.sh` no movia `BUNDLE.toml` | `faddec03` | `test_release_bump_bundle_sync.sh` (5) |
| paso 10 redecidia la postura y hardcodeaba el prefix | `0962a79d` | `test_release_unsigned_propagation.sh` (6) |
| instalador con tres `local` sin fuente | `0f416de2` | `test_install_signature_execution.sh` (19) + mutacion (9) |
| flake de concurrencia en `process_service()` | `304c5d2a` | el propio `cargo test --workspace` del paso 1 |
| `cargo fmt` roto por el arreglo anterior | `89b470c3` | el propio `cargo fmt` del paso 1 |

Los **dos ultimos** no anadieron guard porque **el gate de produccion ya los
cogia**. Anadir un guard que duplica un gate que ya corre crea un sitio mas
donde dejar de mirar; la evidencia la daba el release, y en los dos casos llego
tarde porque no se ejecuto el gate que ya existia.
