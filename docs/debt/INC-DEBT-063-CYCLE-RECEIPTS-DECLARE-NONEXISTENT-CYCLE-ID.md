---
id: INC-DEBT-063
title: "Tres recibos declaran un cycle_id que SDDK nunca emitió: la documentación afirma un ciclo que la autoridad no tiene"
status: open
severity: medium
priority: P2
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
