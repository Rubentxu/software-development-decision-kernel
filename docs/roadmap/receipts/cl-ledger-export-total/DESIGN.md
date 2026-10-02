# DESIGN — cl-ledger-export-total

**Cycle:** `p-63676b11dc0ef88f/ledger-export-total` (`OPEN/design`)
**Date:** 2026-10-02T22:35:24Z
**SCOPE:** `SCOPE-CONTRACT.md` · **PRE-FLIGHT:** `PRE-FLIGHT.md`

---

## §1 — La regla, y por qué no cuesta nada

El total es **`all_events.len()` antes del `.take(limit)`**, y `all_events` ya
es **la consulta entera con su filtro aplicado**: `run_ledger_export` elige
`list_cycle_events(cycle)` → `list_frame_events(frame)` → `list_events()`
(`ledger.rs:485-491`) **antes** de cortar.

Eso significa que **aquí no hay que construir nada**: no hace falta una segunda
consulta, ni un `count`, ni un predicado paralelo, ni una función de filtro
compartida. `ledger watch` necesitó `apply_watch_filters` porque allí el filtro
se aplica **dentro** del bucle, sobre un `list_events_after` acotado; `export`
selecciona la consulta entera de una vez, luego el total ya está en la mano.

**Y esa asimetría es la que hay que escribir**, porque es la que hace que el
remedio de aquí no sea el de ayer: copiar la solución de `watch` a `export`
habría añadido una segunda cuenta que no hace falta.

## §2 — La forma: cablear la que existe

`ExportOutput` **deriva `Serialize` y nunca se serializa** (`ledger.rs:537-542`).
El arreglo **no añade un `json!` al lado**: pone la forma declarada en vigor.

```rust
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ExportOutput {
    path: std::path::PathBuf,
    /// Cuántos eventos se ESCRIBIERON en el fichero.
    written: usize,
    /// Cuántos eventos tenía la consulta antes del límite. Derivado del vector
    /// que ya contenía la consulta entera, no de una cuenta aparte.
    total_events: usize,
}

impl ExportOutput {
    /// Derivado, nunca un tercer número escrito a mano.
    fn pending(&self) -> usize { self.total_events.saturating_sub(self.written) }
}
```

`count` se renombra a `written` **porque el struct nunca se ha serializado**:
no hay ningún consumidor del nombre, y junto a `total_events` un `count` es
ambiguo —«¿cuántos de los que existían?»—. Renombrar un campo de un tipo que
nunca cruzó un límite público no cambia ningún contrato; renombrar `count` en
`ReplayOutput` sí lo haría, y ese no se toca.

## §3 — Un solo pintor, dos formatos

El texto y el JSON son **dos declaraciones del mismo hecho**, y por eso los dos
salen del **mismo struct**:

- `export_text(&ExportOutput) -> String` para `--format text` (por defecto, y
  el comportamiento de hoy más la declaración)
- `serde_json::to_string(&ExportOutput)` para `--format json`

Es exactamente el cierre del ciclo de ayer (`LedgerWatchSummary` leído por los
dos renderizadores), y por el mismo motivo: si el texto escribiera sus números
a mano y el JSON los escribiera desde el struct, habría **dos reglas** que
pueden divergir, que es la forma del defecto que se viene a cerrar.

## §4 — `--format` es aditivo

`LedgerExportArgs` gana `format: OutputFormat` con `default_value_t =
OutputFormat::Text`. **El comportamiento por defecto no cambia** salvo por la
declaración añadida. Las otras dos superficies de lectura ya lo tienen, así que
esto también cierra una asimetría de la CLI que no estaba escrita en ninguna parte.

## §5 — El texto

```
exported 5 of 600 events (595 not written) to /ruta/export.jsonl
```

Misma frase, más la declaración. Cuando no queda nada fuera:

```
exported 3 of 3 events (0 not written) to /ruta/export.jsonl
```

**La declaración aparece también en el caso no interesante**, por la misma razón
que en R1 de F63 y en R1 de `watch`: una declaración que solo aparece al truncar
no se puede leer de un log donde no truncó, y un fichero exportado sin truncar es
precisamente el caso en el que nadie sospecha que falta la declaración.

## §6 — Lo que este diseño NO hace

| no | por qué |
|---|---|
| no cambia el payload JSONL | una línea por evento, orden ascendente, igual que hoy. El arreglo es del **resumen** (STOP 2) |
| no añade un `json!` paralelo | `ExportOutput` ya deriva `Serialize`; un segundo serializador sería la segunda forma (§3) |
| no añade `count` ni ninguna API de storage | el total sale de `all_events.len()` antes del `take` (§1) |
| no toca `ReplayOutput` | renombrar un campo **ahí** sí cambiaría un contrato, porque ese struct sí se serializa |
| no repite la auditoría de superficies que truncan | está medida y agotada (EXPLORATION §4) |

## §7 — Radio de impacto

Un solo punto de producción: `run_ledger_export` y su struct de resumen. El
**contenido del fichero exportado no cambia**, luego cualquier consumidor del
JSONL se ve afectado lo mismo que antes. Los que se rompen son los que leen la
**frase de salida**: cambia de `exported N events to P` a `exported N of M events
(P not written) to P`. Es un consumidor que ya dependía de una frase que no
decía lo que el comando hacía, y el cambio queda escrito aquí y en el changelog.
