# CL-release-version-source — RECEIPT del lote 2

**Bloque:** `cl-release-version-source` — bloque de roadmap, **no** es un ciclo del ledger
**Ciclo SDDK:** ninguno. El ledger de `p-63676b11dc0ef88f` da **0 filas** para
`version-source` (medido 2026-10-06). El encabezado declaraba antes un `cycle_id`
completo que la autoridad nunca emitió; corregido por INC-DEBT-063.
**Date:** 2026-10-02T14:20:00Z
**Baseline:** `main@9de63438`
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto v2.5.2)
**Scope:** `SCOPE-CONTRACT-lote-2.md` — **cerrado en 5 de 5 criterios**
**Status:** D1 cerrado. **D2 sigue abierto y escrito** (§4 de este recibo)

## 1. Qué cambió

`crates/sddk-cli/src/release_cmd.rs`, un solo fichero:

| qué | antes | después |
|---|---|---|
| `run_release_plan` | `ensure_version_lockstep(...)` | `ensure_version_lockstep_detailed(...)`, y el `VersionAuthority` se propaga |
| `ReleasePlanOutput` | `route, branch, base, tag, head, steps` | los mismos seis, **más** `version_authority` |
| `release_plan_text` | no decía de dónde sale la versión | dice la autoridad, la versión y el manifiesto leído |

Ningún campo se renombra ni se quita: el cambio es **aditivo**, y hay un test
que lo fija (`the_plan_keeps_its_previous_fields`).

## 2. Evidencia: el antes y el después, sobre el mismo caso

Proyecto Go (`go.mod`, sin `Cargo.toml`), fixture temporal, binario de este
checkout. **Antes** del cambio:

```json
{ "route": "local", "branch": "main", "base": "main", "tag": "v1.0.0",
  "head": "c90cc15", "steps": [ "push_main", ... ] }
```

exit 0, y nada que distinga este plan del de un repo Rust que sí se comprobó.
**Después**:

```json
{ "route": "local", ..., "steps": [ "push_main", ... ],
  "version_authority": {
    "kind": "tag_is_the_only_authority",
    "version": null,
    "declared_in": [],
    "undeclared_ecosystems": [ "go" ] } }
```

Y en texto, que antes no decía nada:

```text
version_authority: tag_is_the_only_authority
version: null
note: no manifest declares a version; the tag is the only authority and nothing was cross-checked
version_undeclared_in: go
```

Un repo Rust sale `cross_checked` con `version_declared_in: …/Cargo.toml (rust) = 1.0.0`.

## 3. Criterios del SCOPE, medidos uno a uno

| # | criterio | resultado | cómo se midió |
|---|---|---|---|
| 1 | la salida declara la autoridad, en JSON y en texto | **PASS** | e2e §§1-3 del falsificador + test `cli_release_plan_text_says_the_authority_out_loud` |
| 2 | Rust sale `cross_checked` con el manifiesto leído | **PASS** | e2e §1; `declared_in[0].path` termina en `Cargo.toml` |
| 3 | Go sale `tag_is_the_only_authority` y nunca `cross_checked` | **PASS** | e2e §2, con la aserción negativa explícita |
| 4 | un repo Rust conserva el lockstep, texto del refusal incluido | **PASS** | `cli_release_plan_refuses_on_version_mismatch` pasó **sin reescribirlo** |
| 5 | un proyecto sin versión declarada sigue planificando | **PASS** | e2e §2: exit 0 sobre Go |

## 4. Lo que NO se cierra: D2, y por qué no cabía aquí

`release_cmd.rs:847-848` sigue haciendo la comprobación con `?` y escribiendo
después `let version_lockstep_passed = true;`, que viaja a
`ReleaseOutcome.version_lockstep_passed`. Es un hecho afirmado que no ocurrió
en un proyecto sin versión declarada. **Se deja escrito y medido, no cerrado.**

El motivo por el que no es un arreglo de una línea está en el SCOPE §2 y es
una asimetría real, comprobada leyendo el gateway:

| campo | ¿se lee para decidir? | qué significa |
|---|---|---|
| `LocalReleasePreconditions.version_lockstep_passed` | **sí**, `release.rs:205` | puerta; `false` aborta con `ReleaseError::Precondition` |
| `ReleaseOutcome.version_lockstep_passed` | **no**: se escribe y se serializa | hecho reportado |

Cambiar la puerta local a `was_cross_checked()` **dejaría a Go y a Bazel sin
poder publicar jamás**: su lockstep no tiene nada que comprobar, luego el
resultado sería `false` y la puerta rechazaría el release. El campo reportado
sí debería llevar la distinción. Arreglarlos exige decidir cuál de los dos
nombres cambia y con qué tipo, que es un cambio de contrato de `sddk-gateway`
con sus tests de integración: un lote con SCOPE propio, no una línea.

También queda sin corregir, **por el mismo motivo**, el doc de
`sddk-gateway/src/release.rs:395-396`, que sigue diciendo «the workspace
Cargo.toml version» — una frase que ADR-0153 ya dejó de ser cierta.

## 5. Verificación ejecutada (scoped, no el perfil completo)

```text
cargo test -p sddk-cli --lib release        → 17 passed; 0 failed
cargo test -p sddk-cli --test cli release   → 33 passed; 0 failed
cargo fmt --check                           → limpio
cargo clippy -p sddk-cli --all-targets -D warnings → limpio
bash /var/home/rubentxu/ff-release-plan-authority.sh → PASS=14 FAIL=0
```

3 tests unitarios nuevos en `release_cmd.rs` (render de los dos brazos,
concordancia JSON/texto, campos previos intactos) y 3 de integración en
`cli.rs` (Go, Rust, texto).

**Falsificador, 4 mutaciones, las 4 detectadas:**

| mutación | detectada por |
|---|---|
| la salida declara `cross_checked` sin haberlo medido | e2e §4 |
| el segundo brazo del enum informa siempre `cross_checked` | e2e §5 |
| el call site D1 vuelve a la variante que aplana | e2e §6 |
| el texto deja de advertir que no hubo comprobación | e2e §7 |

## 6. El falsificador encontró un defecto en sí mismo

La primera pasada dio **12 PASS / 1 FAIL**, y la que sobrevivió era la
mutación «quitar el campo de la salida». La causa **no era un hueco del
código**: quitar un campo no opcional de una construcción de struct hace que
el fuente **no compile**, el binario viejo se queda en su sitio, el comando
sigue saliendo con `exit 0` y la aserción lee **el artefacto que no se mutó**.
El falsificador se declaraba satisfied midiendo lo contrario de lo que
creía.

Dos arreglos, ambos en el arnés:

1. La mutación inválida se sustituye por una que **sí compila** y reproduce el
   defecto de verdad —consolidar la autoridad en un `cross_checked` fijo— que
   es lo que el código haría si aplana la resolución.
2. **Toda** mutación debe comprobar que su build terminó bien antes de que se
   le pregunte nada; si no, se marca `SKIP … mutacion invalida`, nunca `PASS`.

Se añade además el paso 8: tras restaurar, el caso Go tiene que volver a su
forma correcta. Un falsificador que solo sabe decir «detectado» y no sabe
decir «sigue bien» no mide el estado final.

## 7. Lo que encontró el commit, y no el lote

Al commitear se ejecutó el paso 2b de `release.sh`
(`tests/test_changelog_coverage.sh`): **PASS=29 FAIL=8**. Solo uno de los ocho
fallos era de este lote; los otros siete eran trabajo del mismo objetivo —los
tres lotes del alias de identidad, el audit, las dos entradas de deuda del
cierre por alias y el lote 1 de ADR-0153— **sin declarar en la sección
`## [2.5.3]`**.

Eso significa que **v2.5.3 tenía dos bloqueos, no uno**: la clave del KMS y
este gate. El segundo se llevaba invisible porque el gate 2b solo corre en el
paso 2 del release, y el release está parado en el 8c por la firma.

El fallo propio era de forma, no de contenido: la huella del gate son las
**cuatro primeras palabras del payload**, en minúsculas y **sin normalizar
acentos** (`norm()` solo colapsa mayúsculas y espacios), y mi entrada empezaba
por el nombre del comando en vez de por el sujeto del commit. Las tres lotes
de `feat(identity)` comparten huella —`alias de identidad de`—, luego cada una
necesita su entrada: una sola línea habría cubierto tres commits y el gate lo
aceptaría sin distinguir nada.

Corregido en `1f93dc1a`. **PASS=38 FAIL=0**, la primera vez que este gate
queda en verde.

## 8. UAT y riesgos

| UAT | estado | nota |
|---|---|---|
| criterio 1-5 del SCOPE | **PASS medido** | §3 de este recibo |
| ADR-0153 aceptación | **no** | D2 sigue abierto; la aceptación llega con todos los criterios verdes uno a uno |
| ruido de contrato en el JSON | **ninguno** | aditivo, con test que lo fija |

Riesgo residual, ya escrito: quien lea solo `converged` y `applied` de
`release apply` sigue sin ver la autoridad. Lo ve `release plan`, que es donde
se decide publicar. Es la línea que este lote pudo tocar sin abrir el contrato
del gateway.

## 9. Contexto real / fake

- **Real:** el código, los tests, el binario reconstruido de este checkout, el
  proyecto Go y Rust de los fixtures, los 14 puntos del falsificador.
- **Fake:** nada. El proyecto Go es un `go.mod` de dos líneas, no un proyecto
  Go de producción: prueba que el contrato no necesita un repositorio Go real,
  no que un repositorio Go real publique bien. Eso último es lo que el paso
  `release apply` mediría y no se ha ejecutado.

## 10. Primer paso de la sesión siguiente

Abrir el lote 3 con SCOPE propio: decidir el contrato de los dos
`version_lockstep_passed` de `sddk-gateway` y corregir el doc de
`release.rs:395-396`, que sigue nombrando `Cargo.toml` como si el contrato de
ADR-0153 no existiera.
