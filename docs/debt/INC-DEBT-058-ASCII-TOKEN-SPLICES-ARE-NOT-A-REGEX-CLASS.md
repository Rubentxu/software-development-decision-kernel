---
id: INC-DEBT-058-ASCII-TOKEN-SPLICES-ARE-NOT-A-REGEX-CLASS
title: La corrupcion de prosa en ASCII (token pegado dentro de una palabra) no es una clase automatizable, y anadir el guard seria peor que no tenerlo
status: open
severity: low
priority: P3
detected_at: 2026-10-02
detected_in_session: session-66
component: documentation
surface: docs/
fingerprint: "ascii_token_splices_ambiguous_with_legitimate_identifiers"
related: [INC-DEBT-057]
references:
  - tests/test_docs_script_contamination.py
  - docs/architecture/adrs/ADR-0148-DYNAMIC-EXPANSION-ROOT-MODULE.md
  - docs/debt/INC-DEBT-021-MUSL-ASSET-NAME-LIE.md
  - docs/debt/INC-DEBT-043-EXPLICIT-CYCLE-BOUND-TO-NONEXISTENT-CYCLE.md
  - docs/debt/INC-DEBT-046-STATE-YAML-UNPARSEABLE-AND-DUPLICATE-POINTER-KEYS.md
---

# Por qué esta deuda NO se cierra con un regex

INC-DEBT-057 protege la clase «prosa española con un sistema de escritura no
latino pegado en el lugar de una palabra», porque un carácter CJK o cirílico en
esa posición nunca es legítimo. Esa clase se detecta con fiabilidad y por eso
tiene guard.

Esta es la clase **vecina** que el guard no cubre, y que apareció seis veces en
una sola sesión: un token de otro idioma o de otro término pegado dentro de una
palabra española, en ASCII.

| lo encontrado | fichero |
|---|---|
| `suBINspección` por «su inspección» | `crates/sddk-cli/src/project_alias.rs` (nuevo, corregido) |
| `unaHop` por «una cadena de un salto» | ídem |
| `Se Writing` por «Se escribe» | `docs/roadmap/receipts/c3m-identity-alias/PRE-FLIGHT.md` |
| `el Justifica` por «el justifica» | ídem |
| `había Forced` por «había obligado» | ídem |
| `reescrituraodrTurn` y `whose` | `tests/test_docs_script_contamination.py` (el guard, corregido) |

## La medición que decidió no añadir la regla

Candidata: una palabra en minúsculas con una mayúscula incrustada. Se barrió
`docs/` (415 ficheros, excluyendo `docs/history/` y `SESSION-JOURNAL.md`):

```text
coincidencias brutas:                94
fuera de code spans (`` ` ``):       41
```

Y los 41 se reparten así:

- **Mayoría legítimas**: `isDraft`, `isPrerelease`, `tagName`, `bomFormat`,
  `specVersion`, `payloadPath`, `outputSignature`, `tlogUpload`, `camelCase`.
  Son nombres de campo de JSON, flags de `gh release` y prosa inglesa — el
  repo es bilingüe y ese uso es correcto.
- **Cinco corruptiones reales**: `puedeAsociar` (ADR-0148:67),
  `comoLatest` (INC-DEBT-021:309), `seResolved` (INC-DEBT-040:251),
  `queMHAgent` (INC-DEBT-043:62), `estaINC` (INC-DEBT-046:114).

**`comoLatest` e `isDraft` tienen exactamente la misma forma**: minúsculas,
mayúscula, minúsculas. Ninguna expresión regular los separa, porque la
diferencia es de *idioma*, no de *sintaxis*, y detectar idioma no es lo que un
guard de este repo puede hacer sin falsos positivos.

Excluir los que van dentro de `` ` `` no basta: baja a 41, y los 41 siguen
mezclando los dos grupos.

## Por qué no se añade con allowlist

Se podría, y sería el mismo patrón que INC-DEBT-057: 36 entradas `ruta:línea`
con su motivo. Pero el guard seguiría **sin detectar la corrupción nueva**,
que es justo lo que justifica su existir: la allowlist solo documenta lo que ya
se revisó a mano. Un guard que nunca falla es un informe que nadie lee, que es el
defecto que este guard vino a cerrar.

Y a diferencia de INC-DEBT-057, aquí **la palabra original sí es a veces
reconstruible** (`puedeAsociar` → «puede asociar» es inequívoco) y a veces no
(`queMHAgent`, `estaINC`: no hay forma de saber si el original era «MHAgent»,
`MHC-Agent` u otra cosa). Una regla de corrección automática sobre esa clase
fabricaría texto normativo en los casos no reconstruibles.

## Lo que sí se puede hacer, y está pendiente

Las cinco corrupciones de `docs/` quedan **sin corregir**, junto con la razón:
algunas son reconstruibles y otras no, y corregirlas a ciegas sería el mismo
error que INC-DEBT-057 ya previene. Quien escribió cada frase tiene que decir
qué quiso decir.

La mitigación real mientras tanto es la de siempre y la que ya funciona: **barrer
a mano antes de cada commit** los ficheros que se tocan. Es un control de
persona, no un invariante del repositorio, y por eso esta deuda queda abierta en
vez de cerrada.

## Lo que la convertiría en automatizable

Un detector de idioma, no una expresión regular: marcar como sospechosa una
palabra con mayúscula incrustada que NO esté en un contexto JSON/flag y que su
ventana de alrededor sea mayoritariamente española. Es un clasificador, con su
propio corpus y sus propios falsos positivos, y no es material para un guard de
`tests/` que se ejecuta en cada commit.
