---
id: INC-DEBT-035-PUBLIC-GATE-REJECTS-THE-SIGNATURES-INSTALL-REQUIRES
title: "El gate de 9 assets rechaza las firmas que el instalador exige: un release correctamente firmado no puede pasar su propio gate"
status: resolved
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-31 (OBSERVED — ejecución del gate con mocks, no lectura)
cluster_id: CL-RELEASE
related: [INC-DEBT-024-LOCAL-KEYLESS-IDENTITY-UNPINNABLE, INC-DEBT-030-LOCAL-RELEASE-BLOCKED-AT-SIGNING-IDENTITY, INC-DEBT-034-CI-RELEASE-ASSET-LAYOUT-DIVERGES-FROM-GATE]
fingerprint: "public_release_gate_extra_assets_rejects_signatures"
resolved_by: session-31 (allowlist fix + gate suite + spec amendment)
---

## Qué pasó (OBSERVED, session-31)

Ejecuté `run_public_release_gate` con la respuesta de `gh` simulada. El
conjunto de assets era **exactamente el contrato canónico de 9** más las dos
firmas que `release.sh` publica:

```json
{"tagName":"v9.9.9","isDraft":false,"isPrerelease":false,"assets":[
 {"name":"sddk"}, {"name":"sddk.sha256"},
 {"name":"sddk-v9.9.9-sddk-linux-x86_64-musl.tar.gz"},
 {"name":"sddk-v9.9.9-sddk-linux-x86_64-musl.tar.gz.sha256"},
 {"name":"CHECKSUMS"}, {"name":"sbom.json"},
 {"name":"gh-release-receipt.json"},
 {"name":"software-development-decision-kernel.tar.gz"},
 {"name":"software-development-decision-kernel.tar.gz.sha256"},
 {"name":"sddk.sig"}, {"name":"sddk.pem"}]}
```

Resultado:

```
✓ isDraft=false
✓ isPrerelease=false
✗ unexpected assets replacing canonical ones: sddk.pem sddk.sig
rc=1
```

Un release **correctamente firmado** — los 9 assets canónicos más sus firmas
— **es rechazado por el gate**. La firma no es un extra tolerado: es un
asset que el gate cuenta como "sustituyendo" a los canónicos.

## Por qué esto importa más de lo que parece

Las dos mitades del sistema se contradicen, y la contradicción está oculta
porque hoy ninguna mitad se puede ejecutar hasta el final:

| Componente | Postura sobre las firmas |
|---|---|
| `scripts/install.sh:394,430` | **Exige** firma. Sin ella, aborta (salvo `SDDK_ALLOW_UNSIGNED=1`) |
| `scripts/release.sh:974-976` | **Sube** `.sig`, `.bundle.json` y `.pem` como assets del release |
| `scripts/release.sh:1008` (gate 9b) | **Rechaza** cualquier asset que no esté en la lista de 9 |

`release.sh` sube las firmas en el paso 9 y ejecuta el gate en el 9b. Si la
firma estuviera disponible — la tuviese, el paso 9 publicaría los `.sig`/`.pem` y
el paso 9b abortaría con `unexpected assets`. **El script se contradice a sí
mismo en dos pasos consecutivos**, y no hay test que lo cubra.

**Por qué nadie lo ha visto**: el guard de `INC-DEBT-030` aborta en el paso
**8c** (firmado), que es *anterior* al 9. Así que la rama que firmaba nunca
llegó al gate. Y el único release público íntegro, `v2.0.1`, **no tiene
ninguna firma** (0 assets `.sig`/`.pem` observados). O sea: el único release
que pasó el gate pasó porque no estaba firmado. La contradicción es
invisible por construcción.

`v2.0.1` cumple el contrato de 9 assets, pero **no es instalable sin
`SDDK_ALLOW_UNSIGNED=1`**. El PRE-FLIGHT de esta sesión lo llamó "íntegro" sin
verificar esa condición. La corrección: es el último release con el contrato
de assets correcto, y el único publicado por la vía de Actions que produjo
binarios musl reales — pero **no está firmado**.

## Por qué no lo arreglo aquí

Arreglarlo exige elegir qué es canónico, y las dos respuestas son cambios de
política de distribución, no correcciones:

**(A) Las firmas son aditivas al contrato.** El gate pasa a exigir los 9
assets *como mínimo* y filtra `.sig`/`.pem`/`.bundle.json` antes de comparar
extras. Es lo que ya asume `release.sh` en su comentario (líneas 961-966:
*"They are additive to the 9-asset canonical contract, not part of it"*) — el
código y el comentario ya discrepan. Requiere cambiar el gate **y** sus 10
escenarios en `tests/test_release_public_gate.sh`, ninguno de los cuales
incluye una firma en sus fixtures (verificado: los 4 bloques de assets del
test sólo contienen los canónicos).

**(B) Las firmas son parte del contrato.** El gate pasa a esperar 9+N assets
y `gh-release-receipt.json` se mueve. Rompe la forma del gate y obliga a
re-escribir las expectativas de todos los releases ya publicados.

(A) es la lectura coherente con el instalador y con `release.sh`. Pero
relajar un gate que hoy falla cerrado es una decisión de seguridad, y
`INC-DEBT-024`/`INC-030` ya demostraron que este repo prefiere fallar
cerrado antes que publicar algo que no puede verificar. Relajar el gate
para que un release firmado pase es exactamente el caso donde ese
principio se decide, no donde se aplica solo.

## Lo que sí está verificado

- El gate rechaza firmas: ejecución con mocks, `rc=1`, mensaje literal.
- El gate **no** filtra firmas antes de comparar: `lib_public_release_gate.sh:85-91`
  hace `comm` sobre la lista completa, sin `grep -v` de extensiones.
- El instalador **exige** firma: `install.sh:303-312` aborta sin
  `SDDK_ALLOW_UNSIGNED=1`.
- `release.sh` **sube** firmas antes del gate: líneas 974-976 vs 1008.
- `v2.0.1` no tiene firmas: 0 assets `.sig`/`.pem` observados.
- El gate tiene un solo consumidor productivo: `release.sh:1008`. El resto
  son el propio test y la lib.

## Acción siguiente propuesta

Decidir (A) o (B) y aplicarlo como cambio único con sus tests. Si es (A),
el test del gate debe **ganar** un escenario: los 9 canónicos + firmas → PASS.
Hoy ese caso es un FAIL y nadie lo Pins.

Mientras tanto: **ningún release firmado puede pasar el gate actual**, y
**ningún release sin firma es instalable sin `SDDK_ALLOW_UNSIGNED=1`**. Las
dos condiciones son excluyentes con el estado actual del repositorio, que es
lo que hace que esto sea P1 y no P3.

## Resolución (2026-09-28, OBSERVED)

La decisión A fue implementada y verificada:

- `scripts/release.sh` y `tests/lib_public_release_gate.sh` ahora exigen los
  9 assets canónicos y permiten únicamente `.sig`, `.pem` y `.bundle.json`
  para `sddk`, el tarball unificado x86_64-musl y el bundle de framework.
- Un asset arbitrario sigue siendo rechazado, incluso si termina en `.sig`.
- El test de contrato añadió el escenario de los 9 assets más las 9 firmas
  permitidas: **PASS**.
- La mutación que quitó la allowlist volvió a rechazar las firmas: **FAIL**,
  demostrando que el test detecta la regresión.
- `tests/test_release_public_gate.sh` terminó con **PASS=12, FAIL=0**.
- La especificación del ciclo recibió un addendum append-only que distingue
  el conjunto canónico de payloads de los sidecars de firma.

La deuda queda `resolved`, no `closed`: todavía falta observar un release
real firmado que pase el gate y una instalación end-to-end. Esa evidencia
pertenece al bloque posterior de INC-DEBT-034 y al release, no se fabrica en
este commit.
