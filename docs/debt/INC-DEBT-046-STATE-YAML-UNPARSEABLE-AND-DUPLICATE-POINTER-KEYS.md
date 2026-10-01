---
id: INC-DEBT-046-STATE-YAML-UNPARSEABLE-AND-DUPLICATE-POINTER-KEYS
title: el puntero de estado no era parseable por maquina (indentacion) y sus claves `development_head` estaban duplicadas
status: open
severity: high
priority: P1
detected_at: 2026-10-01
detected_in_session: session-58
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

## Estado de la corrección en session-58

**Corregido en session-58** (parte de esta slice):

1. Indentación de la línea 17 normalizada a 2 espacios.
2. `yaml.safe_load` verificado: OK. `development_head` resuelto =
   `DONE_C3L6_X07_PROCESS_BOUNDARY_RELEASE_BLOCKED_MUSL` (session-58),
   `current_sha` = `bdb2ac65`, `workspace_version_at_current` = `2.5.0`.
3. `updated_at` a `2026-10-01T11:05:00Z`; `current_sha` reconciliado al HEAD
   real; `workspace_version_at_current` corregido de `2.4.2` (obsoleto) a
   `2.5.0` declarada-no-publicada; blocker de musl añadido a la lista.
4. La clave de session-58 se añadió **al final** para que sea la que resuelve.

**SIN CERRAR (motivo declarado):** la **consolidación** de las 7 claves
`development_head` duplicadas en una sola, moviendo las anteriores a
`superseded_development_head`. No se hizo aquí por dos razones concretas:

- Reordenar historia de punteros es un cambio de contrato de
  `STATE.yaml` (qué clave es autoritativa), no un fix de formato; exige su
  propia decisión.
- Todas las clavesSESSION-ANTIGUAS son **evidencia de sesión** y el diario es
  quien la conserva (§10: no reescribir historia). Borrarlas del puntero sin
  transladarlas reduce la trazabilidad que el propio esquema pretende preservar.

Lo que **no** se hace, y es la parte que deja esta deuda abierta: seguir
acumulando claves duplicadas. La siguiente sesión que escriba en este fichero
debe consolidar, no añadir una octava.

## Criterio de cierre (verificable)

```bash
python3 -c "import yaml; d=yaml.safe_load(open('docs/roadmap/STATE.yaml')); \
  print(d['source']['development_head'])"
```

- (a) el comando imprime el valor de la **sesión actual**, y
- (b) `grep -c '^  development_head:' docs/roadmap/STATE.yaml` == `1`.

Ambas condiciones deben cumplirse a la vez. (a) sin (b) es el estado actual tras
el fix de session-58: parsea, pero depende del orden de las líneas.

## Falsificador

Reinsertar un espacio en cualquier clave `source:` y reejecutar el comando
anterior debe fallar de nuevo. Sin ese falsificador, "el YAML parsea" es una
afirmación sin respaldo — exactamente el patrón que estaINC documenta en otras.
