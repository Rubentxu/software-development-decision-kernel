---
id: INC-DEBT-028-NONDETERMINISTIC-FALLBACK-IDENTITY
title: "El project_id de un workspace sin remote cambia en cada invocación, y adopt status lo lee como no adoptado"
status: resolved
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-29 (OBSERVED, reproducción en vivo sobre repos reales)
fixed_by: session-30
cluster_id: CL-IDENTITY
related: [INC-DEBT-021]
fingerprint: "fallback_seed_uuid_v4_non_deterministic_project_id"
---

## Qué es

Un proyecto **sin `git remote`** no tiene identidad derivable del remoto, así
que el CLI cae al `fallback_seed`. Ese seed se acuñaba con
`Uuid::new_v4()` en cada invocación, y como `project_id` es
`hash(seed, scope)`, **cada comando reportaba un proyecto distinto**.

Observado en la sesión-29 sobre 3 repos reales sin remote
(`agent-workflows`, `conversational-games-studio`, `hodei-flow`): el mismo
directorio devolvía 3 `project_id` distintos según la llamada, y
`adopt plan` un cuarto valor:

```text
$ sddk project resolve --root hodei-flow --scope .   # 1
project_id: p-945c1fb4b49a2662
$ sddk project resolve --root hodei-flow --scope .   # 2, mismo comando
project_id: p-d1950ab154cc8fd6
```

El síntoma reportado por el operador —"vuelvo al proyecto y consta como no
adoptado"— es la consecuencia operativa: `adopt apply` escribía un recibo con
un seed que ninguna invocación posterior podía redescubrir, así que
`find_persisted_fallback_seed` no lo encontraba y `adopt status` terminaba en
`status: absent` (o, antes del arreglo, en error
`fallback seed is required because no remote or matching adoption receipt
exists`).

## Causa raíz

Cuatro puntos de acuñación, todos con `Uuid::new_v4()`:

| Sitio | Consumer |
|---|---|
| `crates/sddk-cli/src/cycle.rs:440` | `RuntimeContext::open` |
| `crates/sddk-cli/src/lib.rs` (`resolve_project_ids`) | IDs de proyecto |
| `crates/sddk-cli/src/lib.rs` (`run_project_resolve`) | `sddk project resolve` |
| `crates/sddk-cli/src/lib.rs` (`prepare_adoption_plan`) | `sddk adopt plan/apply/status/...` |

El hash **no** era el problema: `stable_fallback_project_id` es puro y
determinista. Con `--fallback-seed` fijo la identidad es estable. El defecto
era la fuente del seed.

## Solución

Nuevo primitivo en el dominio, `sddk_domain::stable_fallback_seed(path)`, que
deriva el seed del **path canónico** con `framed_hash` (mismo algoritmo de
longitud prefijada que el resto de primitivas de identidad, así que es
independiente de plataforma). Los 4 sitios lo usan; `--fallback-seed`
explícito conserva precedencia.

De paso, `adopt status|repair|refresh` ya no abortan cuando no hay remote ni
recibo: derivan la misma identidad que habría escrito `adopt apply`, que es
lo que hace que un workspace recién adoptado se lea `complete` al volver.

El string de dominio `sddk.project.fallback.seed.v1` es distinto del
`sddk.project.fallback.v1` (que hashea el seed), así que las dos derivaciones
no pueden colisionar. Está **pinado por golden value** en
`fallback_seed_is_pinned_to_known_value`: cambiar el dominio reasignaría
silenciosamente el `project_id` de todo proyecto sin remote, y ningún test
estructural lo detectaría.

## Evidencia (OBSERVED, session-30)

- 6 tests unitarios del dominio + 1 test end-to-end del CLI
  (`remote_less_workspace_keeps_one_identity_across_commands`) que cubre el
  síntoma reportado: resolve estable, `status: absent` antes, `apply`, y
  al volver `status: complete` con el **mismo** `project_id`.
- **3 mutaciones**, cada una detectada por el gate que la posee:
  - M1 seed aleatorio → `fallback_seed_is_deterministic_for_the_same_path` falla.
  - M2 dominio colisionado → el golden pin falla.
  - M3 nibble de versión pegado en vez de reemplazar (37 chars) → el test de UUID parseable falla.
  - M4 (CLI) volver a `Uuid::new_v4` → el test end-to-end falla.
- `cargo test -p sddk-domain -p sddk-cli`: **2185 passed / 0 failed / 7 ignored**.
- `cargo fmt --check` limpio, `cargo clippy -p sddk-domain -p sddk-cli
  --all-targets` sin avisos.

## Dos errores propios durante el arreglo (registrados, no ocultos)

1. **`framed_hash` devuelve 64 hex, no 32.** Corté a 32, pero calculé los
   grupos como si el hash fuera de 32, produciendo un seed de 37 caracteres.
   Lo detectaron los tests (`fallback seed must be a valid UUID`), no yo.
2. **El nibble de versión debe *reemplazar* `hex[16]`, no pegarse delante.**
   Pegarlo delante deja 37 chars. El `format!("{}-{}-{}-5{}-{}", ...)` es
   engañosamente correcto en apariencia.

En ambos casos afirmé que estaba arreglado antes de ejecutar el test. La
verificación fue lo que corrigió el error, no la lectura del código.

## Impacto y alcance del cambio de identidad

Este fix **reasigna el `project_id` de todo proyecto sin remote que no tenga
recibo persistido**. Consecuencias honestas:

- Un proyecto remoto (como `sddk-framework`, `p-63676b11dc0ef88f`) **no se
  ve afectado**: su identidad viene del remote y nunca entra por el fallback.
- Un proyecto remoto-less ya adoptado conserva su identidad, porque
  `find_persisted_fallback_seed` tiene prioridad sobre la derivación.
- Solo cambia para workspaces remoto-less sin recibo, que antes eran
  irrecuperables (identidad distinta en cada comando).

Queda pendiente una decisión de política: la máquina tiene **236** directorios
en `~/.local/share/sddk/projects` y **298** en `~/.local/state/sddk/projects`.
Parte son residuos de este bug, pero **no se ha verificado** cuantos ni cuales,
así que no se afirma aquí. Borrarlos es destructivo y requiere al operador.

## Reverificación session-56 y normalización del status

Este documento declaraba `status: fixed`, un valor **fuera del vocabulario
canónico** del corpus (que es `open` / `resolved` / `closed`: 7 / 9 / 60 sobre
77 ficheros). `fixed` aparecía **una sola vez** en las 77. Cualquier herramienta
que filtre por el vocabulario canónico lo habría tratado como no-cerrada.

Antes de tocarlo se verificaron sus criterios, porque una alerta de deuda cuyos
criterios han caducado no es deuda real. **OBSERVED en session-56, sobre un
repositorio sin remote, tres invocaciones del mismo comando:**

```text
$ sddk project resolve --root <probe> --scope .   # 1
project_id: p-aa6761df127e827d
$ sddk project resolve --root <probe> --scope .   # 2, mismo comando
project_id: p-aa6761df127e827d
$ sddk project resolve --root <probe> --scope .   # 3
project_id: p-aa6761df127e827d

$ sddk adopt status --root <probe> --scope .
status: absent
project_id: p-aa6761df127e827d     ← coherente con resolve
```

`project_id` y `workspace_id` estables, e `identity_source: fallback` con
`remote_url: null`. **El fix de session-30 sigue vigente**: los criterios de
esta incidencia han caducado, luego no es deuda abierta. Se normaliza
`fixed → resolved` para devolver el fichero al vocabulario canónico; el
cambio de valor **no** es una afirmación nueva, es la constancia de una
verificación ya hecha.
