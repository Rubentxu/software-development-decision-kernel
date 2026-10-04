---
id: INC-DEBT-072-BUNDLE-VERSION-IMPRIME-NONE
status: open
severity: low
priority: P3
cluster_id: CL-VERIFY
detected_at: 2026-10-04
detected_by: release 2.9.0, al imprimir el estado final del paso 15
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
