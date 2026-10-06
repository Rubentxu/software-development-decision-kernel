---
id: INC-DEBT-063
title: "Tres recibos declaran un cycle_id que SDDK nunca emitió: la documentación afirma un ciclo que la autoridad no tiene"
status: resolved
severity: medium
priority: P2
revalidated_at: 2026-10-03
revalidated_in_session: session-69s
fingerprint: "cycle_receipts_declare_nonexistent_cycle_id"
fingerprint_aliases: []
cluster_id: CL-DOCS
created: 2026-10-03
created_by: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
owner: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
detected_at: 2026-10-02
detected_in_session: session-69l
component: docs
surface: docs/roadmap/receipts/ (versionado)
related: [INC-DEBT-061, INC-DEBT-060]
references:
  - docs/roadmap/receipts/cl-vault-graph/RECEIPT.md
  - docs/roadmap/receipts/cl-vault-html-replica/RECEIPT.md
  - docs/roadmap/receipts/cl-vault-node-projection/RECEIPT.md
  - docs/roadmap/receipts/cl-ledger-watch-total/RECEIPT.md
---

> ## ⚠️ REVISIÓN DE VIGENCIA (2026-10-03, sesión 69s) — CONFIRMADA, y una conclusión publicada era falsa
>
> `REVALIDACION-R2.md` declaró esta deuda **`NOT_VERIFIABLE`** al no encontrar
> los tres recibos. **Ese veredicto era un error de directorio, no una propiedad
> del mundo, y esta deuda queda CONFIRMADA con la evidencia que le faltaba.**
>
> Los tres recibos **existen**, y en las rutas que la `references:` de este mismo
> frontmatter cita:
>
> ```
> docs/roadmap/receipts/cl-vault-graph/RECEIPT.md           10014 bytes
> docs/roadmap/receipts/cl-vault-html-replica/RECEIPT.md     5538 bytes
> docs/roadmap/receipts/cl-vault-node-projection/RECEIPT.md 11161 bytes
> ```
>
> Y sus tres `cycle_id` declarados siguen sin existir en la autoridad — **0 filas
> cada uno** en el ledger real. **Lo que el `NOT_VERIFIABLE` dejó sin medir era
> precisamente lo que la afirma.**
>
> **Cifras del cuerpo que están caducas:** el ledger no tiene «179 ciclos», tiene
> **187 filas que son dos poblaciones**. Las reales de `p-63676b11dc0ef88f` son
> **108**; las otras **79** son `__spine_import__` (todas `OPEN`, manifiesto `{}`).
> El `LIKE '%vault%'` de abajo daba 2 sobre las 187 filas; sobre las **108
> reales** da **2 también**, luego el ejemplo no se cae, pero su denominador
> estaba mal. La sección se conserva sin reescribir.
>
> `status: open`, severidad y prioridad **sin cambio** (`medium`/`P2`): la deuda
> no era más grave de lo que decía, era exactamente tan grave como decía.
>
> Evidencia: `docs/roadmap/receipts/c3m5-bounded-contexts/REVISION-VIGENCIA-R2.md` §0.1.

> ## ✅ CIERRE (2026-10-06, sesión 89) — RESUELTA, y el título mentía por partida doble
>
> **Ejecutada la salida 1** (enmendar los encabezados), que es la que esta deuda
> recomendaba y que quedó a la espera de decisión del operador. La autoridad no se
> tocó: **no se creó ningún ciclo**, que es la línea que esta deuda trazaba.
>
> **LA CIFRA DEL TÍTULO ERA 3 Y LA MEDICIÓN DABA 13.** Dos veces mal contada, y por
> dos motivos distintos que conviene no perder:
>
> 1. **No eran 3 recibos: eran 11 `cycle_id` completos inexistentes + 2 campos mal
>    etiquetados**, todos en documentos que **afirman resultado**
>    (`RECEIPT*.md`, `CLOSURE.md`):
>
>    | Bloque | Declaraba | Filas |
>    |---|---|---|
>    | `cl-ledger-declaration` | `…/ledger-declaration` | 0 |
>    | `cl-vault-declaration` | `…/vault-declaration` | 0 |
>    | `cl-vault-graph` | `…/vault-graph` | 0 |
>    | `cl-vault-html-replica` | `…/vault-html-replica` | 0 |
>    | `cl-vault-node-projection` | `…/vault-node-projection` | 0 |
>    | `cl-adopt-alias-wiring` | `…/identity-alias` | 0 |
>    | `cl-cycle-enumeration` | `…/cycle-enumeration` | 0 |
>    | `cl-release-version-source` | `…/version-source` | 0 |
>    | `cl-lockstep-gate-sees` | `…/version-coherence-070` | 0 |
>    | `cl-project-declares-its-version` | `…/version-coherence-071` | 0 |
>    | `cl-quien-responde-tambien-puede-preguntar` | `…/version-coherence-072` | 0 |
>    | `c3h`, `c4-pre-flight` | `**Cycle:** C3h` / `C4` — un hito, no un id | — |
>
>    Y las `references:` del frontmatter de esta deuda citaban **una receipt
>    válida**: `cl-ledger-watch-total` declara `version-coherence-068`, que
>    **sí** existe en el ledger. Una referencia que sostiene la prueba de la
>    deuda y que no la sostiene.
>
> 2. **Contar mal también fue el primer defecto del guard, en su propia primera
>    ejecución.** Dos, de hecho: `STATE_BASE` sin exportar al probe, y los ids
>    leídos de `stdin` cuando el heredoc ya ocupaba `stdin` — el guard se
>    comprobaba a sí mismo y falló cerrado por su propia causa. Es la forma más
>    barata de la que este repo avisa: un gate que se pone rojo a sí mismo antes
>    de tener nada que decir.
>
> **LO QUE ESTE CIERRE NO DICE**
>
> - **No cierra la clase entera.** Los documentos de **intención** del mismo
>   directorio (`PRE-FLIGHT.md`, `SCOPE-CONTRACT.md`) declaran el mismo tipo de
>   ciclo inexistente: **16 + 11 ficheros** y **31** afirmaciones `**Cycle:** C3e`
>   sin id. Quedan medidos y con seguimiento en
>   [`INC-DEBT-063-FU-PLANNING-DOCUMENTS.md`](./INC-DEBT-063-FU-PLANNING-DOCUMENTS.md),
>   y el guard **no** los cubre — lo dice en su cabecera con los números, para que
>   el hueco sea visible y no heredado.
> - **No corrige ni un número de producción de esta deuda** más allá del título y
>   del estado. El cuerpo conserva sus «179 ciclos» porque ya está marcado como
>   caduco arriba; y de las dos poblaciones que nombraba esa revisión, la real
>   `p-63676b11dc0ef88f` contaba **108** ciclos en 2026-10-03.
>
> **GATES:** `tests/test_receipt_cycle_id_authority.sh` = `PASS=20 FAIL=0 SKIP=0`
> sobre 20 afirmaciones de ciclo en el alcance corregido · autofalsación
> `tests/test_receipt_cycle_id_authority_mutation.sh` = `PASS=4 FAIL=0 SKIP=0`
> (id fabricado en el campo nuevo, campo `**Cycle:**` sin id, fail-closed sin
> autoridad, control negativo) · `shellcheck` limpio en ambos.
>
> Evidencia: `docs/roadmap/receipts/cl-receipt-cycle-id-authority/RECEIPT.md`.

## Qué es

`cl-vault-graph`, `cl-vault-html-replica` y `cl-vault-node-projection` se
cerraron con un `**Cycle:** p-63676b11dc0ef88f/<nombre>` en su encabezado. **Ese
identificador no existe en la autoridad.** El ledger real de
`p-63676b11dc0ef88f` tiene 179 ciclos y **ninguno** de los tres está entre ellos.

Medido, no supuesto:

```
$ SELECT cycle_id FROM cycles WHERE cycle_id LIKE '%vault%'
p-63676b11dc0ef88f/vault-mirror-accepted-adrs
p-63676b11dc0ef88f/vault-mirror-auto
(solo 2 de 179)
```

Esos tres ciclos se llevaron a cabo **sin** `sddk cycle start`: existen como
documentos y no en la autoridad. `cl-ledger-watch-total` es el primero que se
abre de verdad, con `cycle.start` y sus gates evaluados con evidencia.

## Por qué importa

Un recibo es un registro de lo que se hizo y con qué evidencia. Si declara un
ciclo que no existió, quien lo lea **no puede comprobar nada**: `sddk cycle
status` de ese id responde que no hay ciclo, y no hay forma de distinguir «el
ciclo se cerró y se archivó» de «el ciclo nunca existió». El documento afirma una
cosa que la autoridad contradice, que es la forma exacta de la deuda que este
proyecto mide.

## Lo que NO se hace, y por qué

**No se crean los ciclos a posteriori.** Eso sería **escribir historia en la
autoridad**: inventar eventos retroactivos para cuadrar con un documento. Es el
mismo modo de fallo que INC-DEBT-061 (un identificador que no se puede corregir
redirigiendo), aplicado a los recibos propios, y es exactamente lo que esta
sesión lleva tres ciclos midiendo en el otro sentido.

## Gravedad

**medium**, no más alta: el daño es documental y **no afecta al runtime**. Nada
lee estos recibos como entrada; el rodeo es leerlos como lo que son —un informe
de trabajo con un encabezado equivocado— y enmendar el encabezado. No hay pérdida
de datos ni comportamiento incorrecto en el producto.

## Las dos salidas, y ninguna es «crear los ciclos»

1. **Enmendar los tres encabezados** para que declaren lo cierto: qué se hizo,
   con qué baseline y con qué evidencia, **sin** afirmar un `cycle_id` que la
   autoridad no emitió. Se recomienda esta: es additive y no toca la autoridad.
2. **Registrar la divergencia** como deuda permanente, con la autoridad como
   fuente de verdad y los recibos como documento histórico que mentía del ciclo.

Ambas son del **operador**: son documentos de ciclos cerrados, y reescribirlos es
otra concernia que este ciclo no se atribuye.

Cerrado por: —
Cierre previsto: decisión del operador.
