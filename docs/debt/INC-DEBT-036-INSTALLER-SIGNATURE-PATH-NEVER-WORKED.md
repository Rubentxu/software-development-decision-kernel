---
id: INC-DEBT-036-INSTALLER-SIGNATURE-PATH-NEVER-WORKED
title: "El camino de verificación de firma del instalador no podía pasar nunca, y el override de framework dir que el smoke de CI usa no lo leía el CLI"
status: resolved
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-32 (OBSERVED, primer release firmado real v2.2.11)
cluster_id: CL-RELEASE
related: [INC-DEBT-035-PUBLIC-GATE-REJECTS-THE-SIGNATURES-INSTALL-REQUIRES, INC-DEBT-034-CI-RELEASE-ASSET-LAYOUT-DIVERGES-FROM-GATE]
fingerprint: "installer_signature_path_and_framework_dir_contract"
---

## Qué pasó (OBSERVED)

v2.2.11 (run `36477625442`) fue el **primer release firmado de la historia**
(pipeline canónico: 9 payloads + firmas cosign keyless + receipt + CHECKSUMS +
sbom). Al instalarlo, el instalador fallaba la verificación de firma **en tres
puntos encadenados**, y una vez reparados apareció un cuarto defecto que la
firma rota llevaba enmascarando desde siempre:

1. `verify_signature` de `scripts/install.sh` llamaba a `fetch()`, función
   **inexistente** (resto de refactor). El 127 se tragaba con `2>/dev/null` y
   el mensaje de error era siempre "no signature asset published" — la causa
   visible mentaba una causa que no era la real.
2. La regex de identidad esperaba la forma relativa
   `owner/repo:.github/workflows/release.yml@refs/tags/vX.Y.Z` pero Fulcio
   minte la **URI absoluta**
   `https://github.com/owner/repo/.github/workflows/release.yml@refs/tags/v2.2.11`
   (observado con el rechazo real de cosign sobre la firma de v2.2.11;
   evidencia en `/tmp/cosign-probe`, "Verified OK" con la forma URI).
3. `--certificate-chain` lleva solo la cadena en cosign v3.1.3: cosign aborta.
   La hoja va en `--certificate`.
4. **INC-A5-FWDIR**: el instalador documenta `SDDK_FRAMEWORK_DIR` como override
   (ADR-0011) y el job de smoke de CI lo usa con una ruta no default
   (`/tmp/sddk-smoke-framework`), pero el CLI solo resolvía
   `$SDDK_DATA_DIR/framework`. Todo install con framework dir no derivable del
   data root moría en `dev use` ("bundle version X not installed") con rollback.
   El smoke de v2.2.12 (run `36482350538`) lo expuso con la firma ya pasando:
   "signature verified" seguido del fallo de symlink.

## Por qué P1/high

Los cuatro defectos hacen lo mismo por caminos distintos: **convertir un
release firmado y correcto en ininstalable**. El 1-3 rompen la autenticidad
para todo usuario; el 4 rompe la instalación del propio pipeline de CI. Sin
reparación, v2.2.12/13 no pueden certificarse aunque su contenido sea bueno.

## Resolución (OBSERVED)

- `56eeff1f` fix(install): fetch→download_optional, regex en forma URI,
  `--certificate` hoja. Pins nuevos en `tests/test_install_asset_contract.sh`
  (subject observado, no `fetch` sin definir, bandera hoja), con mutaciones.
  e2e file:// completo verde: "signature verified (cosign keyless, identity
  and issuer pinned)" contra los assets reales de v2.2.11.
- `8689506e` fix(cli): misma forma URI en `cosign.rs` (constante + `subject_for`
  + 3 tests repuntados) y bandera hoja en `dev/update.rs`.
- `93fa5090` fix(cli): `CliEnvironment` captura `SDDK_FRAMEWORK_DIR`;
  `framework_dir()` y `sddk_framework_dir()` lo respetan; `install.sh` lo pasa
  a `dev use`/`dev doctor`. 2 pins unitarios + 4 checks de contrato con
  mutación en ambos sentidos. e2e en layout no default con binario nuevo:
  exit=0, `current → 2.2.12`, doctor `all_present: true`.

## Ciclo de releases consumido por la evidencia

| Tag | Run | Resultado |
|-----|-----|-----------|
| v2.2.11 | 36477625442 | completo y firmado; ininstalable por defectos 1-3; release PRESERVADO (no se reescribe historia) |
| v2.2.12 | 36482350538 | firmado; smoke expone INC-A5-FWDIR; release PRESERVADO |
| v2.2.13 | 36485603013 | verificación post-publicación en curso (receipt de cierre en SESSION-JOURNAL) |

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-28 | session-32 | created | diagnóstico con cosign real sobre v2.2.11 |
| 2026-09-28 | session-32 | resolved (fixes 56eeff1f, 8689506e, 93fa5090) | pins con mutaciones; e2e file:// y layout no default verdes; smoke v2.2.12 como evidencia del defecto 4 |

`resolved`, no `closed`: falta el release real (v2.2.13) pasando smoke en CI e
instalación observada de red, que cierra también INC-DEBT-034.

## References

- Run v2.2.11: https://github.com/Rubentxu/software-development-decision-kernel/actions/runs/36477625442
- Run v2.2.12: https://github.com/Rubentxu/software-development-decision-kernel/actions/runs/36482350538
- Commits: 56eeff1f, 8689506e, 93fa5090, e8b148d1 (bump 2.2.12), 096669cb (bump 2.2.13)
