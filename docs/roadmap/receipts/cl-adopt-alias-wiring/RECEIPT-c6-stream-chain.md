# RECEIPT — ADR-0152 criterio 6: `verify_stream_chain` sobre un stream canónico

- **Ciclo:** `cl-adopt-alias-wiring`
- **Fecha UTC:** 2026-10-02
- **WorkItem:** medición del criterio 6 de
  [`ADR-0152`](../../../architecture/adrs/ADR-0152-STORAGE-LEVEL-PROJECT-IDENTITY-ALIAS.md)
- **Veredicto:** **PASS**, y falsificado con **PASS=12 FAIL=0**
- **HEAD:** `001f7e0f` (sin commits sin publicar al empezar)
- **Tipo:** medición de solo lectura. Ningún fichero de producción tocado.

---

## 1. Qué se midió y por qué esa ruta

El criterio 6 dice: «Para al menos un proyecto con fact log, `verify_stream_chain`
sobre su stream canónico devuelve OK».

El candidato evidente era `sddk ledger verify-chain`, y **es el equivocado**: ese
comando corre únicamente `verify_chain_integrity` (`ledger.rs:299`), que
comprueba la cadena `SHA256(content_hash ‖ previo)`. El criterio nombra
`verify_stream_chain`, que es otra función (`event_store.rs:458`) y comprueba otra
cosa: recalcula el `content_hash` de cada evento desde su carga útil y lo compara
con el valor almacenado.

La ruta que corre **los dos**, y por tanto contiene al que el criterio pide, es
`Storage::verify_ledger` (`crates/sddk-storage/src/lib.rs:977`), invocado por
`sddk ledger verify` (`ledger.rs:234`):

```rust
pub fn verify_ledger(&self) -> Result<LedgerVerification> {
    let canonical = self.canonical_events()?;
    let store = SqliteEventStore::open_path(self.database_path()?)?;
    for stream in store.list_streams()? {
        if let Err(error) = store.verify_stream_chain(&stream) { … }
        if let Err(error) = store.verify_chain_integrity(&stream) { … }
    }
```

Es un **superconjunto** del criterio: los 114 streams del proyecto, no uno.

Proyecto elegido: `p-63676b11dc0ef88f`, que es el propio `sddk-framework`
(`project resolve` devuelve `identity_source: pinned` y
`remote_url: https://github.com/rubentxu/software-development-decision-kernel`).
No se eligió por conveniencia: es el que tiene fact log y es el proyecto cuyo
alias importa para este ADR.

## 2. Por qué sobre una copia

`RuntimeContext::open` (`cycle.rs:437`) **no abre en solo lectura**. Su tercer
parámetro es `generate_seed`, no «read-only», y dentro hace
`crate::Storage::open(&paths.ledger)`, que abre en escritura. `sddk ledger
verify` es un comando de verificación que, contra el storage real, habría
podido crear ficheros de SQLite o aplicar migraciones.

El criterio reza «sobre su stream canónico»; no dice «sobre el fichero de la
máquina». Una medición que promete no tocar el storage real no puede correr
contra él. Se copió el `ledger.sqlite` byte a byte a un `HOME` aislado y se
midió ahí:

```
sha256 original : 91ea03529587553ace193bd05eee9aceace84a0b2809f764bdd1f72c7efbf32c
sha256 copia    : 91ea03529587553ace193bd05eee9aceace84a0b2809f764bdd1f72c7efbf32c
COPIA IDENTICA: OK
```

El sha256 del original se volvería a comprobar al final (asignatura M6).

Aislamiento: `HOME`, `XDG_STATE_HOME`, `XDG_DATA_HOME` y `XDG_CACHE_HOME`
propios en `/var/home/rubentxu/c6/sandbox`, con `SDDK_DATA_DIR`,
`SDDK_STATE_HOME` y `SDDK_HOME` sin definir. Nada se escribió dentro del repo ni
en ningún otro repositorio (AGENTS.md §1).

## 3. Resultado

```
$ sddk ledger verify --root <repo> --scope .
event_count: 590
last_hash: sha256:271e58f7c2ffa52e515a1703bac92a66a2de1072448440dd979371a5b5c37371
exit=0

$ sddk ledger verify-chain --root <repo> --scope .
stream: all streams of p-63676b11dc0ef88f
streams: 114
event_count: 590
head_chain_hash: null
status: PASS
exit=0
```

## 4. Tres hechos que la medicion revela y el enunciado no dice

1. **`events_v1` es append-only por trigger, no por convención.** Lleva
   `events_v1_no_update` y `events_v1_no_delete`, ambos `BEFORE … RAISE(ABORT,
   'events_v1 are append-only')`. Consecuencia para este criterio: **solo puede
   pasar a rojo por corrupción, nunca por una escritura legítima**. Es un
   invariante de corrupción. El falsificador tuvo que tirar los triggers en la
   copia para poder manipular una fila, lo que es en sí mismo la prueba (asignatura
   M0).
2. **La verificación es total, no parcial.** `verify_chain_integrity` salta las
   filas con `chain_hash` vacío por ser pre-migración (`event_store.rs:511`). Las
   590 filas de este proyecto tienen `chain_hash` y `content_hash` no vacíos, así
   que no se saltó ninguna. En otro proyecto el mismo `PASS` sería más débil.
3. **El binario es de trabajo, no el instalado.** `/var/home/rubentxu/cargo-targets/debug/sddk`
   (`sddk 2.5.3`). `~/.local/bin/sddk` sigue obsoleto y no se usó.

## 5. Falsificador

Un verde sin falsificar es una afirmación, no una evidencia. `/var/home/rubentxu/c6/02-falsify.py`
—**PASS=12 FAIL=0**, fuera del repo, como los demás falsificadores de este ciclo.

| # | Asserción | Resultado |
|---|-----------|-----------|
| M0 | `events_v1` rechaza un `UPDATE` con «append-only» | PASS |
| M1 | Copia intacta: salida 0, sin «error», y `event_count: 590` | PASS |
| M2 | `content_hash` manipulado ⇒ rojo, el mensaje nombra `hash_drift` y la fila correcta | PASS |
| M3 | `chain_hash` manipulado ⇒ rojo, el mensaje nombra `chain_drift` | PASS |
| M4 | La corrupción de `content_hash` la nombra `hash_drift` `ledger verify` y `chain_drift` `ledger verify-chain` | PASS |
| M4 | La corrupción de `chain_hash` la nombra `chain_drift` en ambos, y nunca `hash_drift` | PASS |
| M5 | Restaurada la copia, el verde vuelve | PASS |
| M6 | `sha256` del ledger real sin cambios al terminar | PASS |

M4 es el que da valor a la cifra: si los dos verificadores dijeran lo mismo, la
medición no distinguiría el que el criterio nombra de su vecino, y PASS sería
indistinguible de haber medido el equivocado.

## 6. Un FAIL que era un guard malo

La primera corrida dio **PASS=11 FAIL=1**. El FAIL fue `M4_verify_chain_NO_ve_content_hash`,
que afirmaba que `ledger verify-chain` seguiría en verde tras rehashear
`content_hash`. Es falso: `verify_chain_integrity` calcula
`SHA256(content_hash ‖ previo)` con el `content_hash` **cargado**, así que
rehashear el contenido rompe la cadena, y el comando cae con `chain_drift`.

No es un defecto del producto: es la cadena cumpiéndose. Y no es una mutación
mala en el sentido de INC-DEBT-059, porque la mutación sí aterrizó y sí cambió
el resultado —lo que estaba mal era la afirmación. Se corrigió el guard para
exigir el **diagnóstico** (cada verificador nombra lo suyo) en vez de exigir el
**silencio** del vecino. El FAIL queda aquí porque el camino que llevó a él es
información: la primera intuición sobre qué distingue a dos verificadores era
equivocada, y el falsificador la corrigió.

## 7. Lo que esta medición NO dice

- **No dice que el storage real esté íntegro.** Dice que una copia byte-idéntica
  del ledger de **un** proyecto pasa. Los otros 332 proyectos con directorio no se
  midieron.
- **No cierra el criterio 5.** El huérfano de `skillgraph` sigue ahí y depende de
  una decisión del operador.
- **No promueve el ADR.** Con el 5 en rojo, el 6 verde no cambia su `status`.
- **No cubre la ruta `graph rebuild`**, que también llama a `verify_stream_chain`
  (`graph_store.rs:73` y `:133`, y `rebuild.rs:51`), y sigue sin medirse.

## 8. Reproducir

```bash
bash   /var/home/rubentxu/c6/00-setup.sh     # sandbox + resolución de identidad
bash   /var/home/rubentxu/c6/01-verify.sh     # copia + medición
python3 /var/home/rubentxu/c6/02-falsify.py   # PASS=12 FAIL=0
python3 /var/home/rubentxu/c6/03-discover.py  # triggers, índices y cobertura de hashes
```
