# Playbook UAT reproducible y receipts

## Preparación por ciclo

1. Registrar Git SHA de SDDK, proyecto fixture, branch/commit y dirty state; cargo/toolchain/provider build/version, flags, `protocol_major/minor`, snapshot de capability, OS/env y policies efectivas. Usar un workspace aislado y base de datos/CAS temporal explícita, sin modificar la instalación del usuario.
2. Seleccionar **una** claim y productor real. Fijar procedimiento de cambio A→B (p. ej. añadir/quitar una dependencia estructural), entrada y salida esperada. No afirmar relación no anunciada por proveedor real.
3. Capturar baseline con provider OFF y con provider ON (A6); grabar resultados raw en ubicación segura, conservar redacción y digests criptográficos canónicos. Ejecutar tests negativos **capaces de detectar** resultados falsos.
4. Tras cada run, capturar `exit code`, stderr clasificado, resultado tipado, evidence refs, receipt id, run/attempt, timestamp de ejecución, completeness y si el resultado fue consumido efectivamente por el comando/host final. Un test de unidad `VerifyKernel` no sustituye `sddk verify-kernel` end-to-end.
5. Para DUR: **cerrar realmente el proceso**, crear un proceso nuevo que abra el mismo storage y CAS, leer refs y reconstruir ContextCapsule/plan; in-memory fixtures no puntúan.
6. Para NEG/SEC: canaries explícitos en stdout/stderr/args, error/deny; comprobar también *artefactos persistidos* y host API, no solo salida de pantalla. No publicar canaries reales.
7. Para dynamic: bloquear o matar proceso en los puntos `proposal persisted`, `before commit`, `after commit`, `before effect receipt`; reanudar y verificar unicidad. Ejecutar otra vez con trigger duplicado y revisión parent divergente.
8. Para READ-MODEL: borrar checkpoint en copia de fixture, reconstruir desde fuente canónica y comparar campos semánticos; si hay stale o conflicto, exigir respuesta tipada, no un informe vacío.

## Plan de comandos (plantilla, NO comandos certificados)

```text
# Seleccionar herramienta realmente instalada y formato soportado:
# <provider binary> --version
# <test tool> --version
# SDDK: version, adopt status, ledger verify, plan roadmap next/blocked,
# architecture findings/receipt/why, verify-kernel --domain architecture --claim <id>.
# Ejecutar test real mediante gateway/CLI de captura SOLO cuando la ruta exista.
# No inventar que `sddk dev run-and-record` ya está instalado.
# Los detalles exactos de opciones requieren `sddk <subcommand> --help` del checkout evaluado.
```

## Plantilla de receipt de aceptación

```yaml
uat_id: AIW-A01
kind: E2E_REAL_PROVIDER
repo_sha: <sha>
fixture_revision: <sha>
workspace_dirty_basis: <digest-or-clean>
sddk_version: <version>
provider_build: <version-or-absent>
capability_snapshot_ref: <ref-or-absent>
policy_ref: <ref>
input_basis_ref: <ref>
consumer_command: <actual-command>
consumer_result_ref: <receipt-or-output-ref>
evidence_refs: [<ref>]
artifact_refs: [<ref>]
attempt_id: <id>
exit_status: <status>
completeness: complete|partial|unknown
negative_case_exercised: true|false
observed_result: <machine-readable-value>
expected_result: <machine-readable-value>
verdict: PASS|FAIL|NOT_RUN|BLOCKED
limits: <explicit>
```

Registrar un receipt por UAT; el receipt describe ejecución real y **no** puede prepararse con `verdict: PASS` antes de observarla. Si un test no se pudo ejecutar por ausencia de proveedor, marcar `BLOCKED` y mantener el hito abierto, sin cambiar Base green. El resto de tests de baseline no se dan por pasados sin logs frescos.
