---
id: INC-DEBT-064
title: "El binario de sddk en el PATH va mas atras que el codigo y declara la MISMA version: la señal que se usa para saberlo sale verde"
status: open
severity: high
priority: P1
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
