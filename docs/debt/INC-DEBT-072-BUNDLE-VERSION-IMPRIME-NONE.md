---
id: INC-DEBT-072-BUNDLE-VERSION-IMPRIME-NONE
status: resolved
severity: low
priority: P3
cluster_id: CL-VERIFY
detected_at: 2026-10-04
detected_by: release 2.9.0, al imprimir el estado final del paso 15
resolved_at: 2026-10-05
resolved_in_session: session-83
resolved_by: miniMax Code (mvs_209ab67f202a4a7aa8101165329eed99)
---

# INC-DEBT-072: el estado final del release imprime `bundle: None` en una release correcta

## Que pasa

El paso 15 imprime tres cifras al operador al final de una release que ha
superado los quince pasos:

```
  binary:        sddk 2.9.0
  bundle:        None
  current:       2.9.0
```

`binary` y `current` son correctos. `bundle` sale `None` en una release
**correcta y verificada**: el gate 11 (`sddk dev doctor`) paso con
`binary.bundle_coherence: present = true` y `all_present: True`, y el
gate 9b verifico los nueve assets por la API de GitHub.

## Causa, medida

`scripts/release.sh:2158` imprime:

```python
print(json.loads(open(".../sddk-install.json").read()).get("bundle_version", "?"))
```

El recibo **si** tiene la clave `bundle_version`, y su valor es **JSON
`null`**. MEDIDO sobre el recibo real instalado tras 2.9.0:

```
version          = '2.9.0'
bundle_version   = None
bundle           = True
bundle_path      = None
channel          = 'release'
layout           = 'flat'
tag              = None
```

El `null` es **correcto** y no es el defecto: la instalacion es de tipo
**flat**, luego no existe un directorio de bundle con version propia, y el
propio doctor lo dice con sus palabras —`flat-install: coherencia versionada
no aplicable (receipt.version ok)`—. `bundle_path` y `tag` tambien son
`null` por la misma razon.

El defecto es el **fallback**. `dict.get(clave, "?")` devuelve el valor
guardado cuando la clave existe, y `null` **existe**: sale `None`, no `"?"`.
El `"?` solo habria aparecido si la clave estuviera ausente, luego el
respaldo que el autor escribio para "no lo se" no llega a dispararse
nunca, y la linea imprime la ausencia de un dato **con formato de dato**.

## Correccion de una medicion mia, escrita porque es la clase que mas ha costado aqui

La **primera** redaccion de este documento afirmaba que el recibo "no tiene
clave `bundle_version`" y que el fallo estaba en el nombre de la clave. Es
**FALSO**, y se constato de medir: `sorted(recibo.keys())` incluye
`bundle_version`. Se escribio porque un `.get()` que devuelve `None` se lee
de un vistazo como "clave ausente", que es exactamente el caso que este
documento demuestra que no es. Se deja el error a la vista porque un
documento que solo guarda sus aciertos no sirve para que el proximo no
repita el mismo salto.

## Por que no es cosmetico y por que no es grave

No es cosmetico porque la linea es la que el operador lee para saber **que
bundle quedo instalado**, y `None` no es un dato: induce a abrir un ticket
en una release que acaba de pasar quince gates.

No es grave porque la verdad **si** esta disponible a un comando de
distancia y el gate 11 la comprueba: `sddk dev doctor --format json` da
`binary.bundle_coherence: present`. El fallo es de la linea que resume, no
del artefacto ni del gate.

## Arreglo propuesto (NO aplicado en este bloque)

Que el paso 15 distinga **ausente** de **null**, y que en los dos casos
diga lo que sabe en vez de imprimir la ausencia con formato de dato:

- en layout `flat`, la version del bundle **es** `version` (no hay bundle
  separado), luego la linea deberia imprimir `2.9.0` o `flat (sin bundle
  propio)`, nunca `None`;
- si de verdad no se sabe, el texto tiene que ser `no-declarado-en-el-recibo`,
  que es lo que el autor quiso escribir con `"?"`.

Un guard que lo fije, del tipo que este repositorio ya sabe escribir: el
paso 15 no puede imprimir la cadena `None` en ninguna de sus tres cifras.
**No se anade aqui**: la 2.9.0 ya esta publicada y cerrada, y abrir un
ciclo de release para arreglar la linea que resume lo que esa release acaba
de hacer bien es desproporcionado. Queda para el proximo que toque el
paso 15.

## Lo que NO se afirma

No se afirma que el bundle instalado este mal: el gate 11 lo contradice y
esa medicion manda. No se afirma que el gate 11 tenga un hueco: cubre lo
que tiene que cubrir, y de hecho es el que evita que este `None` sea un
problema real en vez de una linea mal escrita. No se reescribe una release
publicada para arreglar la salida por pantalla de esa release.

---

## CIERRE — session-83, 2026-10-05

### El defecto se reprodujo en vivo, en una release correcta

La **release 2.11.0** reimprimio el fallo en su paso 15, con sus quince
gates en verde:

```
  binary:        sddk 2.11.0
  bundle:        None
  current:       2.11.0
```

Eso convierte una observacion de pantalla en un hecho de release, y fija la
evidencia en un tag concreto en vez de en una sesion.

### MEDIDO sobre el recibo real, otra vez

```
layout          = 'flat'
bundle          = True
bundle_version  = None      <- la CLAVE existe
version         = '2.11.0'
```

`null` sigue siendo la declaracion correcta: en layout flat no hay bundle con
version propia. Lo que estaba mal seguia siendo **el fallback**.

### Lo que se cambio

La logica vive ahora en `scripts/lib/final_state.sh` y **la ejecuta el
guard**, que es la regla que INC-DEBT-074 acababa de pagar en el merge del
changelog: un guard pegado no vigila el codigo, una llamada si.

- **AUSENTE y NULL** se tratan igual a proposito: los dos son "el recibo no
  lo dice". El default del autor existia para el caso "no lo se" y no podia
  dispararse, porque `null` existe como clave.
- **En layout `flat` la version del bundle ES la del binario**, porque no hay
  bundle separado que la tenga. Esa es la verdad que faltaba, y es la que el
  operador necesita.
- Cuando de verdad no se sabe, el texto es `no-declarado-en-el-recibo` — lo
  que el autor quiso escribir con `"?"`.

### MEDIDO, antes y despues, sobre el MISMO recibo

| | `bundle:` |
|---|---|
| release 2.11.0 (literal) | `None` |
| `final_state_figures` sobre ese mismo recibo | `2.11.0` |

### Guard y autofalsacion

- `tests/test_release_final_state_figures.sh` — `PASS=12 FAIL=0`. Siete
  formas de recibo, un invariante sobre la serie entera, un **control** que
  exige que el caso bueno se ACEPTE (un guard que rechazase todo pasaria su
  propia falsacion sin vigilar nada) y tres aserciones de **cableado**.
- `tests/test_release_final_state_figures_mutation.sh` — `PASS=11 FAIL=0
  SKIP=0`, nueve corrupciones, cada una cayendo por su propia razon, mas un
  caso que exige que una mutacion que NO muta se reporte `SKIP` y nunca
  `PASS`.

### Las tres aserciones de cableado, y por que son el guard de verdad

Un guard que mide algo que ningun gate consulta no vigila: informa. C10a,
C10b y C10c existen para que lo que este fichero mide sea literalmente lo que
el operador ve, y las tres se endurecieron **MEDIANTE su propio falsador**:

- C10a buscaba `lib/final_state.sh` a secas, y daba verde con una linea que
  sourcea `lib/final_state.sh.disabled` — es decir, verde con la libreria
  apagada. Ahora exige que la linea **termine** en el `.sh`.
- C10b buscaba el token `final_state_figures` en cualquier parte del
  fichero, y daba verde aunque la llamada no existiera, porque el nombre
  aparece en un comentario y en el nombre de un test. Ahora exige una linea
  de **codigo** que llame a la funcion.
- C10c buscaba el fallback viejo en todo el fichero, y un comentario que lo
  describiera lo hacia fallar a si mismo. Ahora exige que este en codigo.

Las tres correcciones estan en el commit y en el falsador, que es donde se
midieron.

### Una clase que se repitio TRES veces

Un comentario que empieza por el nombre de una herramienta lo lee la
herramienta como codigo:

1. `# shellcheck $SH` citado dentro de `lint_gate.sh` — el fichero que
   arregla el gate de lint no pasaba el gate (SC1073/SC1072).
2. `# shellcheck o un grep leen como codigo` en `release.sh` — el mismo
   error, otra vez.
3. el patron viejo citado literalmente en el comentario de C10c.

Tres veces no es un descuido, es una regla: **un comentario no puede empezar
por el nombre de una herramienta.** Queda escrita en los tres sitios donde
se cumplio.

### Lo que este cierre NO afirma

No se reescribe la salida por pantalla de la 2.11.0, que ya esta publicada.
No se afirma que el bundle de la 2.11.0 estuviera mal: el gate 11 lo
contradice y esa medicion manda. No se toca la escritura del recibo —que
sigue declarando `bundle_version: null` en layout flat, y es lo correcto—:
lo que se arregla es la LINA QUE RESUME, que es lo que la deuda declaraba.
