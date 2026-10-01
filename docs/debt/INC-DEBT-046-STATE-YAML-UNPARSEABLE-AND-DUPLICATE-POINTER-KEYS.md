---
id: INC-DEBT-046-STATE-YAML-UNPARSEABLE-AND-DUPLICATE-POINTER-KEYS
title: el puntero de estado no era parseable por maquina (indentacion) y sus claves `development_head` estaban duplicadas
status: resolved
severity: high
priority: P1
detected_at: 2026-10-01
detected_in_session: session-58
closed_in_session: session-59
component: governance
surface: docs/roadmap/STATE.yaml
references:
  - docs/roadmap/CURRENT.md
  - docs/roadmap/SESSION-JOURNAL.md
fingerprint: "state_yaml_unparseable_and_seven_duplicate_development_head_keys"
---

## Qué es

`docs/roadmap/STATE.yaml` es el puntero de estado que el operador y los agentes
leen para reanudar. Al validarlo con `yaml.safe_load` en session-58, **falló**.
El fichero **no lo parseaba ninguna máquina desde al menos session-46b**: no es
un defecto introducido por esta slice.

**Causa raíz (determinada, no inferida):** la línea con
`development_head: "9d5c13d9"` (session-46b/c3k) tiene **3 espacios** de
indentación en lugar de 2. YAML exige que todas las claves del mismo mapping
compartan columna; ese único carácter hace que el parser aborte con
`expected <block end>, but found '<block mapping start>'` y **el resto del
documento no se lee**. Cualquier consumidor programático recibía un error, no
un estado — y el error seitlement puede descartarse como "el fichero está feo"
en vez de como "el puntero de estado no existe para una máquina".

**Defecto secundario, del mismo grupo:** la clave `development_head` aparece
**7 veces** bajo `source:` (sessions 45, 46, 46b, 48, 54, 55, 56, 57). En YAML,
una clave duplicada no es un error: gana **la última**. Una vez arreglada la
indentación, el valor resuelto era el de **session-45**, no el de session-57: el
puntero vigente que una máquina leía estaba cuatro sesiones atrasado, mientras
`CURRENT.md` sí describía el estado real. La divergencia era invisible al leer
el fichero a ojo, que es exactamente como se han leído estos punteros.

## Por qué es deuda y no un detalle de formato

El paquete de roadmap de la sesión (`AGENTS.md` §10, y la regla del operador
"usa SDDK como autoridad exclusiva del estado operativo") descansa sobre que
`STATE.yaml` sea la fuente legible por máquina. Un puntero que no parsea no es
una fuente: convierte la reanudación en lectura humana de prosa, y reintroduce
precisamente la deriva entre puntero y realidad que la matriz de verdadfulness
existe para eliminar.

## Estado: RESUELTA en session-59 (2026-10-01T11:35Z)

**Cierre completo**, no parcial. Las cuatro condiciones del criterio de cierre
están medidas sobre el árbol final:

| # | Condición | Resultado medido |
|---|-----------|------------------|
| a | El puntero resuelto es el de la sesión actual | `IN_PROGRESS_C3L7_ARCHITECTURE_GATE_RELEASE_BLOCKED_MUSL` |
| b | `grep -c '^  development_head:'` == 1 | **1** (era 9) |
| c | `superseded_development_head` == 1 clave, con los 8 valores previos recuperables | **1** clave · **8** valores recuperables |
| d | Cero claves perdidas en `source:` | diff de key sets **idéntico** contra el backup |

**Falsificador OBSERVED:** reintroducir 3 espacios en una clave `source:`
(`baseline_branch`) reproduce el `ParserError` exacto. La afirmación "el YAML
parsea" tiene contraprueba, no es decorativa.

**Error propio declarado.** Mi primera consolidación **creó** una segunda clave
`superseded_development_head` en vez de concatenar a la preexistente. Con ello la
clave antigua (session-43) quedaba después y **ganaba** en YAML, y el índice de
valores salía vacío. Lo detecté al intentar recuperar los valores (0 recuperados,
cuando debía ser 8); no al escribir. Corregido concatenando a la clave
existente — el patrón ya establecido en este repo, que yo no respeté a la
primera. Es exactamente el mismo modo de fallo que esta INC documenta: una
aserción que se declara sin comprobación.

**Sin pérdida de historia.** Las ocho sesiones afectadas (45, 46, 46b, 48, 54,
55, 56, 57) conservan entrada completa en `SESSION-JOURNAL.md`, y
`superseded_development_head` queda como **índice** — cada valor con su sesión
de origen — no como fuente. La prosa de cada sesión sigue en el diario y en
los recibos de sus slices.

## Divergencia de git reconciliada en la misma sesión

Al recuperar contexto se detectó que `origin/main` contenía `86f2aad7`
("chore(release): bump version") que la rama local **no** contenía: `f78a8bf2`
era el ancestro común y ambos bumps eran de contenido **idéntico**
(`Cargo.toml`, `CHANGELOG.md`, `manifest.toml` verificados por diff vacío). El
remoto salió del step 1c de `release.sh`; el local, del cierre de session-57.

Resuelto con `git rebase origin/main`: historia lineal, `86f2aad7` ahora
ancestro, Git descartó el bump redundante, y el árbol final es **idéntico** al
HEAD de session-58 (`git diff --quiet` sin diferencias: cero bytes perdidos).
El bump duplicado no era un bug del hook sino dos ejecuciones concurrentes del
paso 1c sobre ramas hermanas.

## Criterio de cierre (histórico — cumplido en session-59)

```bash
python3 -c "import yaml; d=yaml.safe_load(open('docs/roadmap/STATE.yaml')); \
  print(d['source']['development_head'])"
```

- (a) el comando imprime el valor de la **sesión actual**, y
- (b) `grep -c '^  development_head:' docs/roadmap/STATE.yaml` == `1`.

Ambas condiciones se cumplieron a la vez en session-59, y se añadieron (c) y
(d) durante la ejecución. Antes del fix de session-58 (a) se cumplía y (b) no:
el fichero parseaba, pero el puntero dependía del orden de las líneas.

## Falsificador

Reinsertar un espacio en cualquier clave `source:` y reejecutar el comando
anterior debe fallar de nuevo. Sin ese falsificador, "el YAML parsea" es una
afirmación sin respaldo — exactamente el patrón que estaINC documenta en otras.
