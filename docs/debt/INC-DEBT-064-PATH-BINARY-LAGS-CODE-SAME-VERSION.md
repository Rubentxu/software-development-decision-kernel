---
id: INC-DEBT-064
title: "El binario de sddk en el PATH va mas atras que el codigo y declara la MISMA version: la señal que se usa para saberlo sale verde"
status: open
severity: high
priority: P1
revalidated_at: 2026-10-03
revalidated_in_session: session-69s
fingerprint: "path_binary_lags_code_while_declaring_same_version"
fingerprint_aliases: []
cluster_id: CL-DIST
created: 2026-10-03
created_by: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
owner: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
detected_at: 2026-10-03
detected_in_session: session-69n
component: distribution
surface: binario desplegado (fuera del repo) frente a crates/
related: [INC-DEBT-061, INC-DEBT-060, INC-DEBT-037]
references:
  - docs/roadmap/CURRENT.md
  - docs/roadmap/STATE.yaml
  - crates/sddk-cli/src/vault_cmd.rs
---

## Qué es

`~/.local/bin/sddk` declara `sddk 2.5.3`. El workspace de este repo **también**
declara `2.5.3`. Los dos se presentan como la misma versión y **no son el mismo
código**: el binario es del **2026-10-01 21:17** y el último commit del repo es
del **2026-10-03 03:02**, con **1,24 días** de diferencia. Todo el trabajo de la
ventana declarada-pero-no-publicada viaja bajo el mismo número.

La comparación de versiones —que es la que todo el mundo hace— **sale verde**:

```
VERSION_DECLARADA_POR_EL_BINARIO=sddk 2.5.3
VERSION_DEL_WORKSPACE=2.5.3
VERSIONES_COINCIDEN=True          <-- la comprobacion habitual dice que si
ESTRUCTURAL_tiene_cycle_list=False
DIAS_DE_RETRASO=1.24
BINARIO_MIDE_EL_CODIGO_ACTUAL=False
```

No es una anomalía de esta máquina: es **estructural mientras haya una release
declarada sin publicar**. El binario se instala desde un release (AGENTS.md §4.2)
y el workspace no bumpea entre releases, así que todo el trabajo posterior a la
última publicación comparte número con ella.

## Cómo se mide, y el daño concreto

Instrumento: `22-medir-binario-al-dia.py`. La prueba de que **no** es un detalle
cosmético es la misma superficie, medida con los dos binarios sobre un fixture
propio de 25 documentos (`21-medir-vault-search.py`, que no toca ningún vault
real):

| Comprobación | binario del PATH | binario del código |
|---|---|---|
| `vault search --limit 5` declara el total | **no** | **sí** |
| `vault search --limit 0` devuelve todos | **no** (`no hits`) | **sí** |
| JSON con dónde llevar el total | **array desnudo** | **objeto** |

`vault search` **no tiene ningún defecto en el código**: `run_vault_search`
(`vault_cmd.rs:464-488`) llama `count_matches` para el total, traduce
`limit == 0` a `usize::MAX` y construye `SearchOutput { truncated, shown,
total_hits, hits }`. El arreglo es `37870817`, del 2026-10-02 21:09, en
`origin/main`. **Lo roto es el artefacto que se ejecuta.**

El daño es de **veracidad de la evidencia**, que es la única moneda de este
framework: una medición de comportamiento hecha con el binario del PATH es
evidencia sobre el código viejo, con toda la apariencia de ser evidencia sobre
el actual. No es hipotético: la auditoría de session-69f se apoyó en `ledger watch`
**medido con el binario del PATH**, y salió bien por suerte, no por método. La
comprobación de que aquello decía la verdad fue leer el código, y llegó tarde.

## Antecedente: la lección ya se había pagado una vez

La nota (a) de **INC-DEBT-061** dice, textual: el binario instalado era
`sddk 2.2.27` del 2026-09-29, anterior a la tabla de alias, luego
«el arreglo no está roto, **no está desplegado**», y la medición que separó las
dos hipótesis fue una sola, *qué binario escribió*.

**La lección se aplicó como dato de un caso, no como mecanismo.** No quedó nada que
impida repetirla, y este documento es esa repetición. Lo que faltaba no era el
diagnóstico —ya se sabía— sino el **guard que se consulta antes de medir**.

## Las tres salidas, y cuál se propone

1. **Un check en `sddk dev doctor`** que compare la fecha del binario con la del
   repo y avise cuando va por detrás. `dev doctor` hoy verifica entorno y bundle
   y no dice nada de esta relación (medido). Es el sitio natural, y el recibo de
   doctor ya es la superficie que la release consulta en su paso 11.
2. **Un script de repo** (`scripts/`) que se pueda correr antes de cualquier
   medición, con salida utilizable como evidencia de gate. Es lo que
   `22-medir-binario-al-dia.py` ya es, pero vive fuera del árbol versionado.
3. **Nada**, y escribir la regla en los punteros: mientras haya ventana
   declarada-pero-no-publicada, medir con el binario construido del repo
   (`CARGO_TARGET_DIR=… cargo build --release --bin sddk`), nunca con el del PATH.

La **3** es la que se aplica hoy y es la de menor coste; la **1** es la que
convierte el conocimiento en garantía. Se proponen la 1 y la 3, con la 2 como
intermedia si la 1 resultara invasiva.

## Por qué `high` y no `critical`

Por el mismo razonamiento que bajó a `high` INC-DEBT-060 y INC-DEBT-062: **no hay
pérdida de datos**. El ledger está íntegro, nada se corrompe y la release
publicada resolvería la condición por la vía natural. Lo que se rompe es un
**contrato** —«`sddk` es este código»— y lo rompe en silencio, que es la
combinación que más veces ha producido números falsos en esta serie.

Sube a `critical` si una medición con el binario obsoleto llega a un **documento
publicado** como afirmación sobre el producto, o si `dev doctor` declara
coherencia donde no la hay.

## Lo que este documento NO afirma

- **No es una regresión.** El binario viejo no está roto: es una build correcta
  de un código anterior.
- **No accuse a la release.** Publicar 2.5.3 deja el binario al día por la vía
  normal, y esa vía está bloqueada por la clave KMS, que es del operador.
- **No invalida la fase `verify` de `cl-release-forge-testability`.** Los cuatro
  gates de `phase.verify.complete` y el requisito `verification-report` que
  aplicó el binario viejo son **los mismos** que declara el código actual
  (`crates/sddk-cli/tests/cli.rs:5626-5660`): `tests-pass`, `policy-compliant`,
  `debt-severity-assigned`, `debt-priority-assigned` y `verification-report`.
  Verificado, no supuesto.

---

## Revisión de vigencia (2026-10-03, sesión 69s)

**Dos de los tres puntos de «Lo que NO se ha hecho» de abajo ya están hechos, y el
que queda sigue siendo el que importa.** No se reescriben: la sección de abajo se
conserva como estaba y esta va delante.

| Punto de «NO se ha hecho» | Medido hoy |
|---|---|
| «la fase verify de `cl-build-identity` está abierta» | **CADUCADO** — cerrada en `261a578c`, con 2 gates `passed` |
| «`dev doctor` no lo invoca» | **CADUCADO** — cableado en `a5c18b97`; `doctor` ya publica `binary.build_identity` |
| «el binario del PATH sigue obsoleto» | **VIGENTE**, y ahora con su distancia exacta |

**La distancia, que el documento solo expresaba en días:** el binario instalado
es de `90f16ad2` (**2026-10-01 21:10:43 CEST**, construido a las 21:17:29) y
`HEAD` es `a2b0bd13`: **202 commits de distancia**. `v2.5.2` se publicó a las
18:22:15Z, luego el binario se construyó **tres horas después de publicar
v2.5.2**, sobre un árbol que ya declaraba `2.5.3`.

**El dato nuevo, y es la forma más difícil de esta condición:** el binario
obsoleto **no tiene `sddk dev build-id`**, porque se construyó antes de
`032e9553`. O sea que **el artefacto que tiene el problema no puede ejecutar el
check que lo encuentra**, y tampoco puede decir que no lo tiene. La
autoinspección ausente no es detectable por sí misma; solo se ve desde fuera,
midiendo la versión que declara contra los subcomandos que expone.

**Y una corrección de una medición publicada.** `REVALIDACION-R2.md` cabecera con
*«Binario usado: `sddk 2.5.3` (build del 2026-10-03 06:57)»*. El fichero en disco
es de **2026-10-01 21:17:29 CEST**: la revalidación de R2 se hizo **con el binario
viejo**, y por eso no vio que `sddk cycle list` ya existía en el código. Medir la
vigencia de una deuda con el artefacto que la deuda describe es medir con el
instrumento que la deuda dice que no sirve.

**Sin cambio de estado ni de severidad.** La condición de escalada —que una
medición con el binario obsoleto llegue a un documento publicado como afirmación
sobre el producto— **acaba de cumplirse una vez**: durante esta sesión, con el
binario viejo, se creyó que `sddk cycle list` no existía. Se detectó comparando
contra el código, no porque el binario lo dijera. `status: open`, `high`/`P1`.

Evidencia completa: `docs/roadmap/receipts/c3m5-bounded-contexts/REVISION-VIGENCIA-R2.md` §4.

---

## Estado del remedio, y por qué esto sigue `open`

**El detector que faltaba ya existe, y la condición sigue viva.** Son dos
hechos distintos y confundirlos es exactamente el error que esta sección
evita.

**Lo entregado** (ciclo `p-63676b11dc0ef88f/cl-build-identity`, fase de build
cerrada):

- `crates/sddk-cli/build.rs` embebe commit, procedencia y suciedad.
- `sddk dev build-id` los declara en texto y JSON, y la procedencia acompaña
  siempre al valor para que un `unknown` no se pueda leer como un commit.
- `sddk dev build-id --check` compara contra un checkout y **nombra** la
  relación: `matches`, `behind`, `diverged`, `no_checkout`, `unknown`.
- `scripts/release.sh` mide el SHA y lo exporta antes de construir, de modo que
  un binario **publicado** declara `source: env` y `--check` puede salir de
  `unknown`. Sin ese último punto, el detector no habría podido cumplir su
  función en el caso que motiva este documento, que es un binario publicado y
  obsoleto.

**Lo que NO se ha hecho, y por qué el estado no baja a `resolved`:**

- **El binario del PATH sigue obsoleto**, y lo seguirá hasta que se publique una
  release y se instale. Publicar 2.5.3 es la vía natural y está bloqueada por la
  clave KMS, que es del operador.
- **La fase `verify` de `cl-build-identity` está abierta.** Lo entregado está en
  `OPEN/verify` con el gate `implementation-complete` en `passed`; los gates de
  la verify todavía no se han graduado.
- **La ruta de detección no está cableada en ningún sitio automático.** Existe
  `dev build-id --check` para que un humano o un script lo consulte, pero
  `dev doctor` **no** lo invoca: hoy `binary.bundle_coherence` sigue sin detectar
  un binario obsoleto. Conectar la detección al doctor es trabajo por hacer, y
  mientras no ocurra, la deuda sigue existiendo como deuda: hay que acordarse
  de mirar.

**Por qué no se escribió «cierra» en el changelog.** Un `CHANGELOG` que dice
que cierra una deuda que sigue abierta es la misma clase de mentira que este
documento denuncia: afirmar por el número en lugar de por el estado. El
changelog dice **«avanza»**, y este documento dice **`open`**, y los dos son
verdadersos el mismo día.

**Severidad sin cambio: `high`, y no `critical`.** Sigue sin haber pérdida de
datos: lo que se rompe es un contrato, y en silencio. La condición de
escalada que este documento ya declaraba —que una medición con el binario
obsoleto llegue a un documento publicado como afirmación sobre el producto, o
que `dev doctor` declare coherencia donde no la hay— **sigue sin cumplirse**, y
por eso la severidad no se toca.
