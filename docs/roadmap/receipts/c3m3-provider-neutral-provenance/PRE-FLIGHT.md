# PRE-FLIGHT — C3m.3 «Runtime provider-neutral provenance»

> **CHECKPOINT 2026-10-06 — dos afirmaciones de este documento quedaron FALSADAS
> durante la ejecucion.** Se conservan abajo con su texto original y se marcan
> aqui; el detalle esta en §Checkpoint. No se reescribieron para que el
> documento cuadre con lo que se hizo: eso seria el documento describiendo su
> propio arreglo.
>
> - **«No hace falta tocar `code_intelligence_port_mcp.rs`»** — FALSO. La razon
>   era que el mecanismo ya existia; eso es cierto, pero existia **privado**.
>   El arreglo minimo y correcto era volverlo alcanzable, y eso es un cambio de
>   API en el crate.
> - **Criterio de exito 1 («el comando no contiene el token `cognicode` en su
>   ruta de produccion»)** — FALSO, y era el criterio mas caro de los tres:
>   obligaba a cambiar el locator y el `producer`, que **no hacen falta** y
>   cuyo cambio exige un ADR que no existe.

## Lo que se mide antes de decidir el corte

**C3m.0 ya esta cerrado y no se toca.** `ADR-0154-KMT-CANONICAL-MEANING.md` esta
`status: accepted`, `accepted_at: 2026-10-03`,
`accepted_by_cycle: p-63676b11dc0ef88f/c3m-kmt-canonical`, con sus siete
criterios medidos uno a uno. MEDIDO que el criterio 1 se sostiene: queda **1**
struct `KMT*` (`KmtUnitIndex`).

**C3m.2 ya esta cerrado y no se toca.** `INC-DEBT-048` esta `status: resolved`
(2026-10-02, opcion (a) del operador). `REQ-A3S1-021` en la spec canonica afirma
que `revised_at` **participa** en `basis_hash`, y el codigo lo hace
(`derive_basis_hash_at(..., Some(revised_at))`). **Exit gate verificado**: no
queda ningun comentario ni test que afirme la semantica opuesta. El roadmap
recomienda la contraria, pero la spec es la autoridad y el operador ya decidio;
esta nota lo dice para que no se reabra por leer solo el roadmap.

## El defecto, medido en el codigo y no en un documento

`crates/sddk-cli/src/verify_kernel_cmd.rs:205` construye el `AnalysisBasis`
asi:

```rust
provider_build: "cognicode-mcp/verify-cmd".to_string(),
```

**Es un literal**, y el comando construye el `AnalysisBasis` entero a mano — seis
campos, cuatro de ellos tambien fijos (`protocol_major: 2025`,
`protocol_minor: 3`, `source_revision: "verify-cmd"`).

**Y aqui esta el dato que reduce el alcance: la mecanica correcta YA EXISTE y no
se usa.** `code_intelligence_port_mcp.rs:300` tiene un metodo `basis()` que
construye exactamente ese `AnalysisBasis`, y su `provider_build` se deriva de
`self.lock_state().server_version`, que el handshake MCP lee **del servidor** en
la linea 167 (`reply["result"]["serverInfo"]["version"]`). Ese `server_version`
no lo escribe el codigo: lo anuncia el proceso al que el comando acaba de
arrancar, y el handshake ya falla cerrado si no encaja.

O sea: **el adapter sabe decir quien es el proveedor, y el comando no pregunta.**

Por que eso es el defecto y no una cuestion de estilo: `provider_build` entra en
`code_intelligence_port_fake.rs:329`, o sea que **participa en el digest de la
evidencia**. Con el literal, dos proveedores distintos que satisfacen el mismo
claim producen **evidencia indistinguible**, y un receipt que dice
`cognicode-mcp/verify-cmd` afirma el nombre de un proveedor sin haberlo medido.

**El arreglo es, por tanto, de una sola direccion**: el comando deja de
construir el basis a mano y toma el del adapter. No hay que inventar ningun
mecanismo nuevo, ni tocar `code_intelligence_port_mcp.rs`, ni decidir nada.

## Alcance, medido antes de abrir

`provider_build` aparece en **un solo sitio de produccion** (linea 205). Los
demas son tests: `a6_s1_uat_coverage_fake.rs:96`, `a6_s2_uat_c05_c07_c09.rs:46`,
`a6_cognicode_protocol_spike.rs:27`, `aiw_s1_cognicode_real.rs:37`,
`a6_s3_ac10_verify_integration.rs:43`. Se tocan si hace falta, y cada uno se
justifica por si mismo.

## Que NO se hace aqui

- **No se renombra `CogniCodeMcpAdapter`.** El adapter concreto puede conocer su
  proveedor — es lo que lo hace concreto. Lo que no puede hacer es el **runtime**
  codificar el nombre del proveedor en la evidencia.
- **No se toca `producer_l0_adapter.rs`**, que tambien menciona `cognicode`: ahi
  es el nombre del productor del evento, no la identidad de una observacion
  port. Se declara fuera de alcance y se revisa aparte.
- **No se abre C3m.4 ni C3m.5.** Son cortes distintos con problemas distintos.
- **No se reescribe el ADR-0154** ni se reabre INC-DEBT-048.

## El criterio de exito, y por que no es «compila»

Un guard que solo comprueba que el literal desaparecio se cumple cambiando el
nombre a otra constante. El criterio tiene que ser **estructural**:

1. `verify_kernel_cmd.rs` **no contiene** el token `cognicode` en ningun punto
   de su ruta de produccion (los doc-comments que lo nombran como ejemplo de
   `--provider-bin` pueden seguir nombrandolo, porque nombran lo que el operador
   tiene que pasar, no lo que el runtime afirma).
2. `provider_build` **deja de ser un literal** y se deriva de algo observado.
3. Un guard falsado: cambiar la identidad observada por otra constante tiene que
   **tirar** el guard. Sin ese paso, la comprobacion 1 seria decoracion.

## Readiness

**READY.** El defecto esta medido en una linea de produccion, el consumidor esta
identificado (`code_intelligence_port_fake.rs:329`, dentro del digest de la
evidencia), el mecanismo correcto ya existe en el mismo comando y no se usa, y
los otros dos cortes candidatos de C3m estan cerrados con evidencia. No hay
decision de diseno pendiente: la spec ya dice que la identidad se deriva de lo
observado, y este commit solo deja de mentir sobre lo observado.

---

# §Checkpoint — que falso la medicion posterior (2026-10-06)

Este bloque se anade **despues** de ejecutar, y no sustituye a nada de lo de
arriba. Lo de arriba se queda como se escribio porque es lo que se creyo al
abrir, y porque un PRE-FLIGHT reescrito para cuadrar con lo que se hizo no
sirve para nada: no distingue la hypothesis del resultado.

## 1. El cambio NO era una decision nueva: era restitucion

Al buscar la autoridad antes de editar aparecio
[`ADR-0155-CORE-DOES-NAME-PROVIDERS.md`](../../architecture/adrs/ADR-0155-CORE-DOES-NAME-PROVIDERS.md):
`status: accepted`, `accepted_at: 2026-10-03`,
`accepted_by_cycle: p-63676b11dc0ef88f/c3m3-provider-neutral-provenance`. Es el
**mismo ciclo**, y en §Consecuencias afirma:

> **El digest no cambia.** `provider_build` ya era una `String` con el build del
> proveedor y sigue mezclándose en el digest. Este ADR no toca la derivación del
> hash, sólo **de dónde sale el valor**: de un string declarado por el adaptador
> en lugar de un enum del core.

**MEDIDO: eso era falso en el CLI hasta este corte.** El adapter si derivaba de
`server_version`; `verify_kernel_cmd.rs` se construia su propio basis con un
literal. `grep` sobre `provider_build:` en `crates/*/src/` deja **un** constructor
vivo —el del adapter— y el resto en `tests/`.

O sea: el §3 del SCOPE-CONTRACT de C3m.3 lista lo que se aplico y
`verify_kernel_cmd.rs` **no esta**. El ciclo se cerro sin cubrir esa ruta. Lo que
este commit hace es dejar cierta una afirmacion que un ADR aceptado ya hacia y
que el codigo incumplia — no abrir una decision nueva, que para eso habria que
escribir un ADR nuevo.

## 2. Por que el criterio 1 era falso, y que lo hacia en su lugar

El criterio 1 pedia que el token `cognicode` desapareciera del comando. Se
cumplio, y al medir resulto ser el **criterio equivocado**, por tres razones
concretas:

1. **Un URI malformado.** `provider_build` es `nombre/version`
   (`cognicode-mcp/0.4.1`). Usarlo como esquema produce
   `cognicode-mcp/0.4.1://find_usages/...`, y un esquema URI **no puede
   contener `/`**. El arreglo producia algo que parece una observacion y no
   lo es.
2. **Un contrato de evidencia que exige ADR.** El ciclo C2a dejo escrito, en
   `c2a-msgfix/SCOPE-CONTRACT.md` F5, que ese locator es *DO NOT TOUCH* y que
   cambiarlo «requires an ADR». Ese ADR no existe. Lo que si hizo el SCOPE de
   C2a fue dejar constancia de la puerta, no cerrarla: su F5 marca el
   contrato, no lo veta para siempre.
3. **No hacia falta.** `digest_result` hashea `provider_build` **primero**
   (`code_intelligence_port_mcp.rs:325`), luego la identidad observada ya
   llega a `ObservationId::derive` por el `basis`. Meterla tambien en el
   locator duplicaba el mismo hecho en dos campos que se presume
   independientes.

La frontera que queda, y que es la de ADR-0155 §218 — *«No declara que
nombrar al proveedor en un **adaptador** sea un defecto: es donde el nombre
vive»*:

| Que | De donde sale | Se mide |
|---|---|---|
| **build** (`provider_build`) | del `serverInfo.version` anunciado | **si**, y por eso entra en el digest |
| **familia** (`producer`, locator) | del adapter, que es cognicode-especifico por construccion | no, y no tiene por que |

## 3. El criterio que si, y que sustituye al 1

1. El runtime **no construye** el `AnalysisBasis` a mano, y el literal de build
   no medido no existe en codigo ejecutable de ningun modulo de `src/`.
2. `provider_build` **deja de ser literal** y se deriva de `server_version`, que
   viene del `serverInfo` **negociado** —no de una constante del adapter.
3. **POSITIVA, y es la que faltaba:** el build observado **sigue siendo
   load-bearing**. `digest_result` lo hashea primero, y el CLI convierte ese
   digest en la base de la observacion. Sin esta comprobacion, A y B se
   cumplen igual **borrando el campo**, y eso no seria una correccion: seria
   destruir la trazabilidad y llamarle neutralidad.
4. Falsado con **8 mutaciones al source real**, cada una exigida por su propia
   comprobacion: `DETECTADAS=8 MAL_MOTIVO=0 SOBREVIVIDAS=0 SKIP=0`.

## 4. Dos fallos del guard, ambos contra si mismo

Ninguno se resolvio bajando el liston.

- **A3 daba falso positivo.** Filtraba la salida de `grep -rn` con un helper
  que asume lineas de fichero; pero `grep -rn` antepone `ruta:linea:`, luego
  ninguna linea empezaba por `//` y la propia linea de comentario del CLI —que
  cita el literal al explicar el defecto— contaba como reaparicion. Un guard
  que no distingue su propio instrumento del codigo que mide esta midiendo
  sobre algo que no es el codigo.
- **B2 era satisfiedo por la prosa.** Buscaba `serverInfo` en crudo, y el doc
  de `basis()` lo cita al explicar de donde sale la identidad: el check daba
  verde con el handshake desconectado. La correccion fue pasar el check por el
  filtro de comentarios, no relajar el check.