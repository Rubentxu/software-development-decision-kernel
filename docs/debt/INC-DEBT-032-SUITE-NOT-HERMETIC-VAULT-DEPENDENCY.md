---
id: INC-DEBT-032-SUITE-NOT-HERMETIC-VAULT-DEPENDENCY
title: "La suite del workspace pasa en local y falla en CI por tests que dependen de $HOME/.sddk-knowledge, fuera del repo"
status: closed
severity: medium
priority: P2
created: 2026-09-28
discovered_by: session-31 (OBSERVED, fallo remoto-independent + lectura)
resolved: 2026-09-29 (session-33)
cluster_id: CL-VERIFICATION
related: [INC-AUDIT-S14-DEFAULT-GATE-DISCONNECTED]
fingerprint: "cli_incidence_dka_vault_tests_read_home_not_repo"
---

## Resolución (session-33, 2026-09-29) — cerrado

El INC planteaba tres opciones y decía que elegir una sería inventar política
de gobernanza del vault. **No hacía falta inventarla: `AGENTS.md` §2.7 ya dice
que el vault es fuente humana y nunca autoridad del runtime.** Leer el vault
desde un test para certificar comportamiento del runtime es exactamente la
inversión que §2.7 prohíbe. La política ya existía; solo faltaba aplicarla.

Los dos ficheros del vault, además, están `status: closed` y su campo
`resolution` describe **invariantes de runtime**, no objetos del vault:

- `INC-DKA-ORPHAN-REVIEW-PHASE` → "Phase::Review no existe; hay 9 variantes".
- `INC-DKA-MANAGED-CLOSURE-VAULT-ROUTE` → "la transición `archive.vault.complete`
  está declarada en `workflow/workflow.yaml` con los gates esperados".

O sea: los tests certificaban la **fuente humana** de un invariante que el
proyecto ya cumple por otra vía. Se sustituyeron por la certificación de la
invariante real:

| Antes (no hermético) | Ahora (hermético) |
|---|---|
| `cli_incidence_dka_orphan_review_phase_exists` — leía `~/.sddk-knowledge/…` vía `env!("HOME")` | `cli_phase_enum_has_no_orphan_review_variant` — enum `Phase` compilado + su forma serializada (serde) |
| `cli_incidence_dka_managed_closure_vault_route_exists` — leía `~/.sddk-knowledge/…` vía `env!("HOME")` | `cli_archive_vault_complete_transition_declares_its_gates` — transición de `workflow/workflow.yaml` con sus 3 gates |

El test de Phase certifica el **nombre serializado** (`#[serde(rename_all =
"lowercase")]`), que es lo que consumen `workflow.yaml`, los prompts y cada
artefacto persistido. No `Debug`, que no es contrato de nadie.

## Falsificación (OBSERVED, session-33)

Cada test se probó con la mutación que debe detectarlo. Esto importa porque el
primer intento de mutación **no aplicó** (la cadena buscada no coincidía con el
fichero real) y el test dio verde sobre un fichero sin cambios: un verde que
no prueba nada. Se repitió contra la mutación real.

| Dirección | Mutación | Resultado |
|---|---|---|
| Verde | árbol restaurado | 2 passed / 0 failed |
| **ROJO (gates)** | eliminar el gate `vault-index-current` del bloque `archive.vault.complete` | **FAILED** con el mensaje nombrando el gate |
| **ROJO (Phase)** | `#[serde(rename = "uat-stage")]` sobre `Phase::Uat` | **FAILED**: `left: [… "uat-stage" …] right: [… "uat" …]` |

**La segunda mutación es la que importa, y su resultado fue una sorpresa
instruccional.** Se eligió deliberadamente una mutación que **sí** compila:
`assert_variant_count_eq!(Phase, 9, …)` y los `match` exhaustivos siguen
verdes, porque ni el conteo ni la exhausividad ven un cambio de *nombre
serializado*. Ese hueco es exactamente el que cubre el test nuevo, y la
aserción lo detecta.

Mutaciones descartadas por no ser concluyentes, con el motivo (para que nadie
las repita como evidencia):

- **Reintroducir `Phase::Review`**: no compila (match exhaustivo, E0004) y
  rompe `assert_variant_count_eq`. Es un guard **de tiempo de compilación**,
  más fuerte que un test rojo, pero no prueba la aserción del test.
- **Renombrar el *variante* `Uat` → `Uat2`**: tampoco compila (E0599 en
  `execution_scope.rs`, `compiler.rs` y en el propio test). Sigue sin ser una
  prueba de la aserción.

Conclusión honesta: la suite ya tenía dos guards de compilación que impiden
cambios de *variante*; lo que faltaba era el nombre serializado, y es lo
único que el test nuevo verifica.

## Lo que sigue siendo verdad de este INC

El gate de CI ya no está rojo por esto, pero la impureza era real y la clase
de defecto no desaparece: cualquier test futuro que lea `$HOME` para certificar
el proyecto reintroduce el mismo problema. El guard permanente que lo
impide es `grep -rnE 'env!\("HOME"\)|home_dir\(\)' crates/*/tests/*.rs`.

Resultado observado tras el cambio (session-33): **2 coincidencias, ambas
comentarios** que describen el defecto en los tests nuevos — ninguna llamada.
Antes había 2 llamadas. El grep por sí solo no distingue una cosa de la otra,
así que el guard definitivo debe filtrar comentarios; queda anotado como
mejora del guard, no como deuda abierta.


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
