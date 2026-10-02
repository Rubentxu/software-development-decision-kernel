# EXPLORATION-REPORT — cl-ledger-export-total

**Cycle:** `p-63676b11dc0ef88f/ledger-export-total` (real, `OPEN/explore`)
**Date:** 2026-10-02T22:35:24Z
**Baseline:** `866699ec` (`docs(roadmap): el ciclo esta en RELEASE_PENDING, no cerrado`), `HEAD == origin/main`
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**Origen:** INC-DEBT-062, registrada al cierre de `cl-ledger-watch-total`, que
la dejó **medida** y con la superficie **leída** en vez de cerrada

---

## §0 — La medición

`/var/home/rubentxu/f63/09-medir-export.py`, contra una **copia** del ledger
real (mtime del original comprobado antes y después).

```
OK  : el total real se puede leer de `ledger events` -- exit=0 total_events=600
OK  : `ledger export --limit 5` escribe cinco eventos y sale con 0
      -- exit=0 escritos=5 dice='exported 5 events to /tmp/.../export.jsonl'
GAP : `ledger export` declara cuantos eventos existian en total
      -- numeros que declara=[5], total real=600, frase='exported 5 events'
GAP : `ledger export` tiene una salida que una maquina pueda leer
      -- con --format json -> exit=2: "error: unexpected argument '--format' found"
OK  : la forma declarada (ExportOutput) existe y NO es la que esta en vigor
      -- deriva Serialize=True, el RESUMEN se serializa=False
```

## §1 — Qué es, y qué NO es

`ledger export --limit 5` escribe 5 eventos y responde `exported 5 events to
<path>`, con **exit 0**. Sobre 600 eventos, los **595** que no se escribieron no
se mencionan en ninguna parte, y el fichero producido se presenta como «el
export».

Es F63 por construcción: `run_ledger_export` recibe `all_events` de
`list_events()` / `list_cycle_events()` / `list_frame_events()` —**el vector
completo de la consulta**— y hace `.take(limit)` (`ledger.rs:493-497`). El largo
está en la mano y se tira. Igual que en `ledger events` antes de que F63 lo
arreglara, e igual que en `ledger watch` ayer.

**Lo que NO se afirma.** Que `export` deba escribir todo el ledger: `--limit` es
una petición legítima y `--limit 0` significa «todos» desde siempre. **El
defecto es no declarar que el límite dejó algo fuera**, igual que en las otras
dos superficies.

## §2 — Lo que hace este caso peor que los otros dos

Las otras dos truncan **en pantalla**: quien lee el resultado ve que hay un
límite. `export` trunca **en un fichero**. El resultado es un artefacto que
**parece completo** y que otro proceso puede consumir sin ninguna señal de que
le falta el 99 %. Es la diferencia entre un texto que miente y un fichero que
miente.

## §3 — El hallazgo que la medición de la forma destapó

`ExportOutput` (`ledger.rs:537-542`) **deriva `Serialize`** y **nunca se
serializa**. El camino de éxito construye el struct, lee `.count` y `.path`, y
pinta el texto con un `format!` a mano (`ledger.rs:519-528`).

Es decir: **existe una declaración de la forma que este comando podría tener, y
no es la que está en vigor.** Es exactamente la forma que encontraron
`GraphExport` (69i) y `export_node` (69k), y por eso se midió antes de escribir
el remedio: el remedio puede ser **cablear la forma que ya existe** o **añadir
una segunda**, y son opuestos. Añadir un `json!` nuevo al lado de un struct
dead sería repetir, en el arreglo, el defecto que se viene a cerrar.

**Y `--format` no existe en `ledger export`**, medido con `exit: 2`. Las otras
dos superficies de lectura lo tienen. No es una omisión menor: **una máquina no
puede leer la declaración de `export` porque no existe forma de leerla.**

## §4 — La auditoría por criterio, y por qué no se repite

La pregunta del ciclo anterior era *«¿qué más trunca, y quién lo declara?»*, y
dio 5 candidatas → 2 defectos (`watch`, `export`), con `cycle list` y
`telemetry status` ya correctos y `cockpit diff-watch` descartado por lectura.

**No se repite aquí**, y esa es la decisión: la lista de candidatos de «truncan
en silencio» está **agotada y medida**. Repetirla sería trabajo por analogía. Lo
que queda por debajo de esta superficie es otra pregunta —*¿qué más escribe un
artefacto que parece completo?*— y esa no se responde aquí.

## §5 — Lo que la medición evitó

**El detector de la forma mente dos veces seguidas antes de acertar.** La
primera versión buscaba `render_result(result…)` en el **fichero entero** y
encontró el de **`ledger events`**, otro comando del mismo fichero: concluyó que
`export` serializaba. La segunda buscó `to_string` en el cuerpo de la función y
encontró el `to_string` de **cada evento** —que es el payload JSONL y es
exactamente lo que el comando debe hacer—, y concluyó lo mismo.

**Setima vez en esta sesión que un detector mide lo que tiene al lado**, y la
tercera vez que el error era **acotar la sonda a la unidad que se dice estar
midiendo**. La condición correcta —*¿se serializa el **resumen**?*— se
escribe nombrando el resumen, no la operación genérica.

## §6 — Lo que este ciclo NO hace

1. **NO** arregla `ledger watch`: cerrado y verificado ayer.
2. **NO** toca `ledger events` ni `ledger watch`: ya declaran.
3. **NO** cambia el payload JSONL del fichero exportado: una línea por evento,
   como ahora. El cambio es **solo en el resumen**.
4. **NO** cambia `--limit 0` (significa «todos») ni los filtros.
5. **NO** escribe ningún fichero fuera de un temporal durante la verificación.
