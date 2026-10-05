---
id: ADR-0162-UN-SOBRE-LO-CONSTRUYE-EL-NUCLEO-Y-LO-FIRMA-EL-PRODUCTOR
title: Un sobre lo construye el núcleo y lo firma el productor, y el núcleo no rellena lo que no sabe
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-068
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-068
supersedes: null
superseded_by: null
extends: [ADR-0157, ADR-0158, ADR-0159, ADR-0160, ADR-0161]
component: release
surface: crates/sddk-cli/src/release_cmd.rs
closes: []
---

# ADR-0162 — Un sobre lo construye el núcleo y lo firma el productor

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0159 (un tag no es una versión), ADR-0160 (roles de release),
  ADR-0161 (el veredicto no explica su porqué)
- **Superficie:**
  - `crates/sddk-cli/src/release_cmd.rs` — `sddk release handoff`
  - `crates/sddk-domain/src/release_ref.rs` —
    `VersionNaming::prefixed_candidate`
- **Cierra el pendiente** que ADR-0160 dejó declarado: el tipo `CandidateHandoff`
  y su puerta estaban escritos, falsificados y **sin consumidor de producción**.

---

## Contexto

ADR-0160 decidió que un release puede tener varios responsables y escribió el
sobre: `CandidateHandoff`, con su `HandoffArtifact`, su `RoleRefusal` y la puerta
`may_publish`.

MEDIDO antes de escribir una línea de este bloque, porque un tipo sin consumidor
es documentación con tipos:

```
$ grep -rn "CandidateHandoff" crates/
sddk-domain/tests/release_role_falsification.rs:53   (import)
sddk-domain/tests/release_role_falsification.rs:471  (construcción en test)
sddk-domain/src/release_role.rs:262                  (definición)
```

Definición y falsificador. **Nadie lo construía.** Un tipo con nueve campos, probado y sin productor, es exactamente el estado en el que un ADR se cree
implementado.

## Decisión

### 1. El sobre se construye; lo que el productor firma, se exige

SDDK **calcula** lo que son hechos —el target, la versión, la revisión del
árbol, el sha256 de cada artefacto— y **exige** lo que son hechos de otro
sistema: `--external-type` y `--external-digest`.

La asimetría es el diseño entero, y por eso esos dos campos son **obligatorios**
y no opcionales con default:

> Con default, el sobre se podría rellenar entero sin hablar con nadie. Y un
> sobre que dice cosas que nadie dijo es peor que no tener sobre, porque el
> certificador lee la firma del productor y no hay productor detrás.

Un default invisible en el crate que sostiene los demás defaults no es un
default: es una segunda convención sin dueño.

### 2. Las dos puertas son las que ya existían

| Puerta | De dónde | Qué rechaza |
|---|---|---|
| `may_reach(channel)` | ADR-0160 | un productor que entrega más allá de su techo |
| `binds(reference, version, naming)` | ADR-0159 | un sobre cuya referencia no nombra su versión |

Ninguna nueva. Producir hasta un canal es exactamente lo que dice el techo del
rol, y «¿nombra esta referencia a esta versión?» ya tenía respuesta. Una función
`may_produce()` nueva habría sido una segunda respuesta a una pregunta que el
techo ya contestaba, y las dos divergirían.

### 3. La versión del sobre es LA versión del repositorio

`product_version` sale de la **misma** llamada `version_registry().resolve(...)`
que usan `release plan` y `release version matches`. Una segunda resolución
sería una segunda autoridad, y la que acaba dentro del sobre es la que el
certificador leería como si la hubiera emitido SDDK.

---

## El hueco que encontró la medición, y la lección que es la segunda del bloque

MEDIDO ejecutando el comando, no leyendo el código:

```
$ sddk release handoff --tag v2.1.0-rc1 --sequence 1 …
error: this release reference names a candidate, and the declared naming has no
place for a candidate sequence.
```

**Un handoff de candidato era imposible.** `PrefixedCandidate` se podía **usar**
—`name_for` y `binds` lo conocían— pero no **construir**: el único `parse`
acepta `v_prefixed` y `exact`, y ninguno tiene sitio para una secuencia. Luego el
campo `sequence` de `CandidateHandoff` —que VA6 escribió y este bloque tenía que
honrar— era inalcanzable desde fuera del crate.

Se añade **un constructor**, `VersionNaming::prefixed_candidate(prefix,
separator, marker)`. No se añade una convención ni una política: se hace
construible una variante que el dominio ya tenía y ya falsificaba.

**La lección es la misma que la de ADR-0161, y es la segunda vez: el tipo
estaba probado, su falsificador estaba verde, y aun así era inalcanzable.** Un
test que ejercita un tipo **desde dentro del módulo que lo declara** no puede
observar esa clase de hueco. Hace falta un consumidor de fuera, y aquí lo fue el
comando. Un arnés que solo conoce lo que el código ya conoce no puede
descubrirle un hueco al código.

### Y un consejo equivocado, encontrado por el mismo camino

MEDIDO: un rechazo de lockstep proponía `--naming <v_prefixed|exact>` cuando la
convención activa era `prefixed_candidate`, que **no** es seleccionable por ese
flag. Un rechazo que sugiere una salida imposible es peor que un rechazo sin
consejo, porque gasta la confianza del lector en un camino que no lleva a
ninguna parte. Ahora el consejo nombra las tres banderas que sí aplican.

---

## Consecuencias

### Positivas

- `CandidateHandoff` tiene por fin el productor que le faltaba, y su puerta tiene
  el consumidor que justificaba haberla escrito.
- El digest de cada artefacto es verificable por cualquiera que tenga los bytes:
  no es una opinión sobre el material, son los bytes.
- El sobre es **transportable**: no dice qué significa un candidato en el sistema
  que lo produjo, y no pretende saberlo.

### Negativas y costeadas

- **Entregar no es publicar, y el sobre no lo confunde.** Este comando **no**
  llama a `may_publish`. Lo dice en su propio `NOT_CHECKED`: *nothing was
  published, and nothing was certified*.
- **Los artefactos se hashean, no se juzgan.** Que el material sea lo que un
  certificador necesita es su pregunta, y está escrito en la salida para que
  nadie lo lea como un aval.
- **El digest externo se acepta, no se recalcula.** SDDK no tiene la
  serialización del productor y, por tanto, no puede comprobarlo. Decirlo es
  parte del contrato.

### Deuda declarada, no abierta aquí

- `PrefixedCandidate` sigue fuera de `VersionNaming::NAMED`, así que
  `--naming` no lo selecciona. Es intencionado: tiene tres parámetros y un
  nombre de una palabra los escondería. Se construye por sus partes.

---

## Verificación

| # | Ley | Suite |
|---|-----|-------|
| H1 | El digest del artefacto es el de los bytes | `m1_el_digest_del_artefacto_es_el_de_los_bytes` |
| H2 | Cambiar los bytes cambia el digest | `m2_cambiar_los_bytes_cambia_el_digest` |
| H3 | Un productor no entrega en `Stable` | `m3_un_productor_no_puede_entregar_en_stable` |
| H4 | La referencia nombra la versión del sobre | `m4_la_referencia_tiene_que_nombrar_la_version_del_sobre` |
| H5 | SDDK no inventa la identidad del productor | `m5_sddk_no_inventa_la_identidad_del_productor` |
| H6 | El sobre no tiene una segunda versión | `m6_el_sobre_no_tiene_una_segunda_version` |
| H7 | El sobre escrito es el que se resume | `el_sobre_escrito_es_el_mismo_que_el_impreso` |
| H8 | `rc0` no es un candidato | `una_secuencia_cero_no_es_un_candidato` |

### Falsificación

Seis mutantes y dos leyes con nombre. **H2 existe porque H1 solo no distingue**:
un digest constante pero bien formado pasaría H1 sin que nadie comprobara que
depende del contenido. Es la tercera vez en este bloque que un falsador
sobreviviente se explica por una fila que no distinguía, y la segunda vez que la
fila correctora se eligió leyendo por qué la anterior no caía.

**H5 tiene dos mitades a propósito.** La negativa —sin los flags no hay sobre—
comprueba que no haya defaults. La positiva comprueba que lo que llega se copia
**verbatim**, con los espacios incluidos: un núcleo que normaliza el vocabulario
del productor es un núcleo que ha adoptado el vocabulario de un productor.
