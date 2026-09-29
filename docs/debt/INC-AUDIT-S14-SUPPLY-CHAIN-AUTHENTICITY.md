---
id: INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY
title: "El checksum que valida el bundle se descarga del mismo origen que el bundle"
status: closed
severity: high
priority: P1
# session-43: CERRADA. El criterio de cierre era "cerrar en distribucion
# requiere publicar con el workflow firmado y observar que la firma
# verifica". v2.2.27 cumple las dos partes y se verifico con cosign contra
# el pin exacto del producto. Ademas el paso 9c de release.sh comprueba
# esto en cada release futuro, de modo que la condicion ya no depende de
# que alguien recuerde ejecutarla. Ver "Cierre session-43" al final.
resolved_at: 2026-09-29
resolved_by: session-43
resolution_guard: tests/test_supply_chain_authenticity.sh
resolution_release_step: "9c (scripts/release.sh)"
resolution_witness_tag: v2.2.27
created: 2026-09-27
discovered_by: session-14 audit (code-based, no documentado previamente)
cluster_id: CL-SUPPLY-CHAIN
fingerprint: "sddk_bundle_selfattesting_manifest_v1"
---

## Qué es

`crates/sddk-cli/src/dev/update.rs` descarga, del **mismo** origin path
`{base_url}/download/{version}/`:

- el tarball del bundle (L33)
- el `.sha256` que lo valida (L33-45)
- el `MANIFEST.sha256` interno que valida cada fichero (L66-85)

## Evidencia (verificada, lectura de código)

```rust
// update.rs:33-45
download_to(&url, &bundle)?;
download_to(&format!("{url}.sha256"), &checksum)?;
let expected = std::fs::read_to_string(&checksum)?...;
let actual = sha256_hex(&bundle)?;
if expected != actual { bail!("framework sha256 mismatch") }
```

Después, `verify_manifest` (`manifest.rs:92-122`) comprueba que cada
fichero declarado existe y que su hash coincide. Es un control de
**integridad** genuino y bien implementado.

## Por qué importa

Integridad sí, autenticidad no. Un origen comprometido (o un mirror
secuestrado) sirve un payload y su `.sha256` correspondiente: el bundle
valida limpio. La cadena se auto-certifica contra sí misma.

El riesgo es acotado — requiere controlar el origen de descarga — pero es
exactamente el escenario que un checksum existe para evitar.

## Mitigación disponible ya

`cosign` **ya está instalado y usado** en el repo
(`scripts/e2e-install.sh:70-75`, pinneado a `v2.4.3`). Falta aplicar la
misma verificación de firma a la ruta de `sddk dev update`.

## Nota sobre el riesgo adyacente

`update.rs:46` invoca `tar xzf` sin `--no-same-owner` /
`--no-same-permissions` ni validación de rutas de miembro. Un tarball
hostil podría escribir fuera de `staged_bundle` o fijar permisos/ownership
arbitrarios. `verify_manifest` no detecta ficheros *extra*, solo que los
declarados estén bien. Mitigación de bajo coste: los dos flags GNU, o un
lector validante en Rust.

## Opciones

- **(a) Firma out-of-band** con cosign.
- **(b) Allowlist de miembros del tarball** antes de extraer.
- **(c) Ambos.** Son independientes y cada uno es pequeño.

### CORRECCIÓN DE SESIÓN-16: la opción (a) NO es "coste bajo"

Session-14 estimaba (a) como *"coste bajo, reutiliza la infra
existente"*, ycitando que cosign ya se instalaba en
`scripts/e2e-install.sh`. **Esa premisa era falsa y queda anulada.**

Verificado contra el código:

- `scripts/install.sh` **no menciona cosign en ninguna línea**. Sólo
  tiene `verify_sha256`, y el `.sha256` se descarga del mismo
  `$BASE_URL` que el payload (`release_url()` construye ambas URLs
  desde la misma variable).
- `scripts/release.sh` **no firma nada**: no hay `cosign sign`, ni
  generación de clave, ni subida de un asset `.sig`/`.att`.
- El release `v2.0.1` **no publica ningún asset de firma** (los 9
  assets verificados: CHECKSUMS, gh-release-receipt.json, sbom.json,
  sddk, sddk.sha256, el tarball unificado y su .sha256, y el bundle
  tarball con su .sha256).
- `scripts/e2e-install.sh` **instalaba cosign en el contenedor y luego
  exigía una cadena que `install.sh` nunca imprime**:
  `grep -q "signature verified (cosign keyless)"`. Esa variante era
  insatisfacible por construcción y nunca se ejecutó en verde.

Es decir: **no hay infra de firma que reutilizar.** La única cosa que
existía era cosign instalado en un contenedor de test que no verificaba
nada. Cerrar (a) es trabajo desde cero: decisión de trust root,
generación/rotación de clave, firma en el release, y verificación en
`install.sh` y en `crates/sddk-cli/src/dev/update.rs`.

Session-16 corrigió el e2e para que deje de exigir la cadena
inexistente y reporte la ausencia de firma como **warning honesto
vinculado a este INC**, en vez de un fallo que ocultaba todo lo demás.

## Estado

**Parcialmente cerrado en session-14 (b), re-evaluado en session-16.**

- **(b) Cerrado.** Allowlist de miembros del tarball implementada en
  `crates/sddk-cli/src/dev/update.rs` antes de extraer
  (`ensure_safe_tarball_members`, fail-closed) +
  `--no-same-owner`/`--no-same-permissions`. 7 tests nuevos pinean el
  comportamiento, incluido el exploit exacto
  (`software-development-decision-kernel/../../etc/x` →
  `../etc/x` tras `--strip-components=1`). Verificado: 784 passed en
  `sddk-cli --lib`, clippy y fmt limpios.
- **(a) Abierto, y más caro de lo que se decía.** Requiere trust root
  (decisión de diseño: dónde vive la clave, rotación, qué hacer si no
  hay firma) + firma en el release + verificación en las **dos** rutas
  de consumo (`install.sh` y `dev update`). No es un quick win.

Nota: el guard de traversal no mitiga la ausencia de firma. Solo evita
que un origen comprometido convierta la instalación en una primitiva de
escritura arbitraria. La autenticidad del origen sigue sin resolver,
y ahora está confirmado que **ninguna de las dos rutas de instalación
la implementa**.

## Actualización session-21 — (a) cerrado en código, abierto en distribución

**Trust root decidido: Fulcio keyless vía GitHub Actions OIDC.** No hay
clave que conservar ni que rotar: la autoridad es el issuer
`https://token.actions.githubusercontent.com` y la identidad es el subject
del workflow.

### Lo implementado

- `release.sh` firma los 3 artefactos con `cosign sign-blob` y exige
  **todos** firmados (`-ne "${#SIGN_ARTIFACTS[@]}"`, no `-eq 0`).
- `install.sh` y `dev update` rechazan por defecto un artefacto sin
  firma; firma presente e inválida aborta; el opt-out es explícito
  (`--allow-unsigned` / `SDDK_ALLOW_UNSIGNED*`) y anuncia lo que hace.
- Fuente única del pinning en `crates/sddk-cli/src/cosign.rs`, con el
  shell cotejado por `test_install_asset_contract.sh`.

### Cuatro defectos encontrados al contrastar código contra CI

1. **El pin de identidad no matcheaba nada.** Decía
   `@refs/heads/main`, pero `release-automation.yml` despacha
   `gh workflow run release.yml --ref v2.0.7`, así que el subject real
   es `@refs/tags/v2.0.7`. Con el pin enviado, **toda instalación
   habría fallado** con un error de firma indistinguible de un ataque.
2. **Un pin literal a un tag se rompe en el siguiente release.** Fijado
   a `v2.0.7` habría pasado hoy y fallado en 2.0.8: verde hasta el día
   que no. Sustituido por `--certificate-identity-regexp` con el repo y
   el fichero de workflow fijos y el ref como tag SemVer. Confirmado
   contra `cosign verify-blob --help` en la versión fijada.
3. **La firma detached se verificaba sin certificado.** El CI publica
   `.sig` + `.pem`; ambos consumidores caían a
   `verify-blob --signature` sin `--certificate-chain`, donde
   `--certificate-identity` no tiene contra qué casar. El control
   aparentaba ser estricto y era más débil. Ahora una firma detached
   sin `.pem` se rechaza en vez de aceptarse.
4. **El smoke test del CI grepeaba una cadena que el instalador nunca
   imprimía** (`"cosign keyless"` vs `"cosign, sigstore trust root"`).
   El workflow se caía en el smoke test **después de publicar los
   assets**. Alineados, y el grep pasó a `-Fq` sobre la cadena exacta.

### Verificación

- `cosign::tests` (11): el subject real del workflow se acepta; los
  tags futuros se aceptan sin editar el pin; **una rama se rechaza**
  (el valor que se había enviado); otro repo y otro workflow se
  rechazan; el patrón está anclado. Falsificado revirtiendo el pin al
  literal de rama: 3 tests fallan.
- 27 checks en `test_install_asset_contract.sh`, 5 de ellos nuevos,
  falsificados con 7 mutaciones, todas detectadas.
- `cargo test --workspace` sin FAILED; clippy `-D warnings` con 0
  errores; shellcheck limpio.

### Estado real: (a) NO cerrado en distribución

`v2.0.1` **no tiene ningún asset de firma** — la rama `sign` del
workflow nunca se ejecutó. No se ha obtenido ninguna firma real: la
keyless local cae en el device flow de Fulcio y necesita intervención
humana en navegador.

Publicar `v2.0.7` cerraría esto, porque la automatización crea el tag y
despacha el workflow firmado. Antes hay que decidir si se acepta esa
publicación: es una acción irreversible sobre un repo público, y si el
firmado falla a mitad, `v2.0.7` queda publicado sin assets firmados.

---

## Cierre session-43 — distribución verificada sobre un release real

**Status: closed.** Cierra también la parte de *distribución*, que era lo
único que quedaba. El criterio, escrito por session-21 y nunca
satisfecho hasta ahora, era: *"cerrar en distribución requiere publicar
con el workflow firmado y observar que la firma verifica"*.

### Evidencia observada

`v2.2.27` (el release público real más reciente) está firmado por Actions:

```console
$ gh release view v2.2.27 --repo Rubentxu/software-development-decision-kernel \
    --json assets --jq '.assets[].name'
CHECKSUMS
gh-release-receipt.json
sbom.json
sddk
sddk.sig
sddk.pem
sddk-v2.2.27-sddk-linux-x86_64-musl.tar.gz{,.sig,.pem}
sddk-v2.2.27-sddk-linux-aarch64-musl.tar.gz{,.sig,.pem}
sddk-v2.2.27-sddk-darwin-arm64.tar.gz{,.sig,.pem}
sddk-v2.2.27-sddk-darwin-x86_64.tar.gz{,.sig,.pem}
...
```

Verificación con los pins **extraídos del código**
(`crates/sddk-cli/src/cosign.rs`), no escritos a mano:

```console
$ bash tests/test_supply_chain_authenticity.sh --tag v2.2.27
  ✓ pins extracted from cosign.rs (not hardcoded in this script)
  ✓ signature verifies under the pinned trust root: sddk
  ✓ signature verifies under the pinned trust root: software-development-decision-kernel.tar.gz
  ✓ verified 2/2 signed payload(s) for v2.2.27
  ✓ pin rejects a branch ref where a tag is required (control)
  ✓ pin rejects a different OIDC issuer (control)
  ✓ control: a wildcard pattern WOULD accept this certificate (so the pin is what rejects)
  PASS=13 FAIL=0 SKIP=0
```

El subject real que Fulcio minteó, leído del propio certificado:

```
https://github.com/Rubentxu/software-development-decision-kernel/.github/workflows/release.yml@refs/tags/v2.2.27
```

Es repo correcto, workflow correcto y **tag** — exactamente las tres cosas
que el pin restringe.

### Por qué esto no se habría cerrado solo

La razón por la que session-21 no pudo cerrar esto no fue falta de
código: el código estaba todo. Fue que **nada ejecutaba la verificación**.
Los tests existentes comprueban que el pin sea coherente
(`cosign.rs` == `install.sh`) y que case con el subject observado, pero
ninguno verificaba un release real. Sin un comando que produzca la
observación, la deuda solo podía quedar abierta por falta de alguien que
la hiciera, y eso es una deuda de proceso disfrazada de deuda técnica.

De ahí el guard y el paso 9c: la condición pasa a ser **automática y
fail-closed** en vez de depender de la memoria de quien publica.

### Lo que hace 9c y por qué fail-closed

`scripts/release.sh` ahora verifica la autenticidad contra **los bytes que
el CDN acaba de servir en 9b**, pasándolos con `--assets-dir` en vez de
descargar una segunda copia. Verificar otra copia sería verificar otra
cosa.

Sin `cosign` en el PATH, el release **aborta**. El opt-out
`SDDK_SKIP_AUTHENTICITY_CHECK=1` existe y anuncia explícitamente que la
autenticidad NO se verificó — un skip silencioso sería peor que no
tener el paso.

### Falsación

El guard se falsó en sus dos ejes con mutación controlada, y se revirtió:

| Mutación | Resultado observado |
|---|---|
| `DEFAULT_CERT_IDENTITY_REGEXP` → `.*` | **FAIL=1** (la derivación del control de rama falla) |
| `DEFAULT_CERT_ISSUER` → `.*` | **FAIL=4** (0/2 payloads verifican) |

`cosign.rs` restaurado sin cambios en ambos casos (verificado con
`git diff`). El control del wildcard existe precisamente para que la
primera mutación sea detectable: si el pin degrada a `.*`, el wildcard
deja de ser una referencia y pasa a comportarse como el pin, lo que
distingue "el pin rechazó" de "el verificador falló por otra razón".

### Nota de método

El guard encontró **dos bugs en sí mismo** al ejecutarse, no al leerse:

1. La extracción del pin con `sed` era codiciosa. Al plegar newlines en
   espacios devolvía el pin **más el resto de `cosign.rs`**, y cosign
   rechazaba con `malformed subject in identity` — un diagnóstico que no
   tiene nada que ver con la firma real.
2. `verified 0/2` reportaba `ok`. Un check que verifica cero y dice PASS
   es exactamente el fallo que este guard existe para impedir.

Ninguno de los dos se habría detectado leyendo el script. Es la tercera
vez en este repo que el falsador encuentra lo que la inspección no
(INC-DEBT-033, INC-DEBT-037, y ahora el propio guard nuevo).
