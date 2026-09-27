---
id: INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY
title: "El checksum que valida el bundle se descarga del mismo origen que el bundle"
status: code-closed, distribution-open
severity: high
priority: P1
# status real en session-21: la politica de firma y los consumidores estan
# implementados y verificados (ver seccion "Actualizacion session-21"), pero
# v2.0.1 no tiene ningun asset de firma y ninguna firma real se ha obtenido.
# Cerrar en distribucion requiere publicar v2.0.7 con el workflow firmado.
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
