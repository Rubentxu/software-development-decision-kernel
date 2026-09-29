---
id: INC-DEBT-037-SDDK-STATE-HOME-IGNORED-BY-LEDGER-PATHS
title: "`SDDK_STATE_HOME` no dirige el ledger: una verificación aislada escribió en la base de datos real"
status: closed
resolved_at: 2026-09-29
resolved_by: session-42
resolution_commit: e62da1bc
resolution_tests: crates/sddk-engine/tests/inc_debt_037_state_home_precedence.rs (6/6 PASS)
severity_at_detection: critical
priority: P1
created: 2026-09-29
discovered_by: session-15 (observado durante la verificación de MIGRATION_21)
cluster_id: CL-STATE-RESOLUTION-DIVERGENCE
fingerprint: "sddk_state_home_ignored_by_resolve_xdg_paths_v1"
---

## Observación

`SDDK_STATE_HOME` se documenta como el override de primer orden del
directorio de estado, pero **no lo consulta ningún camino que abra el
ledger**. Solo lo lee `admission.rs`, que gobierna el bucle de aprobación
y no el acceso a datos.

`resolve_xdg_paths` (`crates/sddk-engine/src/paths.rs:110-123`) construye
la ruta del ledger a partir de `state_home`, y ese campo lo rellena
únicamente `XDG_STATE_HOME` (`crates/sddk-cli/src/lib.rs:726`):

```rust
// crates/sddk-cli/src/lib.rs:726
state_home: nonempty_env_path("XDG_STATE_HOME"),
```

Es decir, la precedencia real es `XDG_STATE_HOME` > `HOME` > `~/.local/state`,
y `SDDK_STATE_HOME` no aparece por ningún lado. La doc de
`admission.rs:186-188` afirma lo contrario:

> Resolve the default state dir used for ledger access, mirroring
> `enforce_admission_or_block`'s resolution (SDDK_STATE_HOME >
> XDG_STATE_HOME > ~/.local/state).

Escribe *"used for ledger access"*, y luego resuelve una precedencia que el
camino real del ledger no comparte.

## Consecuencia observada

Al verificar end-to-end que `MIGRATION_21` arreglaba `sddk cycle pause`
(session-15), se ejecutó el comando real contra una **copia** del ledger
con `SDDK_STATE_HOME=/tmp/m21v3` redirigido, sobre la premisa —afirmada
dos veces en las justificaciones de la herramienta— de que la base real
quedaba intacta.

No lo estaba. El ledger real
(`~/.local/state/sddk/projects/p-63676b11dc0ef88f/ledger.sqlite`) migró
v20 → v21 y el ciclo `c0-t01-pointer-mutation` quedó en `PAUSED` con la
lease liberada (`cycle_leases` 31 → 30). La copia en `/tmp` nunca fue
escrita.

Un segundo factor agrava: aunque `XDG_STATE_HOME` hubiera funcionado, `resolve_cycle_context_with_cwd` (`cycle.rs:215-221`) infiere
`--root` subiendo desde el CWD buscando un marcador de proyecto, y
`--cycle` explícito hace que ese camino gane. Dos mecanismos
independientes apuntaban al repo real.

**No hubo pérdida de datos**: 1.162/1.162 filas preservadas, 0 violaciones
FK, sin tablas temporales. Pero un comando de verificación escribió en el
entorno de producción del operador porque el mecanismo de aislamiento no
falló: no dio ninguna señal de que estuviera escribiendo donde no debía.

## Por qué es `critical`

No por el impacto de esta sesión concreta, sino por la clase: **una
variable de entorno documentada como aislamiento que no aísla nada**. Un
operador, un test o un agente que la use para explorar con seguridad
escribirá en la base real, y el CLI no dará ninguna señal: el comando
funciona, devuelve éxito, y la escritura ya ocurrió.

Agravantes:

- La doc de `admission.rs` afirma la precedencia equivocada, así que el
  error se propaga a quien lea el código para entender el comportamiento.
- Es exactamente el fallo que vuelve peligrosa cualquier verificación
  end-to-end contra un ledger existente, que es el modo de verificación
  natural de una migración de esquema.
- No hay test que cubra el contrato, porque el contrato escrito no
  coincide con el implementado.

## Alcance medido

- `resolve_state_dir` (`admission.rs:189`) tiene **1** caller
  (`admission.rs:317`, `admit_governed`), o sea que la variable solo
  influye en el circuito de aprobación.
- `XdgEnvironment.state_home` tiene **1** constructor en la CLI
  (`lib.rs:726`), y lee solo `XDG_STATE_HOME`.
- `SDDK_STATE_HOME` no aparece en `crates/sddk-engine/src/paths.rs`.

## Recomendación

1. **Una sola fuente de verdad.** Que `resolve_xdg_paths` acepte
   `SDDK_STATE_HOME` con precedencia sobre `XDG_STATE_HOME`, y borrar
   `resolve_state_dir` de `admission.rs` para que aprobación y acceso a
   datos no puedan divergir de nuevo. Dos resolvers distintos para la
   misma ruta es la causa raíz, no un síntoma.
2. **Fallar, no mentir.** Si `--root`/`--cycle` resuelven a un ledger
   existente mientras el operador cree haber redirigido el estado, el CLI
   debería decirlo. Un aviso cuando `--cycle` explícito gana sobre el
   root inferido evitaría la sorpresa.
3. **Test de contrato.** Un test que fije la precedencia
   `SDDK_STATE_HOME` > `XDG_STATE_HOME` > `HOME` leyendo el ledger
   resultante, con mutación en ambos sentidos (como
   `test_release_ci_manifest_anchor.sh`, que extrae el patrón real en vez
   de copiarlo).

## Nota sobre la Severidad

`critical` por la definición de [SEVERITY.md](./SEVERITY.md) — *"breaches
security boundary"* aplicado al límite operador/producción. Si se
considera que la frontera es solo de datos, `high` sería defendible; lo
dejo en `critical` porque el modo de fallo (escritura silenciosa en
producción creyendo estar aislado) es el peor disponible para una
herramienta de gobernanza.

## Estado

Abierto. No corregido en esta sesión: el commit `b0371571` (MIGRATION_21)
se limita al esquema y no toca la resolución de rutas. Recuperar el
ledger real es una decisión del operador.

---

## RESOLUCIÓN (session-42, 2026-09-29)

**Status: closed.** La causa raíz fue corregida en el commit `e62da1bc`
(`fix(engine): SDDK_STATE_HOME gobierna la ruta real del ledger`), que ya
formaba parte de la historia local pero **nunca cerró este documento**. La
deuda llevaba session-15 y session-35 marcadas como `open`/`critical`
pcuando el arreglo estaba escrito, commiteado y con sus tests.

### Causa raíz corregida

`XdgEnvironment` no transportaba `SDDK_STATE_HOME`, así que
`resolve_xdg_paths` resolvía la ruta del ledger desde `XDG_STATE_HOME`
únicamente. El fix añadió el campo y le dio precedencia:

```rust
// crates/sddk-engine/src/paths.rs
let state_home = resolve_base(
-   environment.state_home.as_deref(),
+   environment
+       .sddk_state_home
+       .as_deref()
+       .or(environment.state_home.as_deref()),
    environment.home.as_deref(),
    ".local/state",
    dirs::state_dir(),
)?;
```

La precedencia real pasa a ser la que `admission.rs` documentaba:
`SDDK_STATE_HOME` > `XDG_STATE_HOME` > `HOME`/`.local/state`.

### Evidencia de resolución (observada, session-42)

**1. Tests de contrato — 6/6 PASS.**

```
$ cargo test -p sddk-engine --test inc_debt_037_state_home_precedence
test relative_sddk_state_home_is_rejected ... ok
test sddk_state_home_redirects_the_ledger_entirely ... ok
test home_default_applies_when_no_state_override ... ok
test sddk_state_home_takes_precedence_over_xdg_state_home ... ok
test sddk_state_home_does_not_move_the_data_root ... ok
test xdg_state_home_still_wins_when_sddk_state_home_absent ... ok
test result: ok. 6 passed; 0 failed
```

**2. Aislamiento extremo a extremo — observado con el binario release.**
`tests/uat_ctx_003_durable_deltas.sh` ejecuta `adopt apply` con
`SDDK_STATE_HOME` propio, obtiene un `project_id` **distinto** del del
repo (`p-8d17246c…` vs `p-63676b11dc0ef88f`) y luego opera sobre **otro**
ledger. Si `SDDK_STATE_HOME` se ignorase, el ledger real habría cambiado y
el `project_id` habría coincidido. No coincidió. El aislamiento que esta
deuda decía roto está operativo.

### Consecuencia sobre el incidente original

La escritura de session-15 sobre el ledger real (`user_version` 20→21,
ciclo `c0-t01-pointer-mutation` OPEN→PAUSED) **queda explicada** por la
causa raíz ya corregada. Sin pérdida de datos (1.162/1.162 filas, 0
violaciones FK), con backup en `/tmp/ledger.pre-m21.backup.sqlite`. No
requiere acción de recuperación adicional: es historia Closed, no un
pendiente de operador.

### Nota de método (relevante, no cosmética)

`severity_at_detection` se conserva en el frontmatter en lugar de
borrarse. La severidad `critical` **era** correcta en session-15 y explica
por qué merece un documento permanente. Reescribirla a `low` porque hoy
está arreglado destruiría el registro de por qué el aislamiento se tomó en
serio. Lo que cambia es `status`, no la memoria del incidente.

### Deuda residual, distinta y no cubierta por este cierre

Este cierre **no** cubre dos problemas que la propia INC menciona como
agravantes y que siguen abiertos:

1. `resolve_cycle_context_with_cwd` infiere `--root` subiendo desde el CWD
   buscando un marcador de proyecto, y un `--cycle` explícito hace que ese
   camino gane. Es un **segundo mecanismo** de escape del aislamiento,
   independiente de `SDDK_STATE_HOME`. `tests/test_real_path_escape.sh` lo
   cubre parcialmente, pero el vector combinado
   (`SDDK_STATE_HOME` aísla + `--root` por CWD) no tiene cobertura de
   contrato.
2. `admission.rs:186-188` seguía afirmando una precedencia que ya no era
   certain; el fix de `paths.rs` la hizo cierta, pero el texto normativo
   no se revalidó contra el comportamiento nuevo.

Ninguno de los dos es `critical` por sí mismo, y ninguno bloquea trabajo
actual. Se anotan aquí para que no se pierdan en el cierre.
