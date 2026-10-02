---
id: INC-DEBT-062
title: "`ledger export` escribe N eventos a un fichero y no declara cuántos existían: la misma clase que F63, en la cuarta superficie"
status: open
severity: high
priority: P1
fingerprint: "ledger_export_truncation_undeclared"
fingerprint_aliases: []
cluster_id: CL-LEDGER
created: 2026-10-03
created_by: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
owner: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
detected_at: 2026-10-02
detected_in_session: session-69l
component: ledger
surface: crates/sddk-cli/src/ledger.rs (versionado)
related: [INC-DEBT-060, INC-DEBT-061]
references:
  - crates/sddk-cli/src/ledger.rs
  - crates/sddk-cli/tests/ledger_events_declaration.rs
  - crates/sddk-cli/tests/ledger_watch_declaration.rs
  - docs/roadmap/receipts/cl-ledger-watch-total/RECEIPT.md
---

## Qué es

`ledger export --limit 5` escribe **5** eventos a un fichero y responde
`exported 5 events to /ruta/export.jsonl`, con **exit 0**. Sobre el ledger real
de `p-63676b11dc0ef88f` hay **598** eventos: los 593 que quedaron fuera no se
mencionan en ninguna parte, y el fichero producido se presenta como «el
export».

Es **la misma clase** que INC-DEBT-060 / F63, y el caso de F63 por construcción:
`run_ledger_export` (`ledger.rs:497-525`) toma `limit` de un vector que **ya
contiene todos** los eventos —`all_events` viene de `list_events()`,
`list_cycle_events()` o `list_frame_events()`— y hace `.take(limit)`. El largo
está en la mano y se tira, igual que en `ledger events` antes de que F63 lo
arreglara.

## Lo que NO se afirma

Que `export` deba escribir todo el ledger. `--limit` es una petición legítima, y
`--limit 0` significa «todos» desde siempre. **El defecto es no declarar que el
límite dejó algo fuera**, exactamente igual que en `ledger events` y
`ledger watch`.

## Por qué es peor que en las otras dos superficies

Las otras dos truncan **en pantalla**: quien lee el resultado ve que hay un
límite. `export` trunca **en un fichero**. El resultado es un artefacto que
parece completo y que otro proceso puede consumir sin ninguna señal de que le
falta el 99 %.

## Gravedad

**high**, no `critical`, por el mismo razonamiento que bajó a `high` a
INC-DEBT-060: **no hay pérdida de datos**. El ledger está íntegto, y el rodeo es
usar `ledger events`, que ya declara. El daño es de visibilidad por el producto,
y el artefacto escrito se puede contrastar con `ledger events --limit 0`.

## Medición

`/var/home/rubentxu/f63/03-medir-watch.py`, contra una copia del ledger real
(el mtime del original se comprueba antes y después):

```
OK  : `ledger export --limit 5` escribe cinco eventos y sale
      -- exit=0 escritos=5 dice=['exported 5 events to /tmp/.../export.jsonl']
GAP : `ledger export` declara cuantos eventos existian en total
      -- numeros que declara=[5], total real=598
```

## Estado

**No se arregla aquí.** `cl-ledger-watch-total` cierra `ledger watch`; una
concernia por ciclo. La superficie y el remedio ya están medidos, y el patrón
aplicado en `ledger watch` es directamente transportable: `LedgerExportOutput`
gana `total_events` y `pending`, derivados de `all_events.len()` **antes** del
`take`, que es donde el vector ya los tiene.

Cerrado por: —
Cierre previsto: ciclo siguiente a `cl-ledger-watch-total`.
