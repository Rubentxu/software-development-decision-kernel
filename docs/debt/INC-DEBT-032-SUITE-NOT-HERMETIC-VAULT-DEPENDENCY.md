---
id: INC-DEBT-032-SUITE-NOT-HERMETIC-VAULT-DEPENDENCY
title: "La suite del workspace pasa en local y falla en CI por tests que dependen de $HOME/.sddk-knowledge, fuera del repo"
status: open
severity: medium
priority: P2
created: 2026-09-28
discovered_by: session-31 (OBSERVED, fallo remoto-independent + lectura)
cluster_id: CL-VERIFICATION
related: [INC-AUDIT-S14-DEFAULT-GATE-DISCONNECTED]
fingerprint: "cli_incidence_dka_vault_tests_read_home_not_repo"
---

## Qué pasó (OBSERVED, session-31)

La suite completa del workspace **verde en local** y **roja en GitHub
Actions**, sin que ninguna de las dos ejecuciones share el mismo código
producido por el mismo commit.

- Local (`cargo test --workspace`, session-31, commit `17d9b804`):
  fallan `cli_dev_install_default_layout_is_executable_and_verify_passes` y
  `release_bump_prepends_changelog_and_resets_manifest_version`.
- Remoto (run `36450601924`, mismo rango de código): fallan
  `cli_incidence_dka_orphan_review_phase_exists` y
  `cli_incidence_dka_managed_closure_vault_route_exists`.

Los **cuatro** son defectos reales, pero de dos clases distintas. Los dos
locales eran una regresión de `release-bump.sh` (ver
`INC-DEBT-033`, corregida en session-31). Los dos remotos son este INC.

## Por qué los remotos fallan

`cli.rs:13868` y `cli.rs:13896` construyen la ruta así:

```rust
let inc_path = std::path::PathBuf::from(env!("HOME"))
    .join(".sddk-knowledge/sddk-framework/incs/INC-DKA-ORPHAN-REVIEW-PHASE.md");
assert!(inc_path.exists(), "…must exist at {}", inc_path.display());
```

`env!("HOME")` se resuelve **en tiempo de compilación**, contra el `$HOME`
de quien compiló. El fichero que comprueban vive en el vault local del
operador (`~/.sddk-knowledge/`), que:

- **no está versionado** (`git ls-files` no devuelve nada para esos dos
  nombres);
- **no lo crea el repo** en un runner limpio;
- es estado de la máquina, no del proyecto.

En local el fichero existe → `ok`. En el runner de Actions no existe →
`FAILED`. El test afirma certificar un contrato del proyecto (`REQ-DKA-004`)
cuando en realidad certifica que el HOME del operador tiene un fichero.

## Por qué no lo arreglo en esta sesión

Dos razones, y ambas son de alcance, no de pereza.

1. **No es un fix de una línea.** Un test que verifica contenido del vault
   tiene que elegir: (a) versionar la incidencia en el repo y pointedar ahí,
   (b) marcarse `#[ignore]` con el motivo, o (c) declarar el vault como
   input externo y exigir su presencia explícitamente. La opción correcta la
   decide el contrato de `REQ-DKA-004` y la autoridad del vault
   (`AGENTS.md` §2.7: el vault es fuente humana, nunca autoridad del
   runtime). Elegir sin leer ese contrato sería inventar la política de
   gouvernance del vault.
2. **El gate ya falla donde debe.** Con los tests rojos, `ci.yml` para en
   "Run workspace tests" y **no** continúa a los pasos siguientes
   (Clippy, ShellCheck, contratos: todos `skipped`). Es decir, hoy el CI
   detecta su propia impurity y no se publica en verde por accidente. El
   riesgo real no es "publicar roto", es que **el CI lleva tiempo siendo
   inalcanzable**: 0 runs históricos hasta session-31 (§2.5 dice que los
   minutos del plan free estaban agotados).

## Lo que sí está verificado

- Los dos ficheros que el test exige **existen en este host**:
  `~/.sddk-knowledge/sddk-framework/incs/INC-DKA-MANAGED-CLOSURE-VAULT-ROUTE.md`
  (4884 bytes) e `INC-DKA-ORPHAN-REVIEW-PHASE.md` (4525 bytes), ambos del
  11 de septiembre. El contrato no es ficticio: la evidencia está, el
  transporte está mal.
- 3 referencias a `sddk-knowledge` en `cli.rs`; 2 son estos tests.
- El fallo remoto es reproducible sin el vault: basta un `$HOME` limpio.

## Alcance medido (OBSERVED)

`grep -rnE 'env!\("HOME"\)|home_dir\(\)' crates/*/tests/*.rs` devuelve
**exactamente 2 coincidencias**, ambas en `cli.rs` (13871 y 13898). No hay
más tests que dependan del HOME del operador por esa vía, así que el
alcance es N=2, no N=?. La suite completa no tiene otras impurezas de este
tipo por `$HOME` de compilación.

## Acción siguiente propuesta

Abrir WorkItem propio para el contrato `REQ-DKA-004` sobre el vault
(vs repo), con las tres opciones de arriba decididas explícitamente. Con el
alcance ya medido (2 tests, 1 fichero) es un slice acotado, no una
auditoría: las tres opciones son (a) versionar la incidencia en el repo y
apuntar el test ahí, (b) `#[ignore]` con motivo explícito, o (c) declarar
el vault como input externo y fallar con un mensaje que lo diga. La
elección es política de gobernanza del vault (`AGENTS.md` §2.7), no una
corrección técnica, y por eso no se toma aquí.
