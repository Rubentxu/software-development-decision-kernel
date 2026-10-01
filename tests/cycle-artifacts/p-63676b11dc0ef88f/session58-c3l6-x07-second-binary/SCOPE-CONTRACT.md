# SCOPE-CONTRACT — C3l.6 X07: segundo binario real

**Slice:** `session58-c3l6-x07-second-binary`
**Roadmap:** `docs/roadmap/ROADMAP.md` §C3l.6 · **Paquete:** `docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md:340`
**Workflow:** `A-lite` (debt_verification ejecutada en el PRE-FLIGHT: 0 critical/high abierta)
**Baseline:** `bdb2ac65` (release 2.5.0 bloqueado por toolchain musl — no es parte de esta slice)

---

## 1. Defecto verificado

`crates/sddk-storage/tests/aiw_s8_x07_second_binary_integration.rs` (4 tests) tiene esta firma:

```rust
fn run_writer(dir: &TempDir, project_id: &str) -> PathBuf { … }   // devuelve un PATH
let ledger = run_writer(&dir, project_id);
let reader = Storage::open_read_only(&ledger).expect("reader open");
```

`run_writer` devuelve un **path**, y el test abre un **segundo `Storage` en el mismo proceso**. Dos handles, un proceso. El doc del fichero afirma *"a second consumer process"* y no lo hay: el consumidor es el mismo proceso con otra conexión, que comparte memoria,时钟 y —lo importante— **no prueba compatibilidad entre binarios**, solo entre dos conexiones del mismo código en el mismo proceso.

El propio encabezado del paquete lo dice: *"X07 usa dos handles `Storage` dentro del mismo test process."*

## 2. Diseño: el binario real, no un segundo handle

El paquete prohíbe explícitamente introducir un binario permanente si una superficie CLI real ya cubre la lectura. Y la hay:

| Pieza | Evidencia |
|---|---|
| `sddk` es un `[[bin]]` de `sddk-cli` | `crates/sddk-cli/Cargo.toml:9-11` ⇒ `CARGO_BIN_EXE_sddk` disponible en `sddk-cli/tests/` |
| Superficie de lectura real | `sddk ledger events`, `sddk ledger verify`, `sddk cycle status`, `sddk project resolve` |
| Read-only de verdad | `cycle.rs:315` y `context_cmd.rs:334` abren con `Storage::open_read_only` |
| Ledger redirigible | `SDDK_DATA_DIR` / `SDDK_STATE_HOME` (paths.rs:16-21) |

**Arquitectura de la prueba:**

```text
writer process  ──(Storage API)──▶  <tmp>/state/sddk/projects/<pid>/ledger.sqlite
                                          ▲
                                          │ read-only
reader process  ──(sddk CLI real)─────────┘
```

El test vive en **`crates/sddk-cli/tests/`** (no en `sddk-storage`) porque solo ahí `CARGO_BIN_EXE_sddk` está disponible. Eso es lo que hace que el segundo consumidor sea **un binario y no un handle**.

## 3. Los 5 puntos que el consumidor debe demostrar

| # | Punto | Cómo se observa | Falsable |
|---|-------|-----------------|----------|
| D1 | Lectura de project/workspace | el proceso reader resuelve el mismo `project_id` y `workspace_id` que el writer | sí |
| D2 | Lectura de cycle/events | `sddk cycle status` y `sddk ledger events` ven el ciclo y los eventos | sí |
| D3 | Schema guard | `sddk ledger verify` valida continuidad y hashes sobre el ledger del writer | sí |
| D4 | Read-only enforcement | el reader **no puede** escribir: el ledger queda **byte-idéntico** (sha256 igual antes y después) | sí |
| D5 | Compatibilidad observable | el reader abre un ledger en schema 21 y agrees con el writer sobre versión e identidad | sí |

**D4 es el más fuerte**: no se afirma "es read-only" porque el comando lo diga, sino porque el **hash del fichero no cambia un byte** tras ejecutar el consumidor.

## 4. Restricciones

- **Ningún binario nuevo.** Se reutiliza `sddk`. Si una superficie CLI no cubre una lectura, se registra como límite, no se añade un binario artificial.
- El `project_id` lo resuelve el propio CLI en un subproceso (identidad determinista, verificada en session-56), y el writer **debe** usar ese id: si el writer eligiera un id arbitrario, la prueba cruzaría la frontera pero no probaría identidad.
- Los 4 tests existentes **no se tocan**: prueban semántica de storage entre dos conexiones, que sigue siendo válida. Se **añade** la frontera del proceso.
- `AIW-S8` **no** pasa a VERIFIED con esto: X04 ya estaba y X07 cierra, pero la vía C3l necesita C3l.7 (architecture gate) para cerrarse.

## 5. Falsificadores congelados

- **F11 (no side effects):** cambiar el consumidor a una ruta que escriba (o relajar el assert a `assert_ne!` sobre el hash) ⇒ el test debe CAER. Sin esto, "read-only" sería una afirmación sin respaldo.
- **F12 (proceso real):** sustituir la invocación del binario por otro `Storage::open_read_only` en el mismo proceso ⇒ el test debe CAER, porque el hash seguiría igual y el PID sería el mismo. Este es el falsificador que reproduce exactamente el defecto original.
- **F13 (lectura real):** que el reader apunte a un ledger distinto del que escribió el writer ⇒ debe CAER, probando que la aserción lee datos de verdad y no pasa por vacío.

## 6. Límites declarados por adelantado

1. El writer es el propio test usando la API de `Storage`; el reader es el binario. La asimetría es intencionada: lo que hay que demostrar es que **un consumidor independiente y externo** puede leer lo que un escritor produjo, no que dos-library-calls se hablan.
2. Un `sddk` releaseado no es instalable aún (el release está bloqueado), pero el **binario de debug compilado por cargo** sí lo está, y es el mismo código. La prueba usa ese.
3. La prueba **no** cubre el instalador ni el bundle: solo la frontera de lectura entre dos procesos.
4. Este slice **no** cierra el bloqueo de musl ni publica nada.
