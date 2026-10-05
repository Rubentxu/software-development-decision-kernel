---
id: INC-DEBT-064
title: "El binario de sddk en el PATH va mas atras que el codigo y declara la MISMA version: la señal que se usa para saberlo sale verde"
status: resolved
severity: high
priority: P1
revalidated_at: 2026-10-05
revalidated_in_session: session-81
resolved_at: 2026-10-05
resolved_in_session: session-81
resolved_by: scripts/check_binary_freshness.sh + su guard + la regla en AGENTS.md 2.3.1
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

**Addendum session-69s: los contratos de identidad ahora son gates de release.**
Tres tests que median esta misma condición —`test_build_identity_policy.sh`
(PASS=8), `test_release_build_identity.sh` (PASS=22) y
`test_kmt_canonical_meaning.sh` (PASS=6)— estaban **sin runner**, y
`tests/test_gate_coverage.py`, que corre en el paso 1b de `release.sh` y cuyo
fallo hace `die`, estaba **en rojo por eso**. Consecuencia medida: **`bash
scripts/release.sh` moría antes de compilar.** Los tres están cableados ahora, y
el gate de cobertura da `RESULT: PASS` con 0 tests sin runner.

**Addendum session-69s bis 4: los cuatro estados de `binary.build_identity` ya
corren en release**, en el paso `3d/14`, en contra de lo que decía el párrafo
anterior — que queda corregido aquí y no se borra. Estaban excepcionados porque
`test_doctor_identity_states.sh` exige **dos binarios como argv** y el release solo
tiene el concluyente. **Medido:** pasando el concluyente en los dos huecos,
**O2–O5 y O7 pasan y solo O6 falla** (`PASS=17 FAIL=2`); O6 es el único objetivo
que depende de la procedencia **no** concluyente. Y el precio de medirlo entero
está medido también: una compilación en frío de debug son **118,77 s**, o sea ~2
min por publicación para medir un estado en el que el check **por diseño no
decide**. El guard ganó un **modo de un solo binario** que declara O6 `NOT_RUN` con
su motivo y baja la cuenta de veredictos de 5 a 4. El umbral se **falsificó**:
quitar un `registrar` lo detectan dos mecanismos independientes en ambos modos.

**Lo que sigue sin cubrirse, y es el residuo honesto:** el estado de procedencia
**no concluyente** (`source: git`) continúa siendo **cobertura de sesión, no de
pipeline**. Se mide en el verify de `cl-doctor-build-identity` (2 binarios reales,
19 comprobaciones) y en cualquier ejecución del guard con dos argumentos, pero no
en el camino de publicación. Cerrarlo requiere el arnés que construya los dos
binarios, que es trabajo declarado y no una excepción.

**Severidad sin cambio: `high`.** Conectar los contratos al release reduce la
probabilidad de que un artefacto con identidad rota llegue a un tag, pero **no
toca la condición**: el binario del `PATH` sigue obsoleto, y mientras la release
esté bloqueada lo seguirá estando. Un remedy que se acerca al cierre sin
alcanzarlo es exactamente el estado que esta deuda describe desde su título.

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

---

# RESOLUCION (session-81) — la DETECCION ya existia; lo que faltaba era que el JUICIO no vivia en el artefacto

Lo de arriba se conserva: es el registro de lo que se creia al detectar y al
revalidar en session-69s. **Dos de sus afirmaciones ya estan medidas como
obsoletas** y se corrigen aqui.

## 1. REVALIDADO, no supuesto: la deteccion existe y funciona

MEDIDO contra HEAD, con el binario instalado:

```
$ sddk dev build-id --check
commit: e9d368feb71415336652cfdcce95de929852e6cc
checkout_head: 147f2bef3ffb47db787c79c3662a0f4d7f9be6b5
relation: Behind
reason: el commit del binario (e9d368fe) es ancestro del HEAD del checkout,
        luego el checkout tiene trabajo que el binario no contiene

$ sddk dev doctor | grep build_identity
binary.build_identity: missing — FALLO: <el mismo motivo, nombrado>

$ sddk dev doctor --strict >/dev/null; echo $?
1
```

Las dos CADUCADAS de la seccion de session-69s lo eran de verdad:

| Punto de «LO que NO se ha hecho» | Medido en session-81 |
|---|---|
| «la fase verify de `cl-build-identity` esta abierta» | **CADUCADO** — cerrada en `261a578c` |
| «`dev doctor` no lo invoca» | **CADUCADO** — cableado en `a5c18b97`; `doctor` publica `binary.build_identity` y `--strict` sale con 1 |
| «el binario del PATH sigue obsoleto» | **VIGENTE, y por diseno** — ver §3 |

## 2. Lo que faltaba de verdad: el JUICIO vivia DENTRO del artefacto

`dev build-id --check` se ejecuta **desde el binario que juzga**. Eso rompe
la propiedad de dos maneras, y ninguna se arregla metiendo mas codigo dentro del
binario:

1. **No se puede comprobar algo antes de instalarlo.** Para correr el check
   hace falta el artefacto, luego el artefacto solo se pronuncia sobre si
   mismo cuando ya esta instalado y ya se ha usado.
2. **Un artefacto viejo no puede ni declarar su propia ignorancia.** El
   subcomando `dev build-id` entro en `032e9553`. Un binario anterior no lo
   tiene: contesta `unrecognized subcommand` y sale con 2. **MEDIDO con un
   stub.** Ese es el estado MAS VIEJO de todos, y desde dentro es invisible:
   no hay nada que ejecutar que lo delate. El propio documento lo escribia
   («el artefacto que tiene el problema no puede ejecutar el check que lo
   encuentra, y tampoco puede decir que no lo tiene») y la conclusion que
   faltaba era que por eso el check no puede vivir ahi.

La separacion correcta es la que se implemento:

- **HECHO** — el binario dice cual es su commit y si su arbol estaba sucio.
  Un artefacto no puede mentir sobre lo que es, y eso no necesita su permiso
  para ser dato (`dev build-id --format json`).
- **JUICIO** — si ese commit esta antes o despues del HEAD de **este
  checkout**. El punto de referencia es el checkout, luego el juicio es de
  aqui.

## 3. La condicion NO se elimina; se acota y se mide

La condicion —el binario instalado va atrasado respecto al checkout— **no se
puede eliminar**: el binario se instala desde un release y el workspace no
bumpea entre releases, luego en cuanto haya un commit nuevo la condicion
vuelve. Eso no es un defecto pendiente, es la forma del sistema.

Lo que se cierra es el **dano**, que es lo que este documento decia que
importaba: *medir con el binario viejo es evidencia sobre el codigo viejo, con
toda la apariencia de ser evidencia sobre el actual*.

## 4. Lo entregado

- **`scripts/check_binary_freshness.sh`** — el juez del lado del checkout.
  Siete relaciones, mismo vocabulario que el binario pero calculadas aqui:
  `matches` y `ahead` en verde; `behind`, `diverged`, `dirty`,
  `unknown-commit` y `no-build-id` **fallan cerrado**. `--format json` para
  evidencia de gate.
- **`tests/test_binary_freshness_checker.sh`** — el guard, hermetico (repo
  git temporal y stubs), `10 checks 0 fallos`, cableado en el 1b. Con
  **autofalsacion**: el mismo juego de expectativas contra una copia del
  checker con las clasificaciones rotas, y las cinco se notan. Sin ese pase,
  «el checker dice verde» y «el checker no mira nada» serian la misma
  observacion.
- **AGENTS.md §2.3.1** — la regla escrita donde se lee antes de medir, que es
  la tercera salida que este documento proponia y la unica que faltaba.

MEDIDO: contra el binario real da `behind` / `FALLO` / rc 1, nombrando los dos
commits y diciendo **por que la comparacion de versiones sale verde**. Contra
el stub sin subcomando da `no-build-id` / `FALLO`, nombrando `032e9553`.

## 5. El guard cazo un bug real del checker en su primera corrida honesta

`dirty` salia `matches` / **OK** para un binario construido sobre un arbol
sucio. Dos causas, ambas del parser de JSON del checker, y **ninguna la vio la
revision**:

1. `dirty` es un **booleano sin comillas** en el JSON y el `sed` exigia
   comillas, luego salia vacio y el checker caia en la rama de `matches`.
2. Arreglado eso, el valor se capturaba como `true  ` —**con espacios al
   final**— porque la clase de caracteres no los excluia, y la comparacion
   contra `true` es exacta.

La segunda vez que un guard cazaba al autor en la misma sesion, y la segunda
que el sintoma apuntaba al sitio equivocado. La asercion del caso `dirty`
tiene dientes porque se comprobo: si no hubiera estado, el checker habria
dado verde a un artefacto no reproducible.

## 6. El residuo, declarado y NO cerrado

- **Nada obliga a mirar.** `scripts/check_binary_freshness.sh` sale con 1
  cuando el binario no sirve, y `dev doctor --strict` sale con 1, pero **ningun
  runner los invoca**: MEDIDO, `doctor --strict` no aparece en `scripts/`,
  `.github/` ni `githooks/` mas que en un comentario, y el paso 11 de la
  release corre `doctor` en modo advisory y solo mira `bundle_coherence` y
  `all_present`. El cierre de este documento se apoya en la **regla** (que un
  agente lee al cargar AGENTS.md), no en la imposibilidad de saltarsela. Quien
  quiera cerrarlo de verdad tiene que cablear el veredicto en el camino que
  **certifica** una medicion —la verify de un ciclo, o la UAT—, no en el 1b: en
  el 1b el binario del PATH es viejo **por construccion** durante una release,
  luego ahi el gate seria rojo siempre y no mediria nada.
- **La medicion es de esta maquina** y de este checkout. El mecanismo
  (`git merge-base --is-ancestor` en las dos direcciones) es el de cualquier
  clon.
- **`ahead` sale en verde** a proposito: significa que el problema es el
  checkout, no el binario, y un binario mas nuevo que el codigo no puede hacer
  que una medicion sea falsa sobre el.
- **El estado `no-checkout` sale `N/A`, no `FALLO`**, siguiendo el precedente de
  `flat_install` y de `NoCheckout`/`Unknown` en `build_id.rs`: «no se puede
  saber» no debe ser rojo, porque un rojo para «no lo se» es peor que no tener
  check. La diferencia con `unknown-commit` —que SI falla— es deliberada y
  esta razonada en el codigo: sin checkout no hay nada respecto de que ser
  viejo; un artefacto que no se identifica no es una medicion con nombre.
