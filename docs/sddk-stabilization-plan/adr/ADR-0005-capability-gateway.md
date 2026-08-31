# ADR-0005 — Gateway de capacidades y clasificación de efectos

**Estado:** aceptada
**Fecha:** 2026-08-03

## Contexto

Los agentes pueden ser influenciados por datos no confiables y no deben disponer de acceso directo a operaciones externas.

## Decisión

Toda operación externa debe representarse como una capacidad tipada. El gateway realizará:

1. Validación de schema.
2. Comprobación de estado y fase.
3. Evaluación de política.
4. Resolución de aprobación.
5. Registro previo del efecto.
6. Ejecución sin shell arbitrario.
7. Saneamiento de salida.
8. Verificación de postcondición.
9. Emisión de receipt.

## Clasificación

### Riesgo

`low`, `medium`, `high`, `critical`.

### Consecuencia

- `R0`: lectura.
- `R1`: cambio local reversible.
- `R2`: cambio local persistente.
- `R3`: cambio externo compartido.
- `R4`: destructivo o difícilmente reversible.

Riesgo y consecuencia son dimensiones independientes.

## Consecuencias

Las acciones R3 y R4 no podrán autoaprobarse por defecto.
