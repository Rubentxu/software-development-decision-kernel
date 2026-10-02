# DESIGN — cl-ledger-watch-total

**Cycle:** `p-63676b11dc0ef88f/ledger-watch-total` (`OPEN/design`)
**Date:** 2026-10-02T21:12:36Z
**SCOPE:** `SCOPE-CONTRACT.md` · **PRE-FLIGHT:** `PRE-FLIGHT.md`

---

## §1 — La regla, y por qué es una sola

El total se cuenta con **el mismo predicado y el mismo cursor inicial que el
emisor**:

```
initial_cursor  =  el cursor ya ajustado por --from-tail (ledger.rs:707-713)
total_events    =  len(apply_watch_filters(list_events_after(initial_cursor, i64::MAX)))
```

El filtro se extrae a **una función libre**, `apply_watch_filters`, y se llama
desde los dos sitios —el bucle y el recuento—:

```rust
fn apply_watch_filters(events: &mut Vec<LedgerEvent>, cycle: Option<&str>, frame: Option<&str>)
```

**Por qué una función y no dos `retain` en línea.** El bucle ya filtra en
`ledger.rs:727-732`. Copiar esos dos `retain` para el recuento sería **una
segunda regla**: dos lugares que declaran qué es «un evento de esta consulta»,
que pueden divergir, y cuya divergencia no la nota nadie. Es exactamente el
defecto que este ciclo viene a cerrar, y también la razón por la que se
descartó `COUNT(*)` aunque fuera 51,8× más barato. **Un filtro, dos llamadas.**

## §2 — Dónde se mide el total

Al **final** de la corrida, una vez, con `limit = i64::MAX`:

- El total es una **foto del momento en que termina la corrida**. Los eventos
  que llegan durante ella cuentan hacia el total y no se emitieron, luego
  `pending` los incluye. Es cierto en los dos sentidos: son eventos que la
  consulta de esta corrida abarca y que esta corrida no emitió.
- El corte por tope (`ledger.rs:764-766`) se convierte en una bandera y un
  `break 'poll`, para que **los dos caminos de salida** —tope y fin de
  inactividad— pasen por el mismo recuento y no puedan declarar distinto.

## §3 — Forma

```rust
#[derive(Serialize)]
struct LedgerWatchSummary { emitted: u64, total_events: u64 }
impl LedgerWatchSummary {
    /// Derivado, nunca un tercer número escrito a mano.
    fn pending(&self) -> u64 { self.total_events.saturating_sub(self.emitted) }
}
```

`saturating_sub` y no `-`: si alguna vez `emitted > total` (un evento borrado
a mitad de corrida), el cierre aritmético **no entra en pánico** y R3 lo ve
como `emitted + pending != total_events` en vez de tumbar el binario.

- **texto:** `[watch] emitted {e} of {t} ({p} not emitted), exiting`
- **JSON:** `{"__watch_complete":true,"emitted":E,"total_events":T,"pending":P}`

El marcador `__watch_complete` **se conserva**: es lo que permite a un
consumidor NDJSON saber que la última línea es el resumen y no un evento.

## §4 — Una verdad, dos pintores

El texto y el JSON son **dos declaraciones del mismo hecho**. Si cada renderer
escribiera sus números por su cuenta, tendríamos la clase de defecto que este
ciclo cierra, dentro del arreglo: dos verdades que pueden divergir. Por eso los
dos se pintan **del mismo `LedgerWatchSummary`**, y por eso un test **compara
las dos salidas** sobre la misma corrida.

Es la octava vez que la misma forma aparece en esta sesión —«dos maneras de
decir lo mismo, cada una con su regla»— y la razón de que el ciclo anterior
acabara con una lista derivada y no con una lista escrita.

## §5 — Lo que este diseño NO hace

| no | por qué |
|---|---|
| `COUNT(*)` en SQL | 51,8× más barato y **sin prueba** de equivalir con el `retain` en Rust para `--cycle` y `--frame`: barato y posiblemente falso |
| cambiar la firma de `list_events_after` | añadiría un camino de cómputo nuevo en storage para ahorrar una materialización que el bucle ya paga cada 500 ms |
| tocar `ledger events` | ya declara; sus tests deben seguir verdes sin tocarlos |
| tocar `ledger export` | DEFECTO medido de la misma clase, **no-objetivo declarado**: otro ciclo |
| reescribir el pie a mano en cada renderer | §4 |

## §6 — Radio de impacto

Un solo punto de producción: `run_ledger_watch` (`ledger.rs:689-799`), su bucle
y su pie. El JSON de `ledger watch` **solo** lo emite esa línea de resumen, así
que añadir campos es **aditivo** para cualquier consumidor que la lea. El
único consumidor que puede romperse es uno que compare el pie **carácter a
carácter**, y ese es un consumidor que ya estaba mal: dependía de una frase que
no decía lo que el comando hacía.
