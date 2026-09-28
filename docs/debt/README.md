# Debt Documentation

This directory contains the canonical contracts for the durable debt remediation framework (ADR-0047).

## Files

- **[SEVERITY.md](./SEVERITY.md)** — Severity taxonomy (`critical | high | medium | low`). Intrinsic technical impact, independent of scheduling.
- **[PRIORITY.md](./PRIORITY.md)** — Priority taxonomy (`P0 | P1 | P2 | P3`). Remediation scheduling, distinct from UAT priority namespace.
- **[debt-report.schema.json](./debt-report.schema.json)** (v1.0.0, draft-07) — JSON Schema for the per-cycle debt report.
- **[INCIDENCE-TEMPLATE.md](./INCIDENCE-TEMPLATE.md)** — Template for `INC-NNN-{slug}.md` cross-cycle records.
- **[INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE](./INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE.md)** (high/P1, open) — `BUNDLE.toml` declara `manifest_sha256` con el hash de la **primera línea** de `MANIFEST.sha256` (`awk 'NR==1 {print $1}'`, `release.sh:512`), no con el hash del manifest. El comentario del consumidor dice "sha256 of MANIFEST.sha256 itself", así que el código contradice su propia intención documentada. Peor: el campo **no se verifica en ninguna parte** — se escribe y se parsea, nadie lo compara. Es un campo de integridad inerte que además apunta a otra cosa. Descubierto en session-29 al verificar la coherencia del bundle instalado. Corregir el cálculo cambia un valor ya publicado, así que hay que comprobar antes la compatibilidad de `install.sh` con `BUNDLE.toml` v2 fail-closed.
- **[INC-DEBT-027-MUSL-TOOLCHAIN-ABSENT-LOCAL-PUBLISH](./INC-DEBT-027-MUSL-TOOLCHAIN-ABSENT-LOCAL-PUBLISH.md)** (high/P1, open) — `bash scripts/release.sh` aborta en el paso 3/14 al compilar `sddk-linux-x86_64-musl.tar.gz`: el target Rust `x86_64-unknown-linux-musl` esta instalado pero falta el cross-compiler `x86_64-linux-musl-gcc` que necesita `ring v0.17.14` (cc-rs). **No recuperable sin operador**: `sudo -n` falla, sudo exige password. No es un fallo de la ruta de release — el guard es el que impide repetir INC-DEBT-021-MUSL-ASSET-NAME-LIE, y aborta **antes** de publicar, asi que no hay estado parcial. Recuperacion: (1) `sudo apt-get install -y musl-tools gcc-x86-64-linux-gnu` y reintentar, o (2) publicar por CI, que si tiene el toolchain. Observa tambien una **colision de id**: `INC-DEBT-021` esta reclamado por dos deudas distintas (clippy baseline y el asset musl); no se renumera porque la historia publicada es inmutable.
- **[INC-DEBT-026-BUNDLE-CONTENT-NOT-IN-HISTORY](./INC-DEBT-026-BUNDLE-CONTENT-NOT-IN-HISTORY.md)** (medium/P2, open) — El bundle local en `~/.local/share/sddk/framework/2.0.1/` no satisface su `MANIFEST.sha256`: 12 ficheros divergentes y 1 ausente. **Alcance resuelto en session-29 contra el asset publicado: el bundle de GitHub Releases v2.0.1 verifica 377/377 y coincide con el repo, así que no hay incidente de distribución** (baja de high/P1 a medium/P2). La causa es local: el `skills/` del bundle es contenido de **otro proyecto**, 11 de 12 ficheros idénticos a `~/.config/kilo/skills/` y 0 de 12 al repo de sddk-framework, que se declara `__managed_by: gentle-ai/sdd`. Queda abierto determinar el mecanismo: si el bootstrap de `gentle-ai/sdd` escribe en `~/.local/share/sddk/framework/`, es una intrusión en el bundle de otro proyecto y contradice la regla de cero intrusión. `sddk dev doctor` ya lo detectaba (`content.manifest: missing`) y fue descartado como "instalación vieja" durante varias sesiones — descriptora que ha resultado incorrecta.
- **[INC-DEBT-024-LOCAL-KEYLESS-IDENTITY-UNPINNABLE](./INC-DEBT-024-LOCAL-KEYLESS-IDENTITY-UNPINNABLE.md)** (high/P1, **closed** en session-25) — Una firma keyless hecha **en local** emite issuer `https://oauth2.sigstore.dev/auth` y el subject de la persona, no `https://token.actions.githubusercontent.com` y `release.yml@tag`. El pin de `cosign.rs` exige los dos valores de Actions, así que **una firma local no la verifica ni el propio `install.sh` del proyecto**: el release parecería correcto y sería ininstalable. Descubierto en session-23. **Cerrado en session-25** por las tres vías: (1) gate de issuer fail-closed que lee el certificado real del bundle y muere si no es el esperado (`08639ff`); (2) pre-check que aborta antes de firmar si no hay `GITHUB_ACTIONS=true`, lo que además evita el device flow interactivo; (3) `ADR-0143 §(a)` reescrita como "firmar antes de publicar **desde Actions**". 6 checks de contrato, 5 mutaciones que los detectan. **La ruta de firma ya no puede publicar con la identidad equivocada;** lo que sigue abierto es la autenticidad en distribución (`INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY`), que necesita un release real firmado.
- **[INC-DEBT-023](./INC-DEBT-023-lints-advisory-no-expansion-cycle.md)** — Lints advisory sin ciclo de expansión programado (low/P3, closed; decisión registrada 2026-09-20: advisory by design, expansión orgánica, sin ciclo que programar).
- **[INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH](./INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH.md)** - Un commit de metadata derivada (`MANIFEST.sha256`) no tiene camino admisible de push sin quemar un bump (medium/P2, closed; regla (C) generated-only en `githooks/pre-push`, cycle inc-derived-metadata-push-path 2026-09-20).
- **[INC-FINDING-A5-2-DW-RUNTIME-003-CLOCK-SKEW](./INC-FINDING-A5-2-DW-RUNTIME-003-CLOCK-SKEW.md)** - Commentario de anclaje DW-RUNTIME-003 en restart_survival.rs desalineado con lo que el test hace (low/P3, closed; cycle a5-2-2026-09-17).

### Session-16 (2026-09-27) — auditoría con ejecución real del e2e

Todos los hallazgos verificados ejecutando el instalador y la suite e2e en contenedor, no por lectura de código.

- **[INC-DEBT-021-MUSL-ASSET-NAME-LIE](./INC-DEBT-021-MUSL-ASSET-NAME-LIE.md)** (high/P1, **closed** en session-19) — El asset `sddk-<tag>-sddk-linux-x86_64-musl.tar.gz` contenía un binario **glibc**, no musl. **Causa raíz**: `release.sh` compilaba con el target del host y empaquetaba con nombre musl. **Resuelto**: `release.sh` compila con `--target x86_64-unknown-linux-musl` y verifica `statically linked` antes de publicar (fail-closed). Binario real verificado ejecutándose en Debian 12 (glibc 2.36) y Alpine 3.20. La premisa de session-17 ("build musl bloqueado, falta `musl-gcc`") era falsa: `musl-gcc` es el wrapper de toolchains glibc cruzados; en Alpine el `gcc` nativo ya es musl. Pendiente: publicar un release nuevo, `v2.0.1` sigue siendo glibc.
- **[INC-DEBT-022-INSTALLER-ASSET-NAME-404](./INC-DEBT-022-INSTALLER-ASSET-NAME-404.md)** (critical/P1, **closed**) — `install.sh` pedía el asset `sddk-linux-x86_64-musl`, que el release nunca publicó (el asset real es `sddk`). HTTP 404 y abort antes de enlazar nada, en la ruta que se toma sin `gh` en PATH. Corregido en session-16 y protegido por `tests/test_install_asset_contract.sh`.
- **[INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY](./INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY.md)** (high/P1, open) — **Opción (a) re-evaluada**: session-14 la estimaba como "coste bajo, reutiliza infra existente". Falso. `install.sh` no menciona cosign, `release.sh` no firma nada, ningún release publica asset de firma, y el e2e exigía una cadena que nadie imprimía. Cerrar (a) es trabajo desde cero, con trust root por decidir.

### Session-14 (2026-09-27) — auditoría basada en código

Detalle y evidencia de reproducción en cada fichero.

- **[INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY](./INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY.md)** (high/P1, open) — El `.sha256` que valida el bundle se descarga del mismo origen que el bundle: integridad sí, autenticidad no. La opción (b) (guard de traversal en `dev/update`) quedó **cerrada** en session-14; la firma out-of-band sigue abierta.
- **[INC-AUDIT-S14-TEST-PORTS-UNCONSUMED](./INC-AUDIT-S14-TEST-PORTS-UNCONSUMED.md)** (medium/P2, open) — 9 traits del SPI de SPEC-043 §4 con implementadores internos pero sin consumidor externo. **Revisado** desde high/P1: la recomendación original de borrar queda anulada.
- **[INC-AUDIT-S14-NO-STRUCTURED-LOGGING](./INC-AUDIT-S14-NO-STRUCTURED-LOGGING.md)** (medium/P2, open) — 0 `tracing` en `sddk-cli`; sin correlación de `cycle_id` ni `request_id` en fallos.
- **[INC-AUDIT-S14-RELEASE-FORCE-VERSION-ERGONOMICS](./INC-AUDIT-S14-RELEASE-FORCE-VERSION-ERGONOMICS.md)** (low/P2, open) — El override de versión funciona pero su señal queda enterrada en un `warn` de un log de 400 líneas. No es un bug: el flag está plumbéado (`release.sh:361-363`).
- **[INC-AUDIT-S14-PACK-UAT-NO-CRATE-CONSUMERS](./INC-AUDIT-S14-PACK-UAT-NO-CRATE-CONSUMERS.md)** (low/P3, open) — `sddk-pack-uat` tiene 0 consumidores como crate, pero es un pack de ADR-0104, no código muerto. No borrar.
- **[INC-AUDIT-S14-WRITER-XDG-TRAIT-UNIMPLEMENTED](./INC-AUDIT-S14-WRITER-XDG-TRAIT-UNIMPLEMENTED.md)** (low/P3, open) — `WriterXdgFailClosed` exportado con 0 implementadores; deuda ya declarada en el propio código.

## Source of truth

- [ADR-0047 — Remediación durable y priorizada de deuda técnica](../adr/ADR-0047-durable-debt-remediation.md)

## Status

- cycle-7a: ratified status, severity+priority taxonomies published (this directory).
- Cycle-7a + 7b: runtime contracts live.

## LOC policy

The project's LOC budget policy is documented in [ADR-0048](../adr/ADR-0048-loc-budget-policy-reformulation.md). The policy uses **total-module-sum budgets** (implementation + boilerplate + test fixtures) rather than per-file targets. Per-file targets are deprecated as of cycle-10.
