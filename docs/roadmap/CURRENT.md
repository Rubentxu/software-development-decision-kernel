# CURRENT — puntero de reanudación de SDDK

**Estado (session-34, 2026-09-29T14:05Z):** Workspace **`2.2.25`**, `HEAD == origin/main == bd52f99b`, árbol limpio. Último release verificado bit a bit: **`v2.2.24`** (tag `2fb5f738`, run `36577888371` success completo, 27 assets, `isDraft=false`, `isPrerelease=false`) — cosign Verified OK ×2 (identidad `release.yml@refs/tags/v2.2.24`), digest instalado **idéntico** al publicado (`6c84b702…`), `install.sh` → `all_present: true`, `current -> framework/2.2.24`. **Defecto crítico de `dev update` descubierto y corregido**: al actualizar a v2.2.23 el swap destructivo del path legacy borró `2.2.21/`, `2.2.22/` y `current`; causa raíz doble (tar sin BUNDLE.toml + `copy_tree(Always)` sobre la raíz), fix doble `a409fe45` (merge en legacy, RED→GREEN observado) + `101f1b45` (el job del workflow también empaqueta BUNDLE.toml ahora). **Fire test superado**: el mismo escenario que ayer destruyó el layout conservó todo con el binario 2.2.24. Suite shell **22/22 ×2**. `dev::` rust 409/409. Pendiente: verificar v2.2.25 (run 36579731401) al terminar; sus releases ya llevan BUNDLE.toml en el tar standalone. Recibo: `docs/roadmap/receipts/c3/a409fe45/DEV-UPDATE-LEGACY-MERGE-FIX.md`. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**.

**Siguiente acción exacta:** elegir en `docs/roadmap/ROADMAP.md` el siguiente WorkItem: anclar C0 (T01/T02) al SHA actual, o abrir el primer slice de C3 (Authority interleavings o contención SQLite).

Previous: **Estado (session-33b, 2026-09-29T12:20Z):** Workspace **`2.2.20`** Último tag público: **`v2.2.20`** (`46608302ce99181c8d2fba4d382b54fe8be1ced8`, publicado `2026-09-29T12:11:52Z`, 27 assets, `isDraft=false`, `isPrerelease=false`, run `36565787996` 13/13 jobs, sha == HEAD) — **VERIFICADO**: cosign Verified OK (binario y bundle, identidad `release.yml@refs/tags/v2.2.20`), digest del binario instalado **idéntico** al publicado (`fc23bad5…`), instalación real `all_present: true`, `current -> framework/2.2.20`, `sddk dev update` 377 ficheros content-verified. **Suite completa de contratos shell: 22/22 en dos rondas** — primera vez observada con cero rojos. El último rojo (`test_vault_coherence_alignment.sh`) se resolvió de forma honesta: falso positivo del original reproducido con fixture (aprobaba informes sin veredicto), test reescrito con `evaluate_report()` pinada contra 6 fixtures herméticos, e informe real del trigger producido por el agente `sddk-coherence` con veredicto **`n/a`** (session-33 cerró por ruta estándar de release, no por la ruta vault; informe en `.sddk-cycle-artifacts/coherence/release-archive-vault-complete.md`). Defecto nuevo documentado: mezclar `sddk dev update` (binario 2.2.19, extrae en la raíz) con `install.sh` v2.2.20 (directorios por versión) rompe 69 enlaces; la reinstalación con `install.sh` lo restaura. Nota transitoria: `test_vault_mirror_auto.sh` falló 3 veces dentro del bucle de suite en una ventana post-install, 10/10 aislado y 2 rondas de suite limpias después; no reproducible fuera de esa ventana. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**.

**Siguiente acción exacta:** continuar el roadmap desde `docs/roadmap/ROADMAP.md` con el siguiente WorkItem READY. Si se toca la ruta de release: `git fetch --tags origin` antes de bumpear, y publicar solo vía `gh workflow run release.yml --ref <tag>`.

Previous: **Estado (session-33, 2026-09-29T11:48Z):** Workspace **`2.2.19`**, `HEAD == origin/main == 1cf5d535`, árbol limpio. Último tag público: **`v2.2.19`** (`1cf5d5352373a95ae4b4fd642cd42d5ad5338b97`, publicado `2026-09-29T11:44:52Z`, 27 assets, `isDraft=false`, `isPrerelease=false`) — **CERTIFICADO por seis vías independientes**: workflow `36562863987` 13/13 jobs en success con `sha` == HEAD; 11/11 URLs de distribución HTTP 200; `cosign verify-blob` **Verified OK** en binario y bundle con identidad `release.yml@refs/tags/v2.2.19`; digest del binario instalado **idéntico** al asset publicado (`4f5ec5b5…`); instalación real `all_present: true` con `current -> framework/2.2.19` y 0 enlaces rotos; `sddk dev update` con **377 ficheros** content-verified via `MANIFEST.sha256`. Gates: `cargo fmt --check` OK, clippy `-D warnings` OK, `cargo test --workspace` **0 fallos**, suite shell de contratos **21/22**. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**.

**HALLAZGO session-33 — el conocimiento negativo de session-32 queda EXPLICADO.** Session-32 escribió que la mutación "siempre `true`" no ponía los pins en rojo y que "el mecanismo por el que el pin no detectaría ese mutante sigue sin explicar". **La causa: el pin existente prueba la FUNCIÓN del detector, no el SITIO donde se la invoca.** El detector `tarball_wraps_all_members_under_one_dir` sigue siendo correcto bajo el mutante, así que el pin — que lo llama directamente — sigue verde. La forma del defecto es *"función correcta detrás de un `if` equivocado"*: se buscó un fallo en la lógica y la lógica estaba bien. Confirmado por ejecución. De ahí nace `tests/test_release_bundle_layout.sh`, que sí lo detecta porque parsea el bloque del push y exige que su condición nombre el flag que el detector asigna.

**DECISIÓN session-33 — el layout raíz del bundle es el contrato canónico** (cierra `INC-DEBT-034`). Lo fuerza `release.yml`, que extrae sin `--strip-components` y exige `framework/MANIFEST.sha256`; `AGENTS.md §8` está actualizado en consecuencia. El consumidor Rust además tolera las dos formas: detecta el wrapper en el listing del tarball y solo entonces aplica el strip. `tests/test_release_bundle_layout.sh` cruza los cuatro consumidores del contrato (productor, consumidor CI, consumidor Rust, documentación).

**HALLAZGO session-33 — una discrepancia de digest que es FALSA.** `~/.local/share/sddk/bin/sddk` es un binario **viejo de la v1.145.1 (2026-09-09)** que sigue en disco. Comprobar el digest del release vigente por esa ruta da un valor distinto del publicado y parece una distribución rota. El binario vigente es `~/.local/bin/sddk`, cuyo digest **sí coincide bit a bit**. Anotado en `STATE.yaml` para que no se relea como fallo.

**HALLAZGO session-33 — publicar desde la workstation es imposible por diseño, y está bien.** `scripts/release.sh` aborta en el paso 8c: la firma keyless exige que el OIDC provider emita un certificado para la identidad del proyecto (dentro de Actions), mientras que en local emitiría uno para la persona que abre el navegador — que los instaladores rechazan. La vía correcta es `gh workflow run release.yml --ref <tag>`.

**Notas de proceso (valen para cualquier sesión futura):** (1) `release-bump.sh` se apoya en el **último tag local**, que puede ir atrasado respecto al remoto: hacer `git fetch --tags origin` antes de calcular el bump; (2) el hook pre-push **rechaza** un rango que toque `crates/` o `tests/` sin bump real de `[workspace.package] version` — el asunto `chore(release): bump version` **no es autoridad**; (3) al actualizar una versión ya instalada, verificar con `install.sh`: la primera pasada puede escribir enlaces hacia un directorio de versión que aún no existe si el estado viene de la versión anterior.

**Deuda abierta:** `test_vault_coherence_alignment.sh` en rojo — **preexistente, no causado por esta sesión** (el test no se toca desde el import inicial `34d68c21`); exige un artefacto en `.sddk-cycle-artifacts/coherence/<trigger>.md` que genera el agente de coherencia, que no se ejecutó aquí. **NO se fabricó.** Además: `INC-DEBT-035` (decisión de seguridad del operador), `INC-DEBT-031`, `INC-DEBT-026`, `DEFAULT-GATE-DISCONNECTED`, `NO-STRUCTURED-LOGGING`, `TEST-PORTS-UNCONSUMED`, `RELEASE-FORCE-VERSION-ERGONOMICS`, y la asimetría del instalador con `SDDK_PREFIX` distinto del real.

**Siguiente acción exacta:** resolver `test_vault_coherence_alignment.sh` (ejecutar el agente de coherencia, o decidir explícitamente si el test degrada a WARN cuando el artefacto externo no existe), y continuar el roadmap desde `docs/roadmap/ROADMAP.md` con el siguiente WorkItem READY. Si se toca la ruta de release, empezar por `git fetch --tags origin`.

Previous: **Estado (session-32, 2026-09-28T23:05Z):** Workspace **`2.2.18`**, `origin/main` = **`58bea01f`** (= local, árbol limpio). Último tag público observado: **`v2.2.17`** (`447211e2`, publicado `2026-09-28T22:36:15Z`) — **NO certificado**: su smoke falló en el paso de actualización. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**. Estado local en esta sesión: 31/31 tests de `dev::update`, fmt limpio, clippy sin hallazgos en el fichero tocado.

**HALLAZGO session-32 — la ruta de instalación tenía 4 defectos encadenados, uno por release.** Ninguno se detectó en local: cada uno solo aparece al ejecutar el instalador contra un release real. La secuencia observada es la evidencia:

| tag | run | qué reveló |
|-----|-----|-----------|
| v2.2.15 | `36488888188` | smoke step 1 (**install**) pasó por 1ª vez; step 2 murió con 127 (binario en `$PREFIX/sddk`, vive en `$PREFIX/bin/sddk`) |
| v2.2.16 | `36490012790` | step 2 llegó a cosign: `accepts 1 arg(s), received 3` |
| v2.2.17 | `36492642837` | cosign verificó OK; `bundle is missing required MANIFEST.sha256` |

**HALLAZGO session-32 — el argv de cosign nunca funcionó en ningún release publicado.** `dev update` emitía `verify-blob --bundle <BLOB> --certificate-identity-regexp=… --certificate-oidc-issuer=…`: el blob ocupaba el hueco de `--bundle` y los flags de pinning quedaban como posicionales. cosign 2.4.3 rechazaba **antes de evaluar ningún pin**. El comentario del código daba por bueno un argv jamás ejecutado. Corregido en `5e2ff10d` con `cosign_argv()` + 3 pins de forma. Mutación falsada en dirección fallida (reinsertar el defecto → ROJO).

**HALLAZGO session-32 — productor y consumidor discrepan del layout del bundle (ABIERTO).** CI construye el bundle sin directorio envolvente (`release.yml:107`), `AGENTS.md` §8 paso 5 documenta la forma envuelta. El consumidor aplicaba `--strip-components=1` a ciegas: **borraba `MANIFEST.sha256`** y aplanaba `agents/*.md`. Corregido en `2f7d5064` detectando el layout antes de aplicar el strip (5 pins). **Pendiente la decisión de política:** alinear el productor al contrato, o fijar el layout raíz como contrato y corregir AGENTS.md + `release.sh`.

**⚠️ Conocimiento negativo obligatorio:** la evidencia de mutación del fix de layout (`2f7d5064`) **NO es concluyente y así consta en el propio commit**. El mutante "siempre `true`" no puso los pins en rojo pese a `cargo clean` y recompilación forzada; la lógica se validó aparte con un binario `rustc` autónomo (`root_level -> false`, `wrapped -> true`). **Nadie debe citar `2f7d5064` como mutación falsada en ambos sentidos.** El mecanismo por el que el pin no detectaría ese mutante sigue sin explicar y es el primer punto a investigar si la cadena vuelve a fallar.

**Deuda abierta:** `INC-DEBT-034` (layout productor vs consumidor), `INC-DEBT-036` (cadena de 4 defectos — cerrada en código, cierre formal pendiente), `INC-DEBT-035` (decisión de seguridad del operador), `INC-DEBT-032` (suite no hermética), `INC-DEBT-031`, `INC-DEBT-030` (refutado), `INC-DEBT-026`, `DEFAULT-GATE-DISCONNECTED`, `NO-STRUCTURED-LOGGING`, `TEST-PORTS-UNCONSUMED`, `RELEASE-FORCE-VERSION-ERGONOMICS`.

**Siguiente acción exacta:** `gh workflow run release-automation.yml --ref main` para publicar **v2.2.18** (fix de layout ya en `origin/main` = `58bea01f`). Si el smoke pasa el paso de actualización, se cierra la cadena de 4 defectos; después, verificación post-publicación (URLs 200, `cosign verify`, instalación e2e) y cierre formal de `INC-DEBT-036`. Al diagnosticar, usar **`gh run view <id> --log-failed`**: el log completo mezcla jobs y epistoló tres veces antes de llegar al mensaje real.

Previous: **Estado (session-31, 2026-09-28T16:41Z):** Workspace **`2.2.6`**, **no publicado**. Último tag público **`v2.0.1`**. `origin/main` = `ed0e3c47`; local = `b0cff81e` + bump. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**. Suite completa **5077 passed / 0 failed / 19 ignored** (exit 0) en el árbol de session-31.

**HALLAZGO session-31 — el blocker de firma NO era un blocker.** `INC-DEBT-030` daba por agotados los minutos de Actions (§2.5) y ofrecía solo dos salidas, ambas con decisión del operador. **Refutado por ejecución**: el repo es público, Actions está habilitado, y `gh workflow run ci.yml` arrancó el run `36450601924` real. La vía 1 (publicar desde Actions) es viable y es la correcta, porque produce el issuer `token.actions.githubusercontent.com` que los instaladores pinan. La vía 2 (`SDDK_SKIP_SIGNING=1`) sigue descartada: degrada el contrato de instalación para todos los usuarios.

**HALLAZGO session-31 — el mismo bug de `manifest_sha256` seguía vivo en el CI, donde se publica.** Session-30 corrigió `release.sh`; `.github/workflows/release.yml:199` conservaba `awk 'NR==1 {print $1}'`, o sea el digest del primer fichero listado en vez del manifest. Falsificado con el binario release real y `dev install` en las dos direcciones: el bundle que el workflow publicaba era **rechazado por su propio instalador** (*"Refusing to install a bundle whose declared manifest hash does not match"*). Y `dev manifest --verify` no lo detecta: `verify_manifest_anchor` solo se invoca desde el path de instalación (`install.rs:121`). Corregido en `eae22737` con dos tests: uno que extrae el patrón real del workflow (falsificado por mutación en ambos sentidos) y otro que prueba la consecuencia con `dev install` real.

**HALLAZGO session-31 — regresión de la sesión anterior, acotada por bisect.** `17d9b804` (guard de CHANGELOG) referenciaba `"$CHANGELOG.md"`, variable que `release-bump.sh` nunca define: con `set -euo pipefail` el bump abortaba. Bisect: `ed0e3c47`/`cf481d11`/`e6998f01` verdes, `17d9b804` FAILED, aislado también. Los tests que "cubrían" el cambio eran verdes porque **copian el bloque** del script con un nombre de variable que sí existe. Corregido en `a91c273f`.

**HALLAZGO session-31 — la suite no es hermética, y eso lo demo el CI.** Local verde y remoto rojo sobre el mismo código, con **tests distintos**: en CI fallan `cli_incidence_dka_orphan_review_phase_exists` y `cli_incidence_dka_managed_closure_vault_route_exists`, que exigen ficheros de `~/.sddk-knowledge/` con `env!("HOME")` (tiempo de compilación). No están versionados y no existen en un runner limpio. Registrado en `INC-DEBT-032` con alcance medido (2 tests). No corregido aquí: elegir entre versionar la INC, `#[ignore]` con motivo, o declarar el vault como input externo es política de gobernanza (`AGENTS.md` §2.7), no corrección técnica.

**Deuda abierta real: 8 INCs.** `INC-DEBT-030` (blocker **refutado**, acción = publicar por Actions), `INC-DEBT-032` (suite no-hermetica, nuevo), `INC-DEBT-031` (CHANGELOG duplicado, P3), `INC-DEBT-026`, `DEFAULT-GATE-DISCONNECTED`, `NO-STRUCTURED-LOGGING`, `TEST-PORTS-UNCONSUMED`, `RELEASE-FORCE-VERSION-ERGONOMICS`.

**Siguiente acción exacta:** publicar **v2.2.6**. Tres pasos: (1) `git push origin main` — el bump a 2.2.6 satisface la cláusula (A) del pre-push, que exige cambio real de `[workspace.package] version`; (2) `gh workflow run release-automation.yml`, que crea el tag desde `origin/main:manifest.toml` y despacha `release.yml` (donde el binario musl y `cosign` keyless ya están resueltos); (3) verificar los assets públicos e instalar end-to-end. **Irreversible** (`git.release=human_gate`), ya pre-aprobado por el operador en session-31. Antes de cerrar: reconciliar `STATE.yaml` (el guard ya no miente, pero afirma un SHA sin publicar — se resuelve publicando).

Previous: **Estado (session-28, 2026-09-28T10:10Z):** Workspace **`2.2.0`**, **no publicado**. Último tag público **`v2.0.1`**. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`** (session-26 dijo "no adoptado": era falso).

**HALLAZGO session-28 — el bump de CI habría REGRESADO la versión del workspace.** `release-bump.sh` derivaba la siguiente versión siempre del **último tag**, así que un bump manual era invisible: con el workspace en 2.1.1 y el tag en v2.0.1 derivaba **v2.1.0**, una regresión. Y no era hipotético: el paso "Open release PR when a bump is pending" de `release-automation.yml` corre ese script y hace `gh pr merge --auto --squash`, o sea que el rollback se habría mergeado solo sobre main. **Reproducido antes de arreglar** en un clon aislado (2.1.1 → 2.1.0). Corregido derivando desde la más alta entre workspace y tag; se eliminó además la segunda lectura de `WORKSPACE_VERSION` para que el ancla del `sed` y la base de la derivación no puedan divergir. Cobertura nueva: `tests/test_release_bump_derivation.sh` (6 casos) — no existía ninguna para este script. Contra la lógica original da 4 pass / **2 fail** (los 2 rollbacks); contra el fix, **6/0**.

**HALLAZGO session-27 — la admisión de release rechazaba un release válido.** `release.sh` rutaba a **v1** (HEAD vs HEAD^) en vez de **v2** (último tag publicado). Como el bump vive N commits atrás de HEAD, HEAD y HEAD^ carrying la misma versión y el gate rechazaba un release correcto. El defecto ya estaba documentado en `release_admission.sh:15-19` y la v2 existía, probada con 22 checks, **nunca seleccionada**. Corregido en `a716953`; 3 checks de **conexión** añadidos (mutación a v1 → 2 fallan, `die`→`warn` → 1).

**Corrección session-27: el bloqueo musl NO bloquea la publicación.** `release-automation.yml` crea el tag desde `origin/main:manifest.toml` y despacha `release.yml`, que instala `musl-tools` con su propio sudo (líneas 53-57). La publicación es ejecutable por CI sin tocar esta máquina. Solo la ruta local `release.sh` necesita musl en el host.

**Deuda abierta real: 7 INCs.** `SUPPLY-CHAIN-AUTHENTICITY` (code-closed/distribution-open), `DEFAULT-GATE-DISCONNECTED` (nuevo, session-27), `NO-STRUCTURED-LOGGING` (medium), `TEST-PORTS-UNCONSUMED` (medium), tres `low`.

**Siguiente acción exacta:** publicar `2.2.0` es lo único que cierra `INC-AUDIT-S14` en distribución. **Irreversible** y lo decide el operador: `gh workflow run release-automation.yml`. Antes conviene confirmar que la derivación da `v2.2.0` (ya verificado) y que el gate de admisión acepta. `ADR-0143` sigue `proposed`.

Previous: **Estado (session-27, 2026-09-28T09:40Z):** `HEAD == origin/main == 9a642e7` (`chore(release): bump version a 2.1.1`). Workspace **`2.1.1`**, **no publicado**. Último tag público **`v2.0.1`**. **`sddk adopt status` = `complete`** (session-26 dijo "no adoptado": era falso).

**Bump `2.1.0` → `2.1.1`** (`9a642e7`) producido por `scripts/release-bump.sh --force-version`, no a mano. **2.1.0 nunca fue tag ni release**, así que publicar 2.1.1 no salta ninguna versión pública.

**Puntero de estado reconciliado** (`241ada6`): el guard daba FAIL por 4 commits de retraso (tolerancia 3) — tercera repetición del patrón session-18/22. Reconciliación mecánica que preserva la nota de evidencia previa.

**Deuda abierta real: 6 INCs**, no uno. `SUPPLY-CHAIN-AUTHENTICITY` (code-closed/distribution-open), `NO-STRUCTURED-LOGGING` (medium), `TEST-PORTS-UNCONSUMED` (medium), tres `low`. La nota de session-25 ("el único INC abierto") era de alcance más estrecho.

**Siguiente acción exacta:** leer el dry-run hasta el paso 8. Si pasa, **publicar 2.1.1** es el único camino que cierra `INC-AUDIT-S14` en distribución. Irreversible: requiere firma keyless desde Actions (`gh workflow run release-automation.yml`). `ADR-0143` sigue `proposed`.

Previous: **Estado (session-25 cierre, 2026-09-28T09:00Z):** `HEAD == origin/main == 3a142b8` (`chore(release): bump version a 2.1.0`). Workspace **`2.1.0`**, **no publicado**; `--dry-run` con árbol limpio dio **release admission ACCEPT 2.0.12 → 2.1.0** con 0 `✗`. Último tag público sigue **`v2.0.1` → `5ce4bca`**. Cierre de session-25: **`INC-DEBT-024` cerrado** (las tres mitigaciones, verificadas en ambos sentidos) y la deuda menor de la versión de cosign. Previous: session-24 (`d1973b1`) implementó el gate de issuer fail-closed.

**`INC-DEBT-024` — CERRADO en session-25.** Era el único INC abierto (20 de 21 cerrados). Cerrado por las tres vías, cada una con su falsificación:

1. **Gate de issuer fail-closed** (`08639ff`, session-24): `release.sh` lee el issuer del certificado que cosign acaba de acuñar (del bundle) y hace `die` si no es `DEFAULT_CERT_ISSUER`, si falta el certificado, o si no se puede leer. El issuer sale de la **URI SAN**, no del DN RFC4514 (el DN lleva el nombre de la CA Fulcio, no el proveedor OIDC).
2. **Pre-check de contexto** (`ef0d5a6`, session-25): sin `GITHUB_ACTIONS=true`, aborta **antes** de firmar. No es un mensaje más limpio: el gate de issuer corre *después* de que cosign firme, y para entonces ya han pasado el device flow interactivo (cuelgue en run desatendido) y, si el operador lo completa, un certificado de persona. El pre-check evita las dos. `SDDK_SKIP_SIGNING=1` sigue siendo la salida declarada para publicar sin firma; `SDDK_ALLOW_LOCAL_SIGNING=1` existe solo para falsificar el check y **no** es vía a un release bueno.
3. **`ADR-0143 §(a)` reescrita** (`f9beddf`): la opción 1 es "firmar antes de publicar **desde Actions**". Añadido un `§(a-bis)` que separa el código (ya no puede publicar con la identidad equivocada) de la política (quién ejecuta el release, y elegir entre las dos opciones).

**HALLAZGO que reencuadra las dos sesiones anteriores (session-25):** `release.yml` **no invoca `release.sh`** — firma en su propio job `sign`, con `id-token: write` y `cosign sign-blob --output-signature --output-certificate`. Por tanto el pre-check protege la **ruta local** (donde estaba el fallo, y era el no documentado) **sin tocar CI**, y el gate de issuer cubre `release.sh`, **no** la firma de producción. La identidad de la firma de producción ya la garantiza el diseño del workflow; lo que session-24/25 cerraban era el agujero local.

**Deuda menor también cerrada** (`314bc34`): `cosign-release: 'v2.4.3'` explícito en los dos pasos `cosign-installer`. El action estaba pineado por SHA, lo que da falsa sensación de control: lo que quedaba por defecto era la **versión**, que pertenece a un tercero. Un check de contrato exige el pin en todos esos pasos.

**Falsificación permanente (7 checks en `test_install_asset_contract.sh`).** Cada uno probado con su mutación: cambiar el pin del issuer → 2 fallan; `die`→`warn` en el gate de issuer → 1; eliminar el pre-check → 3; `die`→`warn` en el pre-check → 1; **mover el pre-check después del bucle de firma → 1 (el de orden)**; quitar el pin de cosign del job `sign` → 1, nombrando el job. El de orden importa: un pre-check correcto en la posición equivocada no evitaría el device flow.

**Verificación de session-25:** `cargo test --workspace --locked --no-fail-fast` = **5057 passed / 0 failed / 19 ignored**, 258 suites, exit 0 (idéntico al baseline, así que el bump no alteró nada). `fmt`, `clippy -D warnings`, `shellcheck -S error`, `diff --check`, `cargo metadata --locked`, YAML del workflow: limpios. Suite de shell + public gate: **12/12 PASS**.

**Lo que queda abierto, y NO es técnico.** `INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY` está `code-closed, distribution-open`: cerrarlo exige **publicar un release firmado real**. La vía existe y está verificada (`gh workflow run release-automation.yml` crea el tag y despacha `release.yml`), pero crea un tag y un release público irreversibles, así que no se ejecutó. `ADR-0143` sigue `proposed`. Ambas salidas son del operador.

**Estado (session-24 cierre, 2026-09-28T08:11Z):** `HEAD == origin/main == 2f0482e` (`chore(release): bump version a 2.0.12`). Workspace **`2.0.12`**, **no publicado**. Último tag público sigue siendo **`v2.0.1` → `5ce4bca`**. Cierre de session-24: implementado el **identity gate de firma** (`08639ff`), que era la mitigación 1 de `INC-DEBT-024` y que session-23 dejó declarada y sin hacer. Previous: session-23 (`a0729b0`) escribió el INC y corrigió `ADR-0143` para exigir que la firma se obtenga desde Actions.

**Identity gate de firma — CERRADO en `08639ff`:** `release.sh` comprobaba la firma con `command -v cosign`, o sea con la presencia del binario, no con la identidad realmente emitida. Eso era justo el agujero: la identidad keyless depende de **dónde** firmas, y cosign está instalado igual en los dos sitios. Resultado posible: un release correctamente firmado que el propio `install.sh` de este repo rechaza, con un error que en el usuario se lee como manipulación. Ahora `release.sh` extrae el issuer del certificado que cosign acaba de acuñar (del bundle: `.cert` en v2, `verificationMaterial.x509CertificateChain` en el formato nuevo) y **hace `die` antes de publicar** si no es el esperado. El issuer se lee de la **URI SAN**, no del DN RFC4514 — el DN lleva el nombre de la CA Fulcio, no el proveedor OIDC. **Fail-closed en los tres casos**: issuer distinto, bundle sin certificado, bundle ilegible. Un control que no puede leer lo que verifica no ha verificado nada. Falsificado en ambos sentidos con 3 checks permanentes en `test_install_asset_contract.sh` (suite de shell del paso 1: **11/11 PASS**).

**CORRECCIÓN session-24 — hipótesis mía que era FALSA y que no debe quedar escrita:** afirmé que la firma en CI fallaría por incompatibilidad de versión de cosign, deducido del `cosign v3.1.3` instalado en esta máquina. **Falso, por dos errores.** (1) El comentario `cosign-installer@053f9b74 # v3.8.1` es la versión del **action**, no la de cosign; ese SHA **sí** existe y resuelve a `refs/tags/v3.8.1` del propio `cosign-installer` (confirmado con `git ls-remote`). (2) Su `cosign-release` por defecto es **`v2.4.3`** y el workflow **no lo overridea** — CI instala v2.4.3, no v3.1.3. En v2.4.3 `--output-signature` y `--output-certificate` **sí** existen (`sign_blob.go`, `SignBlobCmd(..., outputSignature, outputCertificate, ...)`). **El workflow de firma no estaba roto.** La incompatibilidad real es entre la v3 del host y la v2 de CI, y afecta a quien intente reproducir la firma en local, no a la publicación. Todo esto está escrito en el INC, no en un handoff.

**Deuda menor que abre session-24 (sin registrar como INC aparte):** la versión de cosign que instala CI es un **default implícito** del action. Si sigstore cambia ese default, la versión usada por la firma cambia sin que nada en este repo lo refleje. Fijar `cosign-release: 'v2.4.3'` explícito en el workflow lo convierte en decisión declarada y visible. **No se ha tocado el workflow**: es superficie de publicación y pide su propio slice con su propio bump.

**Lo que sigue abierto es una decisión de política, no de ingeniería** (`ADR-0143 §(a)`, status `proposed`): si la firma se obtiene **antes** de publicar (paso 8c, ya escrito) o se mantiene la rama `sign` del workflow con un guard de publicación. Con el gate de session-24, la opción 1 ya no puede publicar por error. La elige el operador.

**Estado (session-22 cierre, 2026-09-28T07:45Z):** C1 cerrado; C2 NOT_EVALUATED (bloqueada por `chronos-mcp`; `cognicode-mcp` presente); C3a-h PASS_OBSERVED; C4 — último release publicado sigue siendo **`v2.0.1` → `5ce4bca`**. Workspace **`2.0.10`**: 9 PATCH por delante del tag público, sin publicar. Dos cierres en esta sesión: el bloqueo del `Cargo.lock` stale (que rompía CI y release con `--locked`) quedó cerrado en `8136bbf`, y el flake `R-flake-inv10` del gate de concurrencia quedó cerrado en `982014e`. Previous: session-21 (206584b, a030eed, c9984b8) corrigió el pin de identidad cosign a `refs/tags/vX.Y.Z` — insatisfecible con el pin previo `@refs/heads/main` — y alineó el manejo del trio `.sig`/`.bundle.json`/`.pem` en los tres consumidores. Este puntero se revalida al comienzo de cada sesión con `bash tests/test_release_state_pointer.sh`.

**FLAKE `R-flake-inv10` — CERRADO en `982014e`:** el gate `inv10_grep_gate_no_mutex_on_workflow_state` afirmaba `elapsed < 20ms` con hijos triviales. Es un umbral absoluto sin margen: media la máquina y no el código. Reproducido en **4 de 6 corridas bajo carga** (22-31ms) mientras pasaba en 0.00ms en aislamiento. Subirlo a 200ms lo volvía verde pero **dejaba pasar la serialización completa** (`max_concurrency=1`, ~60ms), o sea que habría dejado de detectar la regresión que existe para detectar — se descartó. El wall-clock no separa los dos casos en una máquina desconocida: contención con hijos triviales (~30ms) y serialización genuina (~60ms) se solapan. Lo que sí separa es el **ratio** entre el elapsed total y el coste de una corrida serializada, que es adimensional. Falsificado en ambos sentidos: concurrente bajo `nproc` procesos en burn da **8/8 y 6/6**; `max_concurrency=1` **falla con ratio 100%**; `max_concurrency=2` **falla con ratio 50%**. Los hijos duermen 20ms porque con 4ms una máquina cargada tardaba 42ms solo en agendar 50 duermes concurrentes (21% del serial) — midiendo latencia de despertar, no contención del mutex.

**CORRECCIÓN session-23 — mi registro estaba equivocado, y esto lo corrige:** he estado repitiendo que "el trust root de la firma sigue sin definirse" como el bloqueo abierto. **Es falso.** Session-21 ya lo decidió e implementó: Fulcio keyless, issuer `https://token.actions.githubusercontent.com`, subject regex anclado a `release.yml` sobre un tag SemVer, con ambos extremos obligatorios y fail-closed en las dos rutas de consumo. La decisión existía; lo que faltaba era el documento que la gobernara. Eso es `ADR-0143` (status `proposed`), escrita en esta sesión: fija la política, sus consecuencias asumidas, y delimita con precisión lo que sigue abierto.

**Lo que de verdad queda abierto es una decisión de política, no de ingeniería** (`ADR-0143 §(a)`): si la firma se obtiene **antes** de publicar (paso 8c de `release.sh`, antes de `gh release create`, que ya está escrito) o se mantiene la rama `sign` del workflow con un guard de publicación. Hoy, si `sign` falla a mitad, el tag queda publicado sin assets firmados, es el `latest`, y es **ininstalable** por ser fail-closed. Ambas opciones tienen un coste real y la elige el operador.

**Deuda de proceso, cerrada:** el patrón de puntero desfasado que llevaba tres sesiones (session-18, 22, 23) ya no requiere intervención manual. `scripts/reconcile_state_pointer.sh` (`56eac21`) reconcilia los campos mecánicos respetando la misma tolerancia que el guard, y conserva las notas de evidencia en lugar de pisarlas. El fallo original —el bump viaja en un commit posterior al que mueve el puntero— sigue siendo real, pero ya no requiere que nadie lo recuerde.

**RECONCILIATION session-22 (deuda de proceso propia, segunda vez):** al abrir sesión, `STATE.yaml` declaraba `current_sha=aaed465` / `2.0.7` mientras el repo estaba en `62d4728` / `2.0.8` — **5 commits de deriva** (`1de1caa`, `206584b`, `a030eed`, `c9984b8`, `62d4728`) acumulados en session-21, que sí actualizó el journal pero saltó el puntero de autoridad. Corregido en el mismo ciclo; el guard pasó a `PASS (0 commit(s) de retraso)`. Patrón: session-18 y session-22 repiten exactamente la misma falla. Merece un guard que escriba el puntero, no solo que lo detecte.

**BLOQUEANTE session-22 — CERRADO en `8136bbf`:** `Cargo.lock` estaba en 2.0.7 mientras `Cargo.toml` estaba en 2.0.8 (el commit `62d4728` bumpeó solo el primero). Rompía `ci.yml:36` y `release.yml:63`, que construyen con `--locked` — un checkout limpio fallaba con **exit 101**, y publicar moría en el paso 63 antes de la firma. Cerrado bumpeando `2.0.8 → 2.0.9` con `Cargo.toml`, `manifest.toml` y `Cargo.lock` en el mismo rango (cláusula (A) del hook `pre-push`, **sin tocar la allowlist**). `cargo metadata --locked` pasó de 101 a 0. Además, `tests/test_release_state_pointer.sh` tiene ahora un check 6 que compara `Cargo.lock` contra `Cargo.toml`, falsificado en ambos sentidos: es lo que habría detectado `62d4728`.

Previous: **C4 v1.172.0 — publicado y formalizado** (release v1.172.0 en GH Releases con FC-1 v2 + chromium-skip fix; tag apuntado a `d89c2c0`; `CERTIFICATION-RECEIPT.yaml` schema §5 compliant).

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@0ccecae` (session-18) — **pusheado y verificado contra `origin/main`**. Último release **publicado**: tag `v2.0.1` → `5ce4bcacfb8ec4855cd6c543b59f6bda940b7872`. Rango de session-18: `971e0ea..0ccecae`. Reconciliación de puntero aplicada: `STATE.yaml` decía `5ce4bca`/`2.0.1` con 12 commits de retraso; corregido a `e001e39`/`2.0.5` sin reescribir historia, con `certification_claim` (citaba 1.171.2/v1.172.0, 3 majors obsoleto) y `next_action` (decía PAUSE desde session-13) también reconciliados. La fotografía de session-15 abajo se conserva como historia, sin alterar. |
| Workspace en esa fotografía (session-15) | `2.0.1` (Cargo.toml + manifest.toml, commit `5ce4bca`). Último tag SemVer publicado = `v2.0.1` — **coinciden por primera vez** (la serie `1.17x` del workspace había quedado por debajo del tag `v2.0.0` y era rechazada por la admisión v2 con `not-above-last-publish`; alineado en `5ce4bca`). **Session-18: workspace `2.0.5`**, ya no alineado — 4 PATCH por delante del tag. |
| Release pública comprobada en esa fotografía | `v2.0.1` (2026-09-27T19:26:59Z) — GH Releases Latest. Assets: `sddk`, `sddk-v2.0.1-...-musl.tar.gz`, `sddk.sha256`, `bundle.tar.gz(.sha256)`, `CHECKSUMS`, `sbom.json`, `gh-release-receipt.json`. Digest `sddk` publicado por API == sha256sum local `417a7163a286f75b…` (verificado independientemente del script). |
| Hito activo | C1 CERRADO; C2 NOT_EVALUATED (systemic, provider MCP bridges ausentes); C3a-h PASS_OBSERVED; **C4 v1.172.0 publicado + cert formalizado** (`docs/roadmap/receipts/c4-release-v1.172.0/CERTIFICATION-RECEIPT.yaml` schema §5 compliant). **FC-1..FC-8 todos IMPLEMENTED o DEFERRED-DUPLICATED** (session-13 close: FC-4 implementado como shell orchestrator). |
| Estado PRs abiertos | Ninguno. |
| Commits since v1.171.0 (workspace ahead → release v1.172.0) | `0974292 fix(test)` · `e49ff38 chore(release): bump 1.171.0 -> 1.171.1` · `3d24000 feat(cli): extend sddk uat batch with selective filters` · `3c93472 test(cli): add filter predicate coverage for uat batch` · `8a541c3 chore(release): bump 1.171.1 -> 1.171.2` · `550ef84 Revert "chore(release): bump 1.171.1 -> 1.171.2"` · `17f938b docs(roadmap): reconcile session-12 closeout at 1.171.2` · `e9cf84c Reapply "chore(release): bump 1.171.1 -> 1.171.2"` · `d89c2c0 chore(workspace): regenerate Cargo.lock at 1.171.2` · tag `v1.172.0` published · `00e7b56 docs(roadmap): reconcile release v1.172.0 publication` · `96da6db docs(roadmap): v1.172.0 CERTIFICATION-RECEIPT + UAT-EVIDENCE (T29/T31)` · `1a6f7ef docs(roadmap): state sync — v1.172.0 cert formalized (HEAD 96da6db)` · `2321efe docs(debt): formalize legacy 'body **status**: closed' to frontmatter` · `d5823bc docs(roadmap): enrich v1.172.0 cert with flake root-cause analysis` · `fdfe6fb feat(operations): FC-4 docs/operations/uat-replay.sh — pinned-release replay` · `72825fe docs(roadmap): mark FC-4 as IMPLEMENTED in FEATURE-CANDIDATES`. |
| Working tree uncommitted | Vacío. |
| Tests verified this session | Perfil COMPLETO sobre el árbol publicado como `v2.0.1`: **`cargo test --workspace --offline --no-fail-fast` = 5041 passed; 0 failed; 19 ignored (exit 0)**. `cargo clippy --workspace --all-targets -- -D warnings` → clean (0 diagnostics). `cargo fmt --check` → clean. El `release.sh` volvió a correr el perfil completo en step 1 (sin `--skip-tests`). El fix de traversal verificado **en el código publicado** vía `git show v2.0.1:crates/sddk-cli/src/dev/update.rs` (10× `ensure_safe_tarball_members`, 1× `no-same-owner`). |
| Real-provider binary availability | `cognicode` CLI v0.97.3 presente; `cognicode-mcp` PRESENTE (v0.97.3). `chronos-mcp` AUSENTE (no instalable). `jcode` v0.86.0 presente; `jcode-sdk` no publicado. **C2 sigue NOT_EVALUATED** — por `chronos-mcp` ausente. |
| Siguiente acción exacta | Todo el trabajo propio que quedaba está hecho. Las tres salidas restantes son del operador, y todas tocan política o algo irreversible: **(1) Publicar** (elegir entre firmar antes, opción 1 de `ADR-0143 §(a)`, o mantener la rama `sign` con un guard de publicación; publicar `2.0.11` con firma keyless cerraría `INC-AUDIT-S14` en distribución). **(2) Ratificar `ADR-0143`** y decidir sus huecos (b), (c) y (d). **(3) Decisión de C5** (change-scoped verification da su primer consumidor real al SPI de `test_ports.rs`, SPEC-043 §4). La entrada que requiere input externo sigue igual: **Chronos publica `chronos-mcp`** (sin binario, C2 no es ejecutable). Todo lo demás es mantenimiento P2/P3 ya registrado. |
| Evidencia requerida para mover puntero | Para C4 PROMOTION a `CERTIFIED_BASE`: ejecutar C2 con providers reales (cognicode-mcp + chronos-mcp + jcode-sdk instalados), re-run T01-T35 contra el SHA de release, ejecutar `tests/clean_machine_uat.sh --tag v1.172.0` en podman. |
| Bloqueos y decisiones | **C2 sigue cerrado honesto NOT_EVALUATED** (receipts c2a/c2b/c2c). **C3a-h cerrado PASS_OBSERVED**. **Release v1.172.0 publicado + cert formalizado** (commit `96da6db`). **Certificación v1.172.0: PASS_PARTIAL_OBSERVED** (formal: `CERTIFICATION-RECEIPT.yaml`). **Override SemVer LIFTED en v1.171.0 sigue aplicable retroactivamente**; v1.172.0 es SemVer-correct minor (1 feat detectado) — algoritmo canónico coincidió con override. **FC-3 RECHAZADO por duplicación con `sddk ledger export --cycle`**. **FC-1 v2 IMPLEMENTED** (extensión de `UatBatchArgs`, no nuevo subcomando). J7/J8/J9/X08/R11 siguen DEFERRED. **Riesgo honesto documentado**: v1.172.0 fue publicado con `--skip-tests`; el full profile re-ejecutado post-publish pasó 5048/5048, así que la evidencia durable es sólida, pero el contrato literal del release script (correr full profile inline) NO se cumplió para v1.172.0 — registrado como `R-flaw-concurrency-planning-substrate-flake` en cert. |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.

## Estado de certificación (al cierre de session-12 / 2026-09-22T20:58Z)

- **C1 (Base)**: cerrada con full profile 4998/0/15 sobre e7968f8; certificados H02/H05+H06/cycle-c.
- **C2 (Integraciones reales)**: **NOT_EVALUATED_PROVIDER_MISSING (C2a, C2b) / NOT_EVALUATED_ADAPTER_MISSING (C2c)**. Cierre honesto documentado en `docs/roadmap/receipts/c2a/`, `c2b/`, `c2c/`. Status systemic; recovery requires operator decision.
- **C3a-h**: TODOS **PASS_OBSERVED** — Authority hardening (C3a), Storage adversarial (C3b-c), Performance baseline (C3d), Schema resilience (C3e), Migration re-application safety (C3f, closes C3e-F1 via ADR-0141), Performance budget harness (C3g), Supply-chain audit + remediation (C3h, 0 vulns).
- **C4 (Release y certificación de producto)**: **v1.172.0 publicado y formalizado** (2026-09-22T20:23:49Z). `docs/roadmap/receipts/c4-release-v1.172.0/CERTIFICATION-RECEIPT.yaml` schema §5 compliant creado en commit `96da6db` (status: PASS_PARTIAL_OBSERVED, 4 PASS_OBSERVED gates + 12 HISTORICAL_CARRY_OVER + 1 NOT_VERIFIED; T29 + T31 con 6 falsifiers cada uno, 0 triggered). 9 commits ahead of v1.171.0 tag. FC-1 v2 + chromium-skip fix incluidos. Pipeline completo (0-13) PASS vía `bash scripts/release.sh --skip-tests` con admisión v2. Binary sha256 verificado. Instalación local en `~/.local/share/sddk/framework/1.171.2/` OK. Poda de bundles viejos (`1.171.0`) ejecutada. Doctor check: `binary.bundle_coherence: present, all_present: true`. Distrib round-trip OK. **Full profile re-ejecutado post-publish = 5048 passed; 0 failed; 19 ignored** (flake `concurrency_planning_substrate` no triggered en re-run). Promotion a CERTIFIED_BASE blocked por C2 NOT_EVALUATED. Riesgo honesto documentado en cert: el contrato literal de correr full profile inline en release.sh NO se cumplió (--skip-tests usado), aunque el full profile post-publish sí pasó.
- **C5 (Evolución condicionada)**: pendientes P2/P3 (J7/J8/J9/X08/R11), ninguno activo.
- **FC-1 v2 IMPLEMENTED**: `sddk uat batch` extendido con `--scenario`, `--flag`, `--priority`, `--exclude-flaky` (commits `3d24000` + `3c93472`). 7 nuevos tests predicate. Sin duplicación de código: helpers puros (`batch_filter_matches`, `uat_priority_label`) reutilizables.
- **Chromium-skip fix**: test `stale_detects_geometry_change` ahora hace skip limpio cuando chromium no está instalado (commit `0974292`), no panic.
- **Session-12 audit findings** (operador: "CLOSED != CERTIFIED"):
  - RECEIPT.md original (session-11) era un release receipt, NO un CERTIFICATION-RECEIPT (schema §5). Reemplazado por `CERTIFICATION-RECEIPT.yaml` schema-compliant.
  - 40 INCs declaradas `status: closed` fueron auditadas; todas tienen evidencia de cierre (commit refs, test names, exit codes, matrix case counts). NO hay cierres paperwork.
  - FC-3 working tree RECHAZADO: duplica `sddk ledger export --cycle X --output <file>`. La extensión propuesta es extender `LedgerExportArgs` con `--format text|json|jsonl` y `--output opcional` (FUTURO).
  - FC-5 working tree NO implementado pero marcado DEFERRED-DUPLICATED con `sddk fork diff` y `sddk memory diff`.
  - **v1.172.0 cert formal**: PASS_PARTIAL_OBSERVED con T29+T31 PASS_OBSERVED (0 falsifiers triggered de 12 totales). Acepta flake pre-existente `concurrency_planning_substrate` como R-flaw (workspace-wide concurrency, no en isolation). 5048 tests passed post-publish = evidencia durable sólida.

**Distinción importante**: workspace v1.171.2 == binary-version 1.171.2 (alineado). Tag SemVer publicado = v1.172.0 (computed desde commits acumulados). El binario instalado en `~/.local/bin/sddk` corresponde a v1.172.0 (sha256 generado por pipeline). Próximo release: continuar con FC-* restantes.

## Estado real al cierre de session-29 (2026-09-28) — PENDIENTE DE DECISION DEL OPERADOR

> Esta seccion es mas nueva que la de arriba. El "Estado de certificacion"
> de arriba refleja session-12 (v1.172.0) y **ya no describe el estado real**.

### Git

- `HEAD` local = `99bc6526`, arbol limpio, **9 commits sin push**.
- `origin/main` = `a048ddf1`.
- Workspace `2.2.0` (declarado en `a9da3104`, ya en `origin/main`).
- Ultimo tag publicado = `v2.0.1`. **v2.2.0 nunca se publico** (no existe
  tag ni release), asi que el numero 2.2.0 solo existio como commit.

### Lo que hay sin publicar (9 commits)

Tres defectos reales de la ruta de release, corregidos y con mutaciones que
los detectan:

1. `7559a710` — `release-bump.sh` re-bumpeaba cuando el workspace ya declara
   la release. Con 2.2.0 y tag v2.0.1 derivaba v2.3.0, y
   `release-automation.yml` habria auto-mergeado un PR 2.2.0 -> 2.3.0.
   Matriz 7/7, 4 mutaciones.
2. `df55db4c` — cobertura del contrato de `release.sh` paso 2.5, que es la
   ruta que el fix anterior atraviesa. 3 checks conductuales + 2 estaticos,
   4 mutaciones (M4, M6, M7, M8).
3. `46666a3c` — `BUNDLE.toml` declaraba `manifest_sha256` con el hash de la
   **primera linea** del manifest, no con el del manifest. El campo tampoco
   se verificaba en ningun sitio. Mutacion M10.

Ademas: `993def7b` (journal), `5324a528`, `48561746`, `d33e5f40`,
`99bc6526` (documentales), y dos INCs nuevos:

- `INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE` (high/P1, code-fixed)
- `INC-DEBT-026-BUNDLE-CONTENT-NOT-IN-HISTORY` (medium/P2, open) — el bundle
  local mezcla `skills/` de `gentle-ai/sdd` (`~/.config/kilo/`, declarado
  `__managed_by: gentle-ai/sdd`). El asset publicado verifica 377/377, asi
  que no hay incidente de distribucion. Queda abierto determinar si el
  bootstrap de ese proyecto escribe en el bundle de este: seria una
  intrusion y romperia la regla de cero intrusion.

### Evidencia (OBSERVED, sobre el arbol actual)

- `cargo fmt --check` OK, `cargo clippy --workspace --all-targets -D warnings`
  OK, `cargo test --workspace` = **5057 passed / 0 failed / 19 ignored**,
  exit 0.
- Tests shell: 15 PASS / 2 FAIL. Los 2 fallos son preexistentes
  (`test_vault_coherence_alignment` falla tambien sobre HEAD sin mis
  cambios; `test_release_state_pointer` solo dice "aun no publicado").
- shellcheck limpio en los 4 ficheros tocados.
- 12 mutaciones nombradas, cada una falla el gate que la posee.

### EL BLOQUEO (decision del operador)

`githooks/pre-push` rechaza el push: `scripts/**` y `tests/**` no estan en la
allowlist docs-only y el rango no tiene cambio real de
`[workspace.package] version`. El hook no tiene bypass.

Precedente del propio repo para este caso exacto: `a7169537` (fix de codigo
en `scripts/release.sh` + `tests/`) se publico junto a un bump real,
`3a142b86` a 2.1.0. Mismo patron, misma resolucion.

**Correccion de session-29**: se recomendio 2.2.1 alegando que 2.2.0 se
publicaria con el doble bump. Es falso — los 3 fixes estan en el arbol 2.2.0
(8 commits de trabajo tras `a9da3104`, 4 ficheros, 308 inserciones). El
numero 2.2.0 es correcto y nunca se publico.

Dos salidas, ambas legítimas:

- **A)** bump real a **2.2.1** en commit propio, siguiendo el precedente
  `3a142b86`. El hook acepta y el push sale. Se salta un 2.2.0 que nunca
  existio como release.
- **B)** excepcion de hook para publicar **2.2.0** con los fixes dentro.

La eleccion entre A y B cambia el numero publico y la politica del hook, no
la calidad del codigo. **La decision es del operador y no se toma aqui.**

### Siguiente paso ejecutable

1. Operador elige A o B.
2. `git push origin main` (con bump propio si A).
3. `bash scripts/release.sh` (14 pasos) para publicar 2.2.0 o 2.2.1.
4. Post-publish: `sddk dev install` y `sddk dev doctor` (que hoy reporta
   `content.manifest: missing` por el bundle mezclado de INC-DEBT-026).
