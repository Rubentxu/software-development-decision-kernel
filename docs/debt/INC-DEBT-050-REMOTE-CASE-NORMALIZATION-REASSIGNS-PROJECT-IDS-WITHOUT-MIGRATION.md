---
id: INC-DEBT-050
title: "Normalizar el remote a minúsculas reasignó el project_id de 25 adopciones (24%) y dejó sus ledgers huérfanos, sin migración y sin golden pin que lo detectara"
status: open
severity: critical
priority: P1
partially_resolved_at: 2026-10-01
partially_resolved_in_session: session-64
resolved_part: "golden pin del camino remote: stable_project_id y normalize_remote_url quedan con valores absolutos fijados por test (session-64)"
open_part: "migracion de los 25 receipts huerfanos: destructiva, requiere al operador"
detected_at: 2026-10-01
detected_in_session: session-63
component: identity
surface: crates/sddk-domain/src/identity.rs
cluster_id: CL-IDENTITY
related: [INC-DEBT-049, INC-DEBT-028]
references:
  - crates/sddk-domain/src/identity.rs
  - crates/sddk-cli/src/lib.rs
  - docs/debt/INC-DEBT-049-READOPTION-REASSIGNS-PROJECT-IDSILENTLY-ORPHANS-HISTORY.md
  - docs/debt/INC-DEBT-028-NONDETERMINISTIC-FALLBACK-IDENTITY.md
  - tests/cycle-artifacts/p-63676b11dc0ef88f/session63-pin-identity-authority/RECEIPT.md
fingerprint: "remote_case_normalization_reassigns_project_id_without_migration_or_golden_pin"
---

## Qué es

`project_id` es `hash(remote normalizado, scope)`. El commit `52182522`
(2026-09-30) añadió `normalize_remote_path` con minúsculas para que un cambio de
mayúsculas en la URL no bifurcara el ledger. **Pero cambiar el algoritmo de
normalización cambia el `project_id` de todo proyecto ya adoptado, y no hubo
migración.**

El propio subject del commit dice lo que el cambio comprobaba y lo que no:

> `fix(domain): normalizar case del path del remote — case-change ya no forkea el ledger (D2)`

Para un proyecto que adopta **después** del fix, es cierto. Para uno ya
adoptado **antes**, el mismo cambio **forKEA su ledger**: es exactamente la
propiedad que el commit afirma ofrecer.

## Reproducción (OBSERVED, session-63, esta máquina)

Mismo remote, misma scope, distinto `project_id` según la *caste* del owner:

```text
https://github.com/Rubentxu/software-development-decision-kernel   -> p-63676b11dc0ef88f
https://github.com/rubentxu/software-development-decision-kernel   -> p-995939af668a53d8
https://github.com/Rubentxu/software-development-decision-kernel.git -> p-83b01d24861af185
https://github.com/rubentxu/software-development-decision-kernel.git -> p-447c6f4fd496d161
```

Los dos primeros son **reales**, no sintéticos: son los dos `project_id` que
coexisten en esta máquina, con receipts que declaran **el mismo remote**:

| receipt | timestamp | remote declarado | project_id |
|---|---|---|---|
| workspaces/`w-2e7853…`/adoption.json | `2026-09-30T07:47:47Z` | `…/Rubentxu/software-development-decision-kernel` | `p-63676b11dc0ef88f` (**65 ciclos, 3.911.680 B de ledger**) |
| workspaces/`w-92344b…`/adoption.json | `2026-09-30T19:29:34Z` | `…/rubentxu/software-development-decision-kernel` | `p-995939af668a53d8` (**vacío**) |

**12 horas de distancia**, mismo día, mismo repo. El commit `52182522` entra
entre medias. El último receipt con la forma pre-normalización es de las
`09:07:15Z`, nueve horas antes del fix.

## Magnitud (medida sobre `~/.local/share/sddk/projects`, no estimada)

```text
receipts de adopcion revisados:                 104
ids que NO coinciden con la derivacion actual:    25   (24%)
```

25 receipts repartidos en **16 `project_id` distintos** sobre **13 repos
remotos**, todos con el owner `Rubentxu` en mayúscula:

```text
p-c1fac1fea05615c6  CogniCode                                p-f58d41952fdf56c1  arch-skillkit
p-52b95ef55999f9de  sddk-framework                           p-f4d8f28cd78d443e  sddk-hardness
p-38e02210a9f14317  arch-stack                               p-033dccc0fef1a91f  pipelinek-release-harness
p-07dc4abdd11cc9be  golem                                    p-74299cf88f51dab9  skillgraph
p-28fce7028ac3c497  bevy-2d-editor                           p-3416cfb8288f8964  chronos
p-4c8272c9e7dcdfa2  pipelattice                              p-8ccd53f319722656  asdf-pipelinek
p-7c4aff45199a2069  reactive-narrative-studio-specs          p-733fb505b5a6bd2d  pipeline-kotlin
p-63676b11dc0ef88f  software-development-decision-kernel
```

Un cuarto de las adopciones de esta máquina quedaron con su ledger fuera del
alcance del CLI. **Los datos no se pierden** — los storages siguen íntegros —
pero `sddk` resuelve la identidad nueva y opera sobre un storage vacío.

## Por qué no lo detectó nadie

INC-DEBT-028 ya establecía el principio para el camino del *fallback seed*:

> El string de dominio `sddk.project.fallback.seed.v1` está **pinado por golden
> value** en `fallback_seed_is_pinned_to_known_value`: cambiar el dominio
> reasignaría silenciosamente el `project_id` de todo proyecto sin remote, y
> **ningún test estructural lo detectaría**.

**Ese mismo razonamiento se aplica al camino del remote, y ahí no hay pin.**
`stable_project_id` usa el dominio `sddk.project.remote.v1` y **ningún test
fija su salida**: `grep 'p-[0-9a-f]\{16\}'` en `crates/sddk-domain` sólo
encuentra ids escritos a mano en fixtures y comentarios, ninguno como golden
value del derivador. Por eso una reasignación del 24% pasó sin ruido: no había
nada que la detectara.

Es la misma clase de defecto que INC-DEBT-028, un nivel más arriba y con más
alcance, y la misma lección aplicada a un caso que nadie cubrió.

## Relación con INC-DEBT-049

Session-63 registró INC-DEBT-049 con la hipótesis de que *"el remote había
cambiado"*. **Era incorrecta, y queda corregido aquí:** el remote es el mismo.
Lo que cambió fue el **código que lo normaliza**. INC-DEBT-049 describe el
síntoma en este repo; esta incidencia describe su causa y su alcance.

## Remedios

**1. El pin funciona y ya está aplicado en este repo.** Con el fix de
session-63 (`5b5e2d58`, que hace que las cinco vías del CLI honren el pin),
`sddk adopt status` y `sddk cycle status` ya reportan `p-63676b11dc0ef88f`.
Verificado ejecutando el binario compilado con el pin aplicado.

**2. Migración de los 25 receipts.** Requiere reescribir `project_id`,
`workspace_id` y las rutas derivadas de cada receipt, y mover los ledgers.
Es **destructivo y necesita al operador**: no se ejecuta desde un agente.

**3. El arreglo de fondo — HECHO en session-64.** `stable_project_id` y
`normalize_remote_url` tienen ahora valores absolutos fijados por test (tres
tests nuevos, falsificadores F53–F55 OBSERVED abajo). Cualquier cambio futuro
en el normalizador, en el dominio `sddk.project.remote.v1` o en el framing del
hash rompe un test en vez de reasignar silenciosamente el `project_id` de cada
proyecto del mundo. Es el mismo mecanismo que ya protegía el seed de fallback,
aplicado donde faltaba.

El comentario del golden pin dice explícitamente lo que **no** hay que hacer si
falla: copiar el valor nuevo. Eso reasignaría la identidad de todos los
proyectos con ese remote; lo correcto es decidir antes si procede migración.

**4. Regla de cambio:** cualquier modificación de `normalize_remote_url` o del
dominio `sddk.project.remote.v1` es **breaking change** y requiere migración o
pin, aunque parezca inocua. Sin esa regla, el mismo defecto se repite en el
próximo refactor del normalizador.

### Corrección de session-64: el "gate automático de CI" que propuse NO era deuda real

Esta incidencia, tal como se escribió en session-63, dejaba un cuarto remedy
pendiente: *"convertir la regla BREAKING CHANGE en un gate automático de CI"*.
**Se reevaluó y se retira: no era deuda real.**

Se había escrito sin verificar cómo funciona el pipeline. Verificado en
`.github/workflows/ci.yml`:

- `cargo test --workspace` **sí** se ejecuta en el workflow, así que el golden
  pin **ya corre automáticamente** con cada perfil completo. No hacía falta
  ningún mecanismo nuevo.
- Y el propio encabezado del workflow dice: *"MANUAL-ONLY (2026-08-10): SDDK
  never depends on CI/CD. Validation runs locally; cloud CI is an optional
  on-demand check via workflow_dispatch, **never a gate**"*.

Añadir un gate en la cloud habría sido redundante **y** contrario a la política
del repo, que por AGENTS.md §2.5 trata la cloud como evidencia asíncrona, nunca
como bloqueo. El gate autoritativo es el perfil local, y el golden pin ya está
dentro de él.

**Lección aplicada a sí misma:** es el mismo patrón que la regla del operador
enuncia — *alerta de deuda sin verificar si sus criterios siguen vigentes no es
deuda real*. Esta vez la alerta la escribí yo treinta minutos antes, en el
documento que estaba redactando. Lo que la deja cerrada no es una discusión de
gusto sino un `grep` en el pipeline.

**Queda una sola parte abierta, y es la que de verdad importa:** la migración de
los 25 receipts.

## Falsificadores OBSERVED (session-64) — el pin dorado está puesto y muerde

Tres tests nuevos en `crates/sddk-domain/src/identity.rs`:

| Test | Qué fija |
|---|---|
| `project_id_is_pinned_to_known_values` | valor absoluto de `stable_project_id` en 3 formas: scope raíz `"."`, owner en GitHub, subpath en GitLab |
| `remote_normalization_is_pinned_to_known_values` | salida exacta del normalizador: casse mixta, `.git`, puerto por defecto, puerto no-por-defecto, scp/ssh, credenciales + query + fragment |
| `case_normalization_reassigned_real_project_ids_without_migration` | los **dos ids reales** que convivieron en esta máquina, con sus valores exactos |

| # | Mutación | Resultado |
|---|---|---|
| **F53** | quitar el `.to_lowercase()` del path en `normalize_remote_path` | **OBSERVED** — `remote_normalization_is_pinned_to_known_values` FAILED: `left: "https://github.com/Acme/Widgets"` frente a `right: "https://github.com/acme/widgets"` |
| **F54** | dominio del hash `sddk.project.remote.v1` → `.v2` | **OBSERVED** — FALLAN `project_id_is_pinned_to_known_values` **y** `case_normalization_reassigned_real_project_ids_without_migration` |
| **F55** | invertir el framing (remote y scope intercambiados) | **OBSERVED** — FALLAN los mismos dos |

**El primer F53 diseñado estaba mal y se sustituyó antes de ejecutarlo.**
Proponía mutar `to_lowercase` → `to_ascii_lowercase`, que para entradas ASCII
produce **exactamente el mismo resultado**, así que el pin no habría fallado y
la prueba habría sido inútil. Un falsificador que no puede fallar no es un
falsificador. Las tres mutaciones finales cambian cada una el valor derivado, que
es la condición para que el pin signifique algo.

**Un valor de tener las capas separadas:** con F53 (el normalizador), el golden
del **hash** sigue verde, porque `stable_project_id` recibe el remote ya
normalizado. Con F54 y F55 (el hash), el golden del **normalizador** sigue
verde. Cada pin protege **su** capa y ninguno depende del otro. Ese reparto es
justo lo que hacía falta: el defecto original cruzaba las dos.

**El test histórico fija más de lo previsto:** con F54 y F55 falla también,
porque contiene los ids reales. Avisa de que un cambio en el dominio o en el
framing no es "otro hash": es exactamente la misma reasignación silenciosa de la
vez anterior.

## Límites declarados

- La medición es **de esta máquina**. No se afirma nada sobre otras; el
  mecanismo, eso sí, es independiente del entorno: depende sólo del código.
- **No se ejecutó ninguna migración.** El remedio 2 está escrito y no aplicado.
- `normalize_remote_path` normaliza el **path**, no el host: `github.com` ya
  venía en minúsculas del input, pero un host con mayúsculas seguiría
  reasignando. No verificado si algún remoto de esta máquina tiene esa forma.
