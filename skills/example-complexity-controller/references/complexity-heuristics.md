# Heurísticas de complejidad de ejemplos

## Conteo de ideas
Para cada ejemplo, listar conceptos técnicos no triviales:
- Concepto foco: el que el capítulo está enseñando.
- Concepto secundario: se usa pero no es el foco (máx. 2).
- Distracción: lo que no aporta al foco → eliminar o mover.

## Señales de OVERCOMPLEX
- Un "hello world" que define 3 traits, 2 enums y un builder.
- Ejemplo que activa 5 feature flags cuando el concepto no las necesita.
- Uso de patrones avanzados (async, macros, unsafe) no introducidos.
- Setup de 60+ líneas antes del código que ilustra el concepto.

## Señales de SCOPE_CREEP
- APIs del ejemplo que no aparecen en el contrato del capítulo.
- Crates que el libro aún no ha introducido.

## Señales de NOISE
- El concepto foco está enterrado entre logging, manejo de errores y configuración.
- Solución: recortar con `tag::` para mostrar solo la región relevante.

## Regla práctica
Si un lector no puede señalar con el dedo "este ejemplo sirve para entender X", el ejemplo es OVERCOMPLEX.
