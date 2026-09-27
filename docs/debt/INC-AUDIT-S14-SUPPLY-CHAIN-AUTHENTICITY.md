---
id: INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY
title: "El checksum que valida el bundle se descarga del mismo origen que el bundle"
status: open
severity: high
priority: P1
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

- **(a) Firma out-of-band** con cosign (reutiliza la infra existente).
  Coste: bajo. Cierra el hallazgo principal.
- **(b) Allowlist de miembros del tarball** antes de extraer. Coste: bajo.
  Cierra el riesgo adyacente.
- **(c) Ambos.** Recomendado: son independientes y cada uno es pequeño.

## Estado

Abierto. No ejecutado en este ciclo. Candidato a WorkItem propio de
seguridad; no es quick win porque toca la ruta de instalación.
