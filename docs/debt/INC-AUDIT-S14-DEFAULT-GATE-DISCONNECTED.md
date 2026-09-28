---
id: INC-AUDIT-S14-DEFAULT-GATE-DISCONNECTED
title: "Un gate correcto, probado en aislamiento y nunca conectado al punto que se ejecuta"
status: open
severity: medium
priority: P2
created: 2026-09-28
discovered_by: session-27 (dry-run real del pipeline de release)
cluster_id: CL-SUPPLY-CHAIN
fingerprint: "sddk_gate_written_tested_never_selected"
---

## Qué es

Una clase de fallo, no un bug puntual. Un gate se escribe, se prueba con
cobertura amplia, y aun así **el punto donde realmente se ejecuta nunca lo
invoca**. La suite pasa en verde y el gate real no protege nada.

`scripts/lib/release_admission.sh` tiene dos implementaciones:

- `release_admission_check` (v1): compara HEAD contra HEAD^. Estricta.
- `release_admission_check_v2`: compara contra el **último tag publicado**.

La suite `tests/test_release_admission.sh` probaba **ambas** con 22 checks
completos, incluyendo casos de bootstrap, published-equal, published-lower,
`query-failed` y el selector v1/v2. Verde.

Y `scripts/release.sh:171` llamaba a `release_admission_check` **sin fijar
modo**, lo que rutea a v1. La v2 era opt-in tras
`SDDK_RELEASE_ADMISSION_MODE=v2`, una env var que **nadie ponía** en ningún
sitio del repo.

El defecto de v1 estaba escrito en la propia cabecera del fichero
(líneas 15-19): *"Fails when a docs-only commit follows a bump"*.

## Por qué importa

Un gate que no está conectado no es un gate. La cobertura de la suite
medía la corrección de las **funciones**, no su **conexión**, y esa
distinción es invisible salvo que alguien afirme explícitamente el punto de
llamada.

La asimetría de coste es la que mata: construir y probar una variante
correcta cuesta mucho; conectarla cuesta un token. Aquí se pagò la parte
cara y se olvidó la barata, durante 7 sesiones.

## Evidencia (OBSERVED, session-27)

```text
# antes del fix
$ bash scripts/release.sh --dry-run
  ✗ release admission refused: REJECT non-monotonic 2.1.0 -> 2.1.0
  exit=1

# el selector real, antes del fix
$ grep -n 'release_admission_check' scripts/release.sh
171:ADMISSION="$(release_admission_check HEAD)" \

# la variante que si funciona, contra el mismo repo
$ SDDK_RELEASE_ADMISSION_MODE=v2 ... release_admission_check_v2 HEAD
ACCEPT last-publish=2.0.1 -> 2.1.0   exit=0
```

Y el caso bueno, que es el que de verdad importa: el release **válido**
era rechazado. No un release malformado, no un atacante: el release
correcto, rechazado por el gate, con el mensaje `non-monotonic` que sugiere
un error de versión donde no lo hay.

## Por qué estuvo latente 7 sesiones

Porque solo se manifiesta cuando un release viene **realmente due**. Antes
de eso, un gate que nunca se ejecuta y un gate que siempre acepta son
indistinguibles desde fuera. El bug se hizo visible exactamente en el
momento en que ya no había margen para iterar.

## Mitigación aplicada

1. `release.sh` invoca `release_admission_check_v2` directamente.
2. **3 checks nuevos** en `tests/test_release_admission.sh` que afirman la
   **conexión**, no la función: que `release.sh` seleccione v2, que el
   default no sea degradable a v1 por una env var vacía, y que un rechazo
   aborte con exit no-cero. Falsificados con mutación cada uno.

Esto cierra la instancia. **La clase sigue abierta**: cualquier gate nuevo
puede repetirla.

## Recomendación (no implementada)

Un guard genérico que falle cuando una función de gate exista en
`scripts/lib/` y tenga cobertura de tests, pero **no aparezca** en el punto
de llamada real. Es la mechanización directa de este incidente. Requiere
decidir el alcance (¿todos los gates? ¿un registro explícito de gates?) y
por eso no se hace aquí: sería un slice propio.

## Relación

- `INC-A5-PUSH-RELEASE-MARKER-FRICTION` — el hook `pre-push` y la admisión de
  release son gates distintos con la misma clase de riesgo. Ahí la tensión
  real era que la cláusula (A) exige cambio de versión en el rango y la
  cláusula (B) solo admite paths de documentación, así que un fix del
  pipeline de release no entra por ninguna de las dos. Se resolvió bumpeando.
- `INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY` — independiente; este incidente
  era un obstáculo **para llegar** a la distribución, no la causa.
