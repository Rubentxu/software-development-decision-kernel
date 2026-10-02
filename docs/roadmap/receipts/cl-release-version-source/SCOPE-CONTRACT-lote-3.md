# CL-release-version-source — SCOPE-CONTRACT del lote 3: los dos `version_lockstep_passed`

**Cycle:** `p-63676b11dc0ef88f/version-source` (cluster `CL-RELEASE`, de INC-DEBT-051)
**Baseline:** `main@1c82e910` (workspace v2.5.3, declarada no publicada; último tag remoto v2.5.2)
**Opened:** 2026-10-02T15:05:00Z
**Owner:** orchestrator (ejecución directa)
**Cierra:** D2, que el lote 2 dejó abierto y escrito (`SCOPE-CONTRACT-lote-2.md` §2)
**Predecesor:** `SCOPE-CONTRACT-lote-2.md`, que este fichero **retoma**, no repite

## 0. Vigencia del defecto, verificada antes de escribir nada

El lote 2 dejó medido que los dos campos llamados `version_lockstep_passed`
**tienen que significar cosas distintas**, y se abstuvo de arreglarlos. La
medición de este lote confirma la asimetría leyendo el gateway, no el
recuerdo del lote anterior:

| campo | ¿se lee para decidir? | dónde |
|---|---|---|
| `LocalReleasePreconditions.version_lockstep_passed` | **sí**: `false` aborta con `ReleaseError::Precondition` | `release.rs:205` |
| `ReleaseOutcome.version_lockstep_passed` | **no**: se escribe en `release.rs:502` y se serializa. Cero lecturas en todo el workspace | `release.rs:75` |

Y el productor del segundo es un literal:

```rust
// release_cmd.rs:847-848
ensure_version_lockstep(&root, &args.tag).map_err(|e| anyhow::anyhow!("{e}"))?;
let version_lockstep_passed = true;
```

Sobre un proyecto Go o Bazel, esa línea informa «el lockstep pasó» sobre una
comprobación que **no ocurrió**: no hay manifiesto contra el que comparar. El
`?` de arriba garantiza que no hubo **violación**, que es un hecho distinto del
que el nombre del campo afirma.

## 1. Objetivo (falsable)

Que el resultado de un release diga **de dónde salió la versión** en vez de
afirmar un lockstep que el gateway nunca comprobó, y que la homonimia de los
dos campos desaparezca **por construcción**, no por una nota que lo explique.

**Criterio de salida, falsable por construcción:** `ReleaseOutcome` no tiene
ningún campo llamado `version_lockstep_passed`, y el único tipo que expresa
«de dónde salió la versión» en el camino de release es `VersionAuthority` del
engine — no una copia local en el gateway ni otra en la CLI.

## 2. Por qué renombrar el reported field y NO el gate

La tentación es renombrar los dos campos. El gate **no se renombra**, por una
razón medida y no estética: `LocalReleasePreconditions.version_lockstep_passed`
llega hasta el storage como **cadena**, en
`ReleaseFailureEvidence.failed_precondition` (`release_failure_evidence.rs:85`),
y tres tests de integración de la CLI la comparan literalmente
(`cli.rs:6644`, `:6988`, `:7425`). Renombrar el campo cambia esa cadena y con
ella un contrato de datos durable, a cambio de clarity en un nombre interno.

El arreglo barato y real es otro: **darles tipos distintos**. `ReleaseOutcome`
pasa a llevar `version_authority: VersionAuthority`; el gate sigue siendo el
bool que es, con la semántica que ya tenía (no-violación) y un doc que la dice.
Después del cambio los dos campos **no comparten nombre**, luego la homonimia
que impedía el arreglo desaparece sin tocar ninguna cadena durable.

## 3. No-objetivos

- **NO** cambiar la semántica de la puerta local. Sigue rejecting cuando el
  lockstep no se cumple o no se puede resolver, y Go y Bazel siguen pudiendo
  publicar. Un test de este lote lo fija por el camino negativo.
- **NO** hacer que `apply_release` **valide** la autoridad que recibe. Hoy no
  valida el bool, y validar convertiría un registro en una puerta nueva con su
  propia STOP condition. Se declara aquí, no se decide en el código.
- **NO** tocar `sddk-domain`. `ReleaseFailureEvidence` no cambia.
- **NO** hacer que `release apply` **bloquee** por falta de versión declarada.
  Este lote informa; bloquear es otra decisión, con su propio SCOPE.

## 4. Superficie

| fichero | cambio |
|---|---|
| `crates/sddk-engine/src/version_source.rs` | `Serialize` en `VersionAuthority` y `VersionCandidate`, con el mismo `snake_case` que ya emite la CLI |
| `crates/sddk-gateway/src/release.rs` | `ReleaseOutcome` lleva la autoridad; `apply_release` la recibe; doc de `:395-396` corregido |
| `crates/sddk-cli/src/release_cmd.rs` | la ruta forge deriva la autoridad; `release_outcome_text` la dice |
| `crates/sddk-gateway/tests/release_flow.rs` | las 4 llamadas pasan una autoridad real, no un `false` arbitrario |

**Una sola representación del concepto.** La autoridad es el tipo del engine;
el gateway lo reutiliza y la CLI lo muestra. Ninguna capa define la suya.

## 5. Criterios y falsificadores

| # | criterio | falsificador |
|---|---|---|
| 1 | `ReleaseOutcome` declara la autoridad y ya no afirma un lockstep no comprobado | reintroducir el bool y comprobar que el falsificador lo ve |
| 2 | el doc de `apply_release` ya no nombra `Cargo.toml` | un test **estructural** que falle si la cadena vuelve |
| 3 | la ruta forge deriva la autoridad, no la fija | mutar el call site a un literal y comprobar que el outcome cambia |
| 4 | la puerta local no cambia de semántica | los 3 tests de `failed_precondition` pasan **sin reescribirlos** |
| 5 | `release apply` dice la autoridad en su salida de texto | render sin la autoridad |
| 6 | un proyecto Go sigue publicando por la ruta local | test de la puerta en `false` con autoridad sin comprobar |

## 6. STOP conditions

1. Si la puerta local cambia de comportamiento, **incluido el texto del
   `Precondition`** ⇒ parada.
2. Si el cambio obliga a tocar `sddk-domain` ⇒ parada.
3. Si `ReleaseOutcome` gana un campo en vez de cambiarlo, y quedan los dos ⇒
   parada: serían dos fuentes de verdad para el mismo concepto, que es
   exactamente el defecto que este lote cierra.
4. Si aparece una **copia** de `VersionAuthority` en el gateway o en la CLI ⇒
   parada, aunque compile y pase.

## 7. Riesgos

| riesgo | por qué importa | mitigación |
|---|---|---|
| que el cambio de tipo rompa un consumidor del JSON | `ReleaseOutcome` es `Serialize` y público | el campo no lo lee nadie en el workspace: medido, no supuesto; y el cambio va en el changelog |
| que el doc corregido vuelva a mentir | es un doc, y los docs no se ejecutan | criterio 2 con test estructural, que es lo único que puede |
| que `Serialize` en el engine sea una dependencia que este lote no necesita | el engine ya depende de `serde`; es un derive | el tipo ya era un dato con `PathBuf` y `Vec`; solo se le da una representación |
| que arreglar la homonimia despierte a alguien a renombrar el gate | el nombre del gate es tentador y el coste está oculto | §2 lo deja escrito con el coste medido, no como preferencia |

## 8. Fuera de alcance

- Publicar v2.5.3: sigue bloqueada por la clave del KMS (bloqueo 1 de 2, ya
  medido en el lote 2; el 2 quedó verde en `1f93dc1a`).
- Promover ADR-0153 a `accepted`: la aceptación llega con D2 cerrado, y este
  lote es el que lo cierra.
- El alias de skillgraph y el contrato de read-option de INC-DEBT-049.
