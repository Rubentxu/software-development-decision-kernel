# PRE-FLIGHT — VA8 `cl-candidate-handoff`

**Cycle:** `p-63676b11dc0ef88f/version-coherence-068`
**Date:** 2026-10-05
**Predecessor:** VA7 `fae6bb9c` (ADR-0161, cerrado)

---

## Readiness: READY

---

## Qué cierra este bloque

VA6 dejó el tipo `CandidateHandoff` y la puerta `may_publish`, ambos escritos,
probados y **sin consumidor de producción**. MEDIDO: `CandidateHandoff` no lo
construye nadie — `grep -rn "CandidateHandoff" crates/` sólo devuelve su
definición y su falsificador. Un tipo que nadie construye es documentación con
tipos, y este bloque existe para que deje de serlo.

El valor: un productor puede **emitir** un sobre y un certificador puede
**recibirlo**, sin que ninguna de las dos partes tenga que saber cómo es el
lanzamiento de la otra.

## Qué NO es objetivo

- **No inventa la identidad del productor.** `external_handoff_type` y
  `external_handoff_digest` son hechos del sistema que produjo el material. Si
  SDDK los rellenara, estaría fabricando la identidad de otro sistema — que es
  exactamente la «segunda autoridad» que ADR-0160 prohibió en el sobre. Son
  banderas **obligatorias**, no opcionales con valor por defecto.
- **No duplica el registro de versión.** `product_version` sale de la **misma**
  llamada `version_registry().resolve(...)` que usan `release plan` y
  `release version matches`. Una segunda resolución sería una segunda
  autoridad, y la que se escribe en el sobre sería la que cuenta.
- **No reescribe las reglas del productor.** El sobre lleva hechos, no
  veredictos. Si esto hiciera judgments sobre si el material es publicable, el
  certificador estaría leyendo la opinion del productor sobre sí mismo.
- **No es un publicador.** Entregar un sobre no es publicar. La puerta de
  publicación sigue siendo `may_publish`, y este comando **no** la llama ni la
  suplanta.

## Superficie mínima declarada

Se toca lo mínimo, y todo lo demás es reutilizado:

| Fichero | Qué |
|---|---|
| `crates/sddk-cli/src/release_cmd.rs` | `sddk release handoff`, y **solo** eso |
| `crates/sddk-domain/src/release_ref.rs` | **desviación medida**, ver abajo |

Y se reutiliza, sin tocar:

- `CandidateHandoff`, `HandoffArtifact`, `RoleRefusal` — `release_role.rs`
- `ReleaseRef`, `CandidateSequence`, `SourceRevision`, `VersionNaming` — `release_ref.rs`
- `may_reach`, `ceiling` — la **puerta existente** de VA6
- `binds`, `named_rejection` — la puerta de ADR-0159
- `resolve_release_target`, `version_registry`, `resolve_naming`, `resolve_role`
  — las mismas funciones que ya usan plan, apply y matches

**Ningún fichero nuevo en el dominio.** La puerta que este comando consulta ya
existe y ya está falsificada; si hiciera falta una nueva, eso sería una señal de
que el bloque se pasó de alcance.

## La desviación, y por qué no es una ampliación de capricho

MEDIDO ejecutando el comando, no leyendo el código: `PrefixedCandidate` se podía
**usar** —`name_for` y `binds` lo conocían— pero no **construir**. El único
`parse` acepta `v_prefixed` y `exact`, y ninguno de los dos tiene sitio para una
secuencia.

Consecuencia medida: **un handoff de candidato era imposible**. `--sequence` sólo
podía producir un `NamingHasNoRoomForCandidates`, luego el campo `sequence` de
`CandidateHandoff` —que VA6 escribió y este bloque tenía que honrar— era
inalcanzable desde fuera del crate.

Se añade **un constructor**, `VersionNaming::prefixed_candidate(prefix,
separator, marker)`. No se añade una convención, ni un default, ni una política:
se hace construible una variante que el dominio ya tenía y ya falsificaba.

La lección es la misma que la de VA7 y merece decirse porque es la **segunda vez
en este bloque**: el tipo estaba probado y su falsador estaba verde, y aun así
era inalcanzable. Un test que ejercita un tipo **desde dentro del módulo que lo
declara** no puede observar esa clase de hueco; hace falta un consumidor de
fuera, y aquí lo fue el comando.

## Riesgo de datos

**Bajo y acotado.** El comando es de lectura por construccion (lee el target,
la versión y el HEAD) y su única escritura es el `--out` que el operador
pide explícitamente. Sin `--out` no escribe nada: el sobre sale por stdout.

Se mide igual: la prueba escribe en un `TempDir` y comprueba que el árbol del
target no cambia, como ya hace `resolver_no_escribe_en_el_arbol`.

## STOP conditions

1. Que haga falta un concepto de dominio nuevo (una noción de «producir» que no
   sea `may_reach` + techo). **Ya comprobado que no hace falta**: el techo del
   rol *es* la puerta de producción, y reusarla es lo que evita una segunda
   autoridad.
2. Que el sobre exija que SDDK interprete `external_handoff_type`. Si lo
   hiciera, sería un kernel que ha adoptado el vocabulario de un productor.
3. Que construir el sobre necesite resolver la versión por una vía distinta de
   la de `release plan`.

## El riesgo de este bloque, escrito antes de empezar

El modo de fallo no es que el sobre esté mal formado. Es que **parezca más
completo de lo que es**: un sobre con nueve campos, todos rellenos, que no
distingue «SDDK verificó esto» de «el productor afirmó esto».

La defensa es estructural y no de estilo: los campos que SDDK no puede saber
son **obligatorios**, de modo que no puedan quedar rellenos por defecto con un
valor inventado. Un sobre que SDDK puede construir entero sin hablar con nadie
es un sobre que dice cosas que nadie dijo.

## Verificación prevista

- Falsador nuevo `handoff_falsification` con mutantes sobre: handoff sin
  artefactos (sobre vacío), digest que no corresponde a los bytes, secuencia
  que no corresponde al nombre bajo la convención declarada, y puerta de rol
  saltada.
- Pruebas de CLI sobre `/tmp`: el sobre escrito se relee y coincide.
- Suite completa del workspace + clippy + fmt, porque el bloque toca `release_cmd`.
