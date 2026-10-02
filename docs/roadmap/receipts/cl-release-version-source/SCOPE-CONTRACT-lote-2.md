# CL-release-version-source — SCOPE-CONTRACT del lote 2: que la ruta de release diga de dónde sale la versión

**Cycle:** `p-63676b11dc0ef88f/version-source` (cluster `CL-RELEASE`, de INC-DEBT-051)
**Baseline:** `main@9de63438` (workspace v2.5.3, declarada no publicada; último tag remoto v2.5.2)
**Opened:** 2026-10-02T14:05:00Z
**Owner:** orchestrator (ejecución directa)
**Lote hermano:** el SCOPE-CONTRACT de este ciclo cubre `version_source.rs` y
`version.rs`. Este fichero cubre `release_cmd.rs`, que aquel §4 **no** listaba.
Es una ampliación declarada, no una consentedida.

## 0. Vigencia del defecto, reproducida antes de escribir nada

El lote 1 dejó el resultado del contrato disponible
(`ensure_version_lockstep_detailed` → `VersionAuthority`) y el ADRs lo nombra
como riesgo asumido. Lo que no hizo —y no podía hacer, porque no tocaba esta
superficie— es que ese resultado **llegue a quien decide si publica**.

Medido con el binario de este checkout sobre un proyecto Go, en fixture
temporal fuera del repositorio:

```text
$ sddk release plan --tag v1.0.0 --format json     # proyecto con go.mod
exit 0
{
  "route": "local", "branch": "main", "base": "main",
  "tag": "v1.0.0", "head": "c90cc15",
  "steps": [ "push_main", "verify_main_sha", "create_annotated_tag", "verify_remote_tag" ]
}
```

Go no declara versión en ningún manifiesto. **No hubo comparación del tag con
nada**, y el plan es byte a byte la forma que tendría el de un proyecto Rust
que sí se comprobó. Quien lee esta salida no tiene forma de distinguir un
verde de un «no había nada que comprobar».

### Los dos sitios donde se pierde

| # | sitio | qué se pierde |
|---|---|---|
| D1 | `release_cmd.rs:668` (`run_release_plan`) | `ensure_version_lockstep` ends in `map(\|_\| ())`: el `VersionAuthority` se descarta y la salida no lo menciona |
| D2 | `release_cmd.rs:847-848` (ruta forge) | la comprobación se hace con `?` y luego se **escribe** `let version_lockstep_passed = true;`. Ese literal viaja a `ReleaseOutcome.version_lockstep_passed` |

## 1. Objetivo (falsable)

Que `sddk release plan` declare **de dónde salió la versión** y con qué fuerza,
en JSON y en texto, y que ninguna afirmación de lockstep exista en la ruta de
release sin una comprobación detrás.

**Criterio de salida, falsable por construcción:** la salida de
`release plan` contiene la autoridad, y un proyecto que no declara versión
**no** se presenta como comprobado.

## 2. Lo que este lote NO arregla, y por qué (D2)

D2 se corrige **en otro lote**, y el motivo no es el tamaño del cambio: es que
los dos campos que se llaman `version_lockstep_passed` **tienen que significar
cosas distintas**, y hoy se llaman igual.

| campo | se lee para decidir | qué significa |
|---|---|---|
| `LocalReleasePreconditions.version_lockstep_passed` (`release.rs:144`) | **sí**, en `release.rs:205` | puerta: si es `false`, `ReleaseError::Precondition` |
| `ReleaseOutcome.version_lockstep_passed` (`release.rs:75`) | **no**: se escribe y se serializa, nadie lo consulta | hecho reportado |

La puerta local **no puede** pasar a ser «hubo comparación»: si lo hiciera,
`was_cross_checked()` sería `false` en Go y en Bazel y **ningún release local
de esos ecosistemas podría volver a publicarse**. El campo reportado sí
debería llevar esa distinción. Arreglarlos exige decidir cuál de los dos dos
nombres cambia y con qué tipo — un cambio de contrato de `sddk-gateway` que
toca sus tests de integración — y ese es un lote con su propio SCOPE.

Mientras tanto, D2 queda **escrito y medido**, no cerrado.

## 3. No-objetivos

- **NO** cambiar la semántica de la puerta local (§2, tabla).
- **NO** tocar `crates/sddk-gateway/` en este lote.
- **NO** relajar el lockstep de Rust. El predicado se cumple igual que antes,
  incluido el texto del error: si algo cambia ahí, es una regresión.
- **NO** convertir el plan en un informe de auditoría. Declara la autoridad;
  no reescribe el contrato del lote 1.

## 4. Superficie

| fichero | cambio |
|---|---|
| `crates/sddk-cli/src/release_cmd.rs` | `ReleasePlanOutput` gana la autoridad; `release_plan_text` la renderiza; los call sites preguntan por la variante que la devuelve |

Un solo fichero de código. El cambio es **aditivo** en el JSON: los consumidores
que leen `branch`, `base`, `tag` o `steps` siguen viendo lo mismo.

## 5. Criterios y falsificadores

| # | criterio | falsificador |
|---|---|---|
| 1 | la salida declara la autoridad, en JSON y en texto | quitar el campo de la construcción del plan y comprobar que el e2e lo nota |
| 2 | Rust sale `cross_checked` **con el manifiesto que se leyó** | el mismo e2e sobre un proyecto Go, que no puede ser `cross_checked` |
| 3 | Go sale `tag_is_the_only_authority` y nunca `cross_checked` | mutar el brazo del enum para que todo caiga en `cross_checked` |
| 4 | un repo Rust conserva el lockstep, incluido el texto del refusal | el test de mismatch ya existente, sin reescribirlo |
| 5 | un proyecto sin versión declarada **sigue** planificando | es el criterio 3 visto desde el otro lado: el lote no bloquea Go |

## 6. STOP conditions

1. Si el lockstep de un repo **Rust** cambia de resultado o de texto ⇒ parada.
2. Si el cambio obliga a tocar `crates/sddk-gateway/` ⇒ parada, y D2 se cierra
   en su propio ciclo (§2 explica por qué no caben juntos).
3. Si `release plan` pasa a fallar en un ecosistema que antes pasaba ⇒ parada.
   El cambio informa; no bloquea.
4. Si la salida resulta no ser aditiva para un consumidor existente ⇒ parada.

## 7. Riesgos

| riesgo | por qué importa | mitigación |
|---|---|---|
| que un release de Go se lea como verificado | es el falso verde, el que este lote cierra | dos valores distintos y un test que niega `cross_checked` en Go |
| que «el tag es la única autoridad» se lea como lockstep desactivado en general | sería una versión más amplia del mismo error | el valor solo lo toman los ecosistemas cuya entrada del registro lo dice; el criterio 2 fija que Rust nunca lo toma |
| arreglar D2 a medias, cambiando la puerta local | dejaría Go sin poder publicar | §2 lo prohíbe explícitamente y STOP 2 lo acota |

## 8. Fuera de alcance

- El lote 3 de D2 (contrato de `sddk-gateway`).
- La clave del KMS, que bloquea v2.5.3 y es del operador.
- Publicar la release.
