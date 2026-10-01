---
id: INC-DEBT-049
title: "La re-adopción reasignó el project_id del repo y `adopt status` sigue reportando `complete` sobre un storage vacío, sin declarar que 65 ciclos y 3.9 MB de ledger viven bajo la identidad anterior"
status: open
severity: high
priority: P1
detected_at: 2026-10-01
detected_in_session: session-62
component: identity
surface: crates/sddk-cli/src/adopt.rs
cluster_id: CL-IDENTITY
related: [INC-DEBT-028, INC-DEBT-037]
references:
  - crates/sddk-cli/src/adopt.rs
  - crates/sddk-cli/src/cycle.rs
  - crates/sddk-domain/src/identity.rs
  - docs/debt/INC-DEBT-028-NONDETERMINISTIC-FALLBACK-IDENTITY.md
  - tests/cycle-artifacts/p-63676b11dc0ef88f/session62-c3m2-knowledge-basis-revise-identity/RECEIPT.md
fingerprint: "readoption_reassigns_project_id_and_adopt_status_reports_complete_over_empty_storage"
---

## Qué es

Este repo tiene **dos identidades de proyecto vivas**, y la que resuelve el CLI
es la que **no tiene historia**.

`sddk adopt status` responde **`status: complete`**. Ese `complete` es un **PASS
falso**: describe un receipt de adopción válido sobre un storage que no ve nada
de los 65 ciclos del proyecto.

## Evidencia (OBSERVED, session-62, sobre esta máquina)

**La identidad que el CLI resuelve hoy:**

```text
$ sddk adopt status --root . --scope .
status: complete
project_id: p-995939af668a53d8
ledger: /home/rubentxu/.local/state/sddk/projects/p-995939af668a53d8/ledger.sqlite
```

**Y aun así el ciclo no existe para el CLI:**

```text
$ sddk cycle status
error: no active cycle found for project p-995939af668a53d8
```

**El historial real, bajo la identidad anterior:**

```text
$ ls -1d .sddk/cycles/p-63676b11dc0ef88f* | wc -l
65
$ stat -c%s ~/.local/state/sddk/projects/p-63676b11dc0ef88f/ledger.sqlite
3911680
```

**Volumen comparado (medido, no estimado):**

| | `p-63676b11dc0ef88f` (histórico) | `p-995939af668a53d8` (resuelto hoy) |
|---|---|---|
| ledger | 3.911.680 B · 173 refs a eventos | 380.928 B · 6 refs a eventos |
| ciclos en `.sddk/cycles/` | **65** | 0 |
| `cycle-artifacts` en storage | 8 | 0 |

**La causa, con fecha y actor, está en el propio receipt de adopción:**

```json
{
  "project_id": "p-995939af668a53d8",
  "identity_source": "remote",
  "remote_url": "https://github.com/rubentxu/software-development-decision-kernel",
  "scope": ".",
  "canonical_workspace_path": "/var/mnt/DiscoChino2-fast/Proyectos/agentesIA/sddk-framework",
  "timestamp": "2026-09-30T19:29:34Z",
  "actor": "rubentxu"
}
```

El `project_id` se deriva de `hash(remote normalizado, scope)`
(`stable_project_id`, `crates/sddk-domain/src/identity.rs:404`). Cambió el
remote —o se adoptó con el remote canónico por primera vez— y con él cambió la
identidad. Reproducido: `stable_project_id("https://github.com/rubentxu/software-development-decision-kernel", ".")`
devuelve exactamente `p-995939af668a53d8`; ninguna combinación de remote ni de
scope razonable reproduce `p-63676b11dc0ef88f` (probado también contra el
fallback seed por ruta antigua y actual).

**El CLI no tiene detección de identidad previa distinta.** No hay una sola
coincidencia de "otra `project_id` con el mismo proyecto" en `crates/sddk-cli/src/`.

## Por qué es un PASS falso y no una molestia

`status: complete` es exactamente la clase de afirmación que este repo ya
trató como severa dos veces: **un artefacto que declara algo que no es**. La
misma forma que C3l.7 (un gate que certificaba conformidad sin ejecutarse) y
que INC-DEBT-047 (un CHANGELOG que describía otra cosa).

Consecuencia operativa sobre la premisa del proyecto —*SDDK es la autoridad
exclusiva del estado operativo*—: **la autoridad no ve su propia historia**.
Cualquier reanudación que confíe en `sddk cycle status` arranca desde un storage
vacío con un `complete` que la invite a no mirar atrás. Las 65 filas de
`.sddk/cycles/` y los 3.9 MB de ledger siguen en disco; lo que se perdió es el
**acceso**, no los datos.

## Agravante: dos identidades, un vault

Los dos proyectos declaran el mismo nombre y **el mismo vault**:

```text
p-63676b11dc0ef88f → vault_path: /home/rubentxu/.sddk-knowledge/sddk-framework
p-995939af668a53d8 → vault_path: /home/rubentxu/.sddk-knowledge/sddk-framework
```

Dos identidades escribiendo sobre el mismo vault viola el principio de una
autoridad canónica por concepto (AGENTS.md §2.7): ahora el vault es un destino
sin dueño declarado.

## Lo que NO es esta incidencia (matices honestos)

- **El código no cambió la identidad.** El `project_id` sigue siendo una
  derivación pura y determinista de remote + scope. No hay regresión de
  determinismo: el fix de INC-DEBT-028 sigue vigente.
- **La causa inmediata fue una acción humana explícita** (`actor: rubentxu`,
  timestamp propio). Re-adoptar un repo puede ser correcto; lo que no es
  correcto es que hacerlo **cargue en silencio** la historia anterior.
- **No se ha perdido nada.** Los dos storages están íntegros en disco.
- **No bloquea el trabajo activo**, que usa `tests/cycle-artifacts/` +
  `docs/roadmap/`, no el CLI de cycle. El impacto es latente, no inmediato.

## Qué es defecto del repo, y qué es reméd local

**Remedio local (operativo, sin tocar código).** El CLI ya trae la primitiva
exacta para esto — `IdentitySource::Pinned` y `sddk project pin`, cuyo `--reason`
documenta literalmente *"remote renamed, case drift, monorepo split"*:

```bash
sddk project pin --root . --project-id p-63676b11dc0ef88f \
  --reason "remote renombrado: la identidad historica conserva 65 ciclos y 3.9 MB de ledger"
```

Esto **no se ejecuta sin que el operador lo autorice**: cambiar la identidad
autoritativa del repo es una decisión de gobernanza, no una corrección de una
slice. Se deja escrito y sin aplicar.

**Defecto del repo (esto sí es código).** Ni `adopt status` ni `cycle status`
declaran que existe historial bajo otra identidad. Un repo re-adoptado debería,
como mínimo, **advertir**:

- que existe otro `project_id` con el mismo `project_name`/`vault_path`;
- cuántos ciclos y qué tamaño de ledger quedan fuera de la vista;
- y `sddk project pin` como salida.

## Por qué no se corrige en una slice de código sin decisión previa

Toca un **contrato de estado**: si `adopt status` pasa a reportar algo distinto
de `complete`, hay que decidir si es un estado nuevo (`complete_with_orphaned_history`),
si `complete` pasa a ser un warning, o si la advertencia va sólo a `cycle status`.
Eso cambia lo que otros consumidores y skills (`sddk-cycle-resume`,
`sddk-debt-verify`) pueden asumir, así que necesita SCOPE y ADRs, no una slice.

## Falsificadores exigidos cuando se implemente

1. **F49** — storage con el mismo `project_name` bajo otro `project_id` y `adopt
   status` **no** lo menciona ⇒ falla.
2. **F50** — el mismo escenario y `sddk cycle status` no cuenta los ciclos
   huérfanos ⇒ falla.
3. **F51** — dos proyectos con distinto `vault_path` ⇒ la advertencia **no**
   dispara (evita el falso positivo de "mismo nombre, distinto proyecto").
4. **F52** — proyecto recién adoptado sin historia previa ⇒ la salida **no**
   cambia respecto a la actual (sin regresión de ruido para el caso normal).

## Conocimiento negativo útil

No se pudo reproducir `p-63676b11dc0ef88f` desde ningún remote ni scope probado,
ni desde el fallback seed de las rutas antigua y actual. La afirmación de
INC-DEBT-028 línea 108 —*"un proyecto remoto (como `sddk-framework`,
`p-63676b11dc0ef88f`) no se ve afectado: su identidad viene del remote"*— es
**inverificable hoy**: ese id ya no se deriva del remote actual. No se corrige
aquí porque el documento es historia verificada de session-56 y esta observación
es de session-62; queda consignada, no reescrita.
