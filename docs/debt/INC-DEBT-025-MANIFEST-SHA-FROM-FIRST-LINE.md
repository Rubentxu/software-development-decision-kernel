---
id: INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE
title: "BUNDLE.toml declara manifest_sha256 con el hash de la primera línea del manifest, y nadie lo verifica"
status: resolved
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-29 (coherencia del bundle instalado)
resolved_by: session-30 (commit bc53207e)
cluster_id: CL-SUPPLY-CHAIN
fingerprint: "sddk_bundle_manifest_sha_from_first_line_unverified"
---

## Estado: resuelto en session-30 (2026-09-28)

Los dos defectos de este INC están cerrados, por separado y con commits
distintos:

- **Parte 1 (cálculo).** `release.sh` pasó a calcular el hash real con
  `sha256sum MANIFEST.sha256`. El valor publicado apunta al manifest, como
  el comentario del código declaraba desde el principio.
- **Parte 2 (verificación).** `verify_manifest_anchor` en
  `crates/sddk-cli/src/dev/bundle_manifest.rs` compara el valor declarado con
  el sha256 del `MANIFEST.sha256` realmente incluido, y `dev install` lo
  invoca después de `verify_bundle_compat` y antes de escribir nada en disco.
  Commit `bc53207e`.

La sección "Corrección propuesta" de abajo se conserva como referencia del
razonamiento original; el criterio de la compatibilidad que allí se exigía
—que un bundle antiguo con el valor viejo siga siendo legible— se resolvió
con el caso permisivo explícito de `verify_manifest_anchor`: un bundle que
**no declara** `manifest_sha256` se acepta (no afirma ningún ancla), mientras
que un bundle que **sí lo declara** y no coincide se rechaza.


## Qué es

Dos defectos encadenados en el mismo campo de integridad.

**1. El valor inyectado es el hash de un fichero arbitrario, no el del manifest.**

`scripts/release.sh:512`:

```bash
MANIFEST_SHA="$(awk 'NR==1 {print $1}' MANIFEST.sha256)"
```

`awk 'NR==1 {print $1}'` saca el hash de la **primera línea** de
`MANIFEST.sha256`, que es el sha256 del primer fichero ordenado del bundle
(`agents/analytics-judge.md`). El valor correcto sería
`sha256sum MANIFEST.sha256`.

La intención declarada está escrita en el propio código que lo consume
(`crates/sddk-cli/src/dev/bundle_manifest.rs:24`):

```
//! manifest_sha256 = "608c6d9c..."   # sha256 of MANIFEST.sha256 itself
```

El comentario dice una cosa, el cálculo hace otra.

**2. El campo no se verifica en ninguna parte.**

`manifest_sha256` se escribe (`release.sh:519`) y se parsea
(`bundle_manifest.rs:85`) pero ningún código lo compara con el sha256 real
del `MANIFEST.sha256` instalado. Búsqueda sobre
`crates/sddk-cli/src/dev/{bundle_manifest,mod}.rs` y `scripts/install.sh`:
cero usos del valor para comparar. El campo es decorativo.

## Evidencia (verificada, session-29)

Sobre el bundle instalado 2.0.1
(`~/.local/share/sddk/framework/2.0.1/`):

```console
$ grep manifest_sha256 BUNDLE.toml
manifest_sha256 = "608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f"

$ head -1 MANIFEST.sha256
608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f  agents/analytics-judge.md
                                   ^ el valor inyectado es el hash de ESTA línea

$ sha256sum MANIFEST.sha256
1f0a3f086d268c29aed422a5ff51efe8cdc8cfcc6679308950da302521fa9837
                                   ^ el hash real del manifest
```

El valor publicado coincide con la primera línea del manifest, no con el
manifest. Confirmado dos veces: en el bundle instalado y en el
`MANIFEST.sha256` del repo de desarrollo, donde la primera línea vuelve a ser
`608c6d9c…` (`agents/analytics-judge.md`).

## Por qué importa

El bundle declara un binding de integridad hacia su manifest, y ese binding
apunta a otra cosa. Combinado con que nadie lo verifica, el resultado es que:

- Un atacante que controle el bundle puede cambiar `MANIFEST.sha256` **y**
  `BUNDLE.toml` de forma coherente consigo mismo y seguiría validando, si
  algún día alguien conecta la comprobación sin corregir el cálculo. El
  campo da una falsa sensación de binding hacia el manifest.
- Hoy el campo no protege nada en absoluto. No es que proteja poco: es
  inerte.

`verify_manifest` sí comprueba cada fichero contra el manifest, así que un
cambio no autorizado en un fichero **sí** se detecta, siempre que el
manifest no se reescriba a mano. Ese es el límite real de la protección
actual, y conviene decirlo: la integridad por fichero es genuina
(`manifest.rs:92-122` está bien implementado); lo que falta es que el
manifest esté *anclado* a algo externo.

## Relación con otros INCs

- `INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY` (code-closed, distribution-open)
  trata de **autenticidad**: el checksum se descarga del mismo origen que
  el bundle, así que la cadena se autocertifica. Es distinto y sigue
  abierto. Este INC es la mitad de **anclaje local**: un campo que debería
  anclar el manifest al bundle y no lo hace. Ambos juntos significan que
  hoy no existe ningún ancla criptográfica en la cadena de instalación.
- `INC-AUDIT-S14-DEFAULT-GATE-DISCONNECTED` (open) es la misma clase de
  fallo en otro sitio: infraestructura correcta que no está conectada al
  punto que se ejecuta. Aquí el "gate" es una comparación que nadie hace.

## Corrección propuesta

1. `release.sh:512` → `MANIFEST_SHA="$(sha256sum MANIFEST.sha256 | awk '{print $1}')"`.
   Es un cambio de una línea, pero cambia el valor de un campo de
   integridad ya publicado: `install.sh` valida `BUNDLE.toml` (schema
   version 2) de forma fail-closed, asi que **un bundle antiguo con el valor
   viejo debe seguir siendo legible o el instalador rechazaria releases
   ya publicados**. Hay que comprobar la ruta de compatibilidad antes de
   tocar esto, no despues.
2. Añadir la verificación que falta: comparar `manifest_sha256` con el
   sha256 del manifest en el momento del install/update, con el mismo
   criterio fail-closed ya aplicado al resto de v2.

## Lo que NO se afirma aqui

No se afirma que el bundle instalado este comprometido. El bundle local
tiene ademas otras 12 divergencias de manifest en `skills/` y un fichero
ausente (`skills/_shared/SKILL.md`), cuyo contenido no corresponde a
ningun commit del historial — eso se registra aparte, en
`INC-DEBT-026-BUNDLE-CONTENT-NOT-IN-HISTORY`. Este INC es sobre el
cálculo del campo, que es un defecto del repositorio y es independiente de
quien escribiera en el bundle local.
