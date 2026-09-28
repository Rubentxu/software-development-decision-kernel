---
id: INC-DEBT-024-LOCAL-KEYLESS-IDENTITY-UNPINNABLE
title: "Una firma keyless hecha en local no puede satisfacer el pin de identidad, y release.sh no lo comprueba"
status: open
severity: high
priority: P1
created: 2026-09-28
created_by: session-23 (verificación de la viabilidad de ADR-0143 §(a))
owner: unassigned
cluster_id: CL-SUPPLY-CHAIN
fingerprint: "sddk_local_keyless_issuer_mismatch"
fingerprint_aliases: []
---

## Qué es

`ADR-0143` §(a) propone, como opción preferida, firmar en el paso 8c de
`scripts/release.sh` **antes** de `gh release create`, para que un fallo de
firma impida publicar en vez de dejar un release publicado e ininstalable.

Al verificar si esa opción es siquiera viable, aparece un condicionante que
la ADR no recogía: **la identidad keyless depende de dónde se firma, y las dos
identidades son mutuamente excluyentes.**

Verificado contra el binario instalado (`cosign v3.1.3`), texto literal de
`cosign verify-blob --help`:

```
The OIDC issuer expected in a valid Fulcio certificate, e.g.
https://token.actions.githubusercontent.com or https://oauth2.sigstore.dev/auth.
```

| Dónde se firma | Issuer | Subject |
|---|---|---|
| GitHub Actions | `https://token.actions.githubusercontent.com` | `Rubentxu/…:release.yml@refs/tags/vX.Y.Z` |
| Local, device flow | `https://oauth2.sigstore.dev/auth` | identidad de la persona |

`crates/sddk-cli/src/cosign.rs` fija `DEFAULT_CERT_ISSUER` al issuer de GitHub
Actions y el subject al workflow sobre un tag SemVer. **Una firma hecha en
local no satisface ninguno de los dos** y, por tanto, no la verificaría ni el
propio `install.sh` del proyecto.

## Por qué importa

`scripts/release.sh` paso 8c firma lo que encuentre disponible. En local,
`cosign` está instalado (`/usr/bin/cosign`), así que la comprobación
`command -v cosign` **pasa** y el paso 8c intenta firmar. El device flow pide
un navegador, una persona lo completa, y el resultado es un release
"firmado" con la identidad equivocada: verificable por un humano que no
compruebe el issuer, e **ininstalable** para cualquier consumidor del proyecto
que sí lo compruebe, porque `install.sh` y `dev update` son fail-closed.

Es el peor de los desenlaces: el release parece correcto y no instala. Y no es
hipotético, es lo que pasaría en el primer `bash scripts/release.sh` firmado
que alguien ejecutara localmente.

Agravante: el paso 8c **no comprueba el issuer del certificado que acaba de
emitir** antes de publicar. El único sitio donde se detectaría es el gate 9b,
es decir, después de que los assets ya están subidos.

## Reproduction

```bash
# El pin exige el issuer de Actions:
grep DEFAULT_CERT_ISSUER crates/sddk-cli/src/cosign.rs
#   pub const DEFAULT_CERT_ISSUER: &str = "https://token.actions.githubusercontent.com";

# Local no hay OIDC de Actions, y cosign existe igual:
command -v cosign            # /usr/bin/cosign
[ -n "$ACTIONS_ID_TOKEN_REQUEST" ] || echo "sin OIDC de Actions"

# Issuer que emite el device flow local, segun cosign:
cosign verify-blob --help | grep 'Fulcio certificate'
#   ... https://token.actions.githubusercontent.com or https://oauth2.sigstore.dev/auth.

# Los dos son distintos -> la firma local no puede satisfacer el pin.
```

## Mitigación

1. **`release.sh` debe comprobar el issuer antes de publicar.** Tras firmar,
   extraer el issuer del certificado emitido (o de la cadena) y compararlo con
   el valor de `DEFAULT_CERT_ISSUER`. Si no coincide, `die` — fail-closed.
   Es un fix pequeño y es la defensa que convierte este INC en imposible.
2. **Alternativa: negarse a firmar fuera de CI.** Si no hay
   `ACTIONS_ID_TOKEN_REQUEST`, la única salida segura es abortar con un
   mensaje que diga que la firma keyless con la identidad del proyecto solo
   existe dentro de GitHub Actions, y apuntar a `release-automation.yml`.
3. **A medio plazo**, `ADR-0143` §(a) debería reescribirse: la opción 1 no es
   "firmar antes de publicar" sino "firmar antes de publicar **desde Actions**".

## Estado

**Mitigación 1 IMPLEMENTADA (session-24).** `scripts/release.sh` ahora
comprueba el issuer del certificado realmente emitido antes de publicar, y
hace `die` (fail-closed) si no es el esperado o si no se puede leer.

- `RELEASE_CERT_ISSUER` en `release.sh` es byte-idéntico a
  `DEFAULT_CERT_ISSUER` en `crates/sddk-cli/src/cosign.rs`. Un check de
  contrato falla si divergen.
- `cert_issuer()` lee el certificado del bundle (formato v2 en `.cert`,
  formato nuevo en `verificationMaterial.x509CertificateChain`) y extrae el
  issuer de la URI SAN, no del DN RFC4514 — el DN lleva el nombre de la CA
  Fulcio, no el proveedor OIDC.
- **Un certificado ilegible es un fallo, nunca un pase.** Un control que no
  puede leer lo que verifica no ha verificado nada.
- Falsificado en los dos sentidos: issuer de Actions → pasa; issuer local
  (`oauth2.sigstore.dev/auth`) → rechaza; bundle válido sin certificado →
  `no certificate found in bundle`; JSON corrupto → falla. Cubierto por 3
  checks en `tests/test_install_asset_contract.sh`, con mutaciones que
  confirman que detectan tanto la deriva del pin como la degradación de
  `die` a `warn`.

Siguen abiertas las mitigaciones 2 y 3.

**No es el mismo incidente que `INC-AUDIT-S14`** (autenticidad ausente: falta
publicar cualquier firma). Es un fallo latente en la ruta de firma: afecta a
un release local firmado, que nadie ha ejecutado todavía, y que fallaría de
una forma silenciosa y confusa.

## Corrección: versión de cosign en CI (session-24)

Se siguió una hipótesis que era **incorrecta** y que conviene no dejar escrita
en ningún sitio sin corregir: se afirmaba que la firma en CI iba a fallar.

Se afirmaba, a partir del `cosign v3.1.3` instalado en esta máquina, que
`--output-signature` y `--output-certificate` no existían y por tanto que
`.github/workflows/release.yml` estaba roto. **Falso**, por dos errores:

1. `sigstore/cosign-installer@053f9b74…` comentado como `v3.8.1` es la versión
   del **action**, no la de cosign. Ese SHA sí existe y resuelve a
   `refs/tags/v3.8.1` del propio `cosign-installer`.
2. Ese action define `cosign-release: default 'v2.4.3'`, y el workflow **no
   overridea** `cosign-release`. CI instala, por tanto, **cosign v2.4.3**, no
   v3.1.3.

En v2.4.3 los dos flags existen y funcionan como el workflow espera. Firma
en `cmd/cosign/cli/sign/sign_blob.go`:

```go
func SignBlobCmd(ro *options.RootOptions, ko options.KeyOpts, payloadPath string,
                 b64 bool, outputSignature string, outputCertificate string,
                 tlogUpload bool) ([]byte, error)
```

y ambos se escriben a disco (`Wrote signature to file` /
`Wrote certificate to file`).

**Conclusión observada:** el workflow de firma **no** está roto por
compatibilidad de versión. La incompatibilidad real es entre la v3 del host y
la v2 de CI, y afecta a quien intente reproducir la firma en local con
cosign v3, no a la publicación.

**Deuda menor que esto deja abierta** (no registrada como INC aparte por ser
menor): la versión de cosign que instala CI es un **default implícito** del
action. Si sigstore cambia ese default, la versión usada por la firma cambia
sin que nada en este repositorio lo refleje. Fijar `cosign-release:
'v2.4.3'` explícito en el workflow lo convierte en una decisión declarada y
visible. No se ha tocado el workflow: eso es cambio de superficie de
publicación y va en su propio slice.
