# RECEIPT — C3l.6 X07: segundo binario real (frontera de proceso)

**Slice:** `session58-c3l6-x07-second-binary`
**Fecha:** 2026-10-01 · **Baseline:** `bdb2ac65`
**Workflow:** `A-lite` · **SCOPE-CONTRACT:** [`SCOPE-CONTRACT.md`](SCOPE-CONTRACT.md)
**boundary_class:** `IN_PROCESS` → **`PROCESS / SQLITE_DURABLE`**
**UAT cubiertos:** AT-UAT-013, AT-UAT-014

---

## §1 El defecto: "dos handles" no es "dos binarios"

`crates/sddk-storage/tests/aiw_s8_x07_second_binary_integration.rs` modelaba el segundo
consumidor así:

```rust
fn run_writer(dir: &TempDir, project_id: &str) -> PathBuf { … }   // devuelve un PATH
let reader = Storage::open_read_only(&ledger).expect("reader open");
```

`run_writer` devuelve un **path**, y el test abre un **segundo `Storage` en el mismo
proceso**. Dos handles, un proceso. El doc del fichero afirma *"a second consumer
process"* y no lo hay: el consumidor comparte el proceso, la memoria y el código
compilado con el escritor. Eso prueba que **dos conexiones** coexisten, no que un
ejecutable aparte pueda leer lo que otro escribió.

El propio paquete lo admite: *"X07 usa dos handles `Storage` dentro del mismo test process."*
Y la matriz lo marcaba como tal: `X07: IN_PROCESS (aún no)`.

## §2 Por qué el binario real, y por qué NO uno nuevo

El paquete prohíbe introducir un binario permanente si una superficie CLI real ya
cubre la lectura. La cubre:

| Pieza | Evidencia |
|---|---|
| `sddk` es un `[[bin]]` de `sddk-cli` | `crates/sddk-cli/Cargo.toml:9-11` ⇒ `CARGO_BIN_EXE_sddk` en `sddk-cli/tests/` |
| Superficie de lectura | `sddk ledger events`, `sddk ledger verify`, `sddk cycle status`, `sddk project resolve` |
| Ledger redirigible | `SDDK_DATA_DIR` / `SDDK_STATE_HOME` (`sddk-engine/src/paths.rs:16-21`) |
| Ruta del ledger | `$STATE/sddk/projects/<project_id>/ledger.sqlite` (`paths.rs:135-142`) — depende **sólo** del `project_id` |

El test vive en `crates/sddk-cli/tests/` porque es el único sitio donde
`CARGO_BIN_EXE_sddk` existe. Los 4 tests de storage **no se tocan**: prueban semántica
entre dos conexiones, que sigue siendo válida; aquí se **añade** la frontera.

## §3 GREEN — 6/6

`crates/sddk-cli/tests/aiw_s8_x07_real_binary_boundary.rs`

| Punto | Test | Qué demuestra |
|---|---|---|
| **D0** | `the_consumer_is_a_separate_process_not_a_second_handle` | El consumidor es un fichero ejecutable real, **distinto del binario de test**, y corre en **otro PID** (`assert_ne!(pid, std::process::id())`) |
| **D1** | `real_binary_resolves_the_writers_identity` | El binario resuelve el mismo `project_id`/`workspace_id` que el escritor usó |
| **D2** | `real_binary_reads_cycle_and_events` | `cycle status` y `ledger events` ven el ciclo y el evento del escritor |
| **D3** | `real_binary_verifies_the_writers_ledger` | `ledger verify` valida el ledger ajeno: `event_count == 1` + `last_hash` presente |
| **D4** | `reads_through_a_real_binary_leave_the_ledger_byte_identical` | Tras cada lectura, el fichero del escritor es **byte-idéntico** |
| **D5** | `a_consumer_cannot_write_through_the_binary` | El consumidor no puede escribir: verbo inexistente y transición sin autoridad **fallan cerradas**, bytes intactos |

`6 passed; 0 failed` en 0.31s.

## §4 El falsificador que NO mordía, y el defecto de diseño que reveló

Éste es el hallazgo honesto de la slice, y es sobre el diseño del test, no sobre el producto.

**F12 era el falsificador central del contrato** — *"sustituir la invocación del binario
por otro `Storage::open_read_only` en el mismo proceso ⇒ el test debe CAER"*. En la
primera versión **no cayó**: `4 passed; 0 failed` con el binario completamente sustituido
por un handle in-process.

**Por qué no podía morder:** la aserción de D4 compara bytes *antes* y *después*. Un
handle in-process tampoco escribe, así que los bytes tampoco cambian: la aserción es
ciega a *quién* leyó. Sólo probaría "nadie escribió", que es la mitad débil de la
afirmación.

**Corrección de diseño:** añadir **D0**, que afirma lo que hace que la frontera sea real
— el consumidor es un artefacto ejecutable distinto del binario de test, y su **PID
difiere del PID del test**. Con D0, F12 vuelve a morder con el mensaje exacto del defecto
original:

```
assertion `left != right` failed: consumer ran in the test's own process — no boundary crossed
  left: 1583162   right: 1583162
```

**Lección transferible:** *byte-equality demuestra no-escritura, no ejecución-por-proceso.*
Un gate que afirme "otro actor lo hizo" necesita afirmar la identidad del actor, no sólo
la quietud del estado.

## §5 Falsificadores ejecutados

| # | Falsificador | Resultado |
|---|---|---|
| **F11** | Escritura **durable** antes de comparar bytes | **OBSERVED** — `assertion left == right failed: read command mutated the writer's ledger bytes` |
| **F12** | Binario → handle in-process | **OBSERVED (tras corregir diseño)** — `no boundary crossed`, PIDs idénticos |
| **F13** | El escritor escribe en **otro** ledger | **OBSERVED** — `STORAGE_NOT_FOUND: cycle not found`, probando lectura real, no vacío |
| **F14** | El write del consumidor "tiene éxito" | **OBSERVED** — `unauthorised transition must fail closed, not succeed` |

**F11 tuvo dos intentos y el primero no era concluyente.** Escribí con la conexión
abierta y la comparación no cambió: el storage usa **WAL** (`sddk-storage/src/lib.rs:335`),
así que la escritura vive en `ledger.sqlite-wal` y sólo llega al fichero principal en el
checkpoint al cerrar. La primera versión del falsificador "no muerde" por una razón
mecánica, no porque la aserción fuera débil. Corregido cerrando el handle antes de
comparar, muerde de inmediato.

**Consecuencia de diseño que esto impone (y que seMidió empíricamente antes de escribir el
test):** D4 compara **sólo `ledger.sqlite`**, no los sidecars. Medido con el binario real:
una lectura **crea** el ledger si no existe, y abre/cierra sidecars `-wal`/`-shm` durante
la operación. El fichero principal permanece byte-idéntico, que es lo que D4 afirma — y
es exactamente lo que el contrato pidió medir. Lo que D4 **no** afirma es "no se tocó el
directorio"; se declara como límite en §7, no se maquilla.

## §6 Perfil de verificación

```text
cargo test -p sddk-cli --test aiw_s8_x07_real_binary_boundary   6 passed; 0 failed
cargo test -p sddk-cli -p sddk-storage                         0 failed (861 + 188 + binarios)
cargo fmt -p sddk-cli -- --check                               limpio
cargo clippy -p sddk-cli --all-targets -- -D warnings          limpio
cargo test --workspace                                         0 failed
```

**Los 4 tests de storage intactos** (`aiw_s8_x07_second_binary_integration`): no se
tocaron, siguen verdes. Sigue siendo cierto que prueban dos conexiones en un proceso; ya
no es la **única** evidencia de X07.

## §7 Límites declarados

1. **D4 afirma quietud del fichero principal, no del directorio.** El binario abre el
   ledger en modo escritura (`SqliteLedgerFactory::open_ledger` → `Storage::open`, y
   `RuntimeContext::open` construye además el engine con `Storage::open`) y crea el
   fichero si falta. Los sidecars `-wal`/`-shm` aparecen y desaparecen. El contenido
   **no** cambia; el directorio sí se toca. Medido, no supuesto.
2. **La asimetría es intencionada:** escritor = API `Storage` en proceso; consumidor =
   binario. Lo que hay que demostrar es que un actor externo e independiente puede leer
   lo que un escritor produjo — no que dos llamadas de la misma librería se hablan.
3. **Binario de debug, no releaseado.** El release v2.5.0 está bloqueado por el toolchain
   musl (§8); el binario que cargo compila es el mismo código.
4. **No cubre instalador ni bundle**, ni el contrato de `MANIFEST.sha256`.
5. **AIW-S8 no pasa a VERIFIED con esto.** X04 ya estaba y X07 cierra, pero la vía C3l
   necesita C3l.7 (gate de arquitectura) para cerrarse.

## §8 Estado del release (sin cambio respecto a session-57)

v2.5.0 sigue **BLOQUEADO** en el step 3 de `scripts/release.sh`: falta el compilador C
`x86_64-linux-musl-gcc` (paquete `musl` no instalado; Bazzite 44 inmutable). No se forzó
build glibc: el asset se publica como musl y `install.sh` lo reparte como musl.
Bloqueador: `docs/architecture/adrs/BLOCKER-MUSL-TOOLCHAIN-MISSING.md`.
Remedio (requiere sudo del operador): `rpm-ostree install --idempotent musl-gcc` + reboot.
