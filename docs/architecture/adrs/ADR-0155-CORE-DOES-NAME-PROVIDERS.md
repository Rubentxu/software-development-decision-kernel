---
id: ADR-0155-CORE-DOES-NAME-PROVIDERS
title: El core no nombra proveedores - capacidad en el tipo, identidad en el valor
status: accepted
proposed_at: 2026-10-03
accepted_at: 2026-10-03
cycle: p-63676b11dc0ef88f/c3m3-provider-neutral-provenance
accepted_by_cycle: p-63676b11dc0ef88f/c3m3-provider-neutral-provenance
supersedes: null
superseded_by: null
component: code-intelligence
surface: crates/sddk-engine/src/code_intelligence_port.rs
closes: []
---

# ADR-0155 — El core no nombra proveedores: capacidad en el tipo, identidad en el valor

- **Status:** `accepted`
- **Date:** 2026-10-03
- **Ciclo:** `p-63676b11dc0ef88f/c3m3-provider-neutral-provenance`
- **Supersedes / relaciona:** ADR-0137 (code intelligence port seam), ADR-0139
  (static enhanced coverage contract), ADR-0154 (el precedente de dos
  definiciones con el mismo nombre)
- **Ruta de roadmap:** C3m.3 — *runtime provider-neutral provenance*
- **Incidencias que este ADR no cierra pero cuya causa explica:**
  INC-DEBT-066 (§3 del SCOPE de C3m.4 nombra este mismo patrón)

---

## Contexto

C3m.3 pide que el core no conozca `chronos-mcp` / `cognicode` / `aiw-s5-capture`
como **conceptos estructurales**, con esta forma:

```text
Capability → ProviderIdentity → Evidence
```

y no esta:

```text
engine → "chronos-mcp"
```

Medido sobre `crates/sddk-engine/src/code_intelligence_port.rs`, el caso es
concreto y no hipotético:

1. `pub enum ProviderKind { Null, Fake, CogniCode }` — un **enum público del
   engine** con una variante que nombra un producto comercial. Su doc decía
   «Production values (e.g. `CogniCode`) will be added in CC-S1+»: el plan
   documentado era **seguir añadiendo nombres de producto a un enum del core**.
2. La variante estaba marcada `#[allow(dead_code)]`: superficie añadida para un
   consumidor hipotético, la misma forma que los 24 módulos de INC-DEBT-065.
3. El nombre además entraba en **evidencia durable**: `AnalysisBasis::provider_build`
   se mezcla en el digest del análisis
   (`code_intelligence_port_fake.rs:307`), así que el nombre del proveedor no
   era solo una etiqueta de memoria, estaba en un hash.
4. **El crate ya tenía la respuesta correcta y en otro sitio.**
   `circuit_breaker::ProviderIdentity { id, kind, credentials_route, model }`, con
   un `ProviderKind { Llm, Tool, Mock, Deterministic }` que son **categorías**.
   O sea: **dos enums `ProviderKind` en el mismo crate con significados
   distintos**, uno neutral y otro con nombres de producto. Es el patrón exacto
   que llevó a ADR-0154 con `KMT`, y aparece por segunda vez en C3m.
5. **El guard no guardaba.** `t_ar_5c_no_cognicode_type_in_sddk_engine` se
   llamaba «sin tipo CogniCode» y su cuerpo hacía
   `assert!(src.contains("ProviderKind::CogniCode"))`: **afirmaba que el nombre
   estuviera presente**. Su comentario decía que el lint
   `no_knowledge_to_provider_sdk` lo aplicaba «precisamente», y ese lint
   (`crates/sddk-cli/tests/context_fitness.rs:112`) escanea **cinco módulos de
   knowledge** —`semantic_graph`, `semantic_node`, `semantic_kind`,
   `vault_boundary`, `why_queries`— y **nunca el puerto**.

## Decisión

1. **`ProviderKind` del puerto expresa estado de capacidad, no identidad de
   producto.** Sus variantes son `Null`, `Fake` y `External`. No hay ningún
   nombre de proveedor en el enum, y el doc deja escrito por qué no puede
   haberlo: un enum cerrado de productos convierte **registrar un proveedor en un
   cambio incompatible de la API pública de `sddk-engine`**, para todo adopter.
2. **La identidad del proveedor es un dato.** `ObservationSet` gana
   `provider_id: String`, que lo declara **el propio adaptador**
   (`code_intelligence_port_mcp::PROVIDER_ID = "cognicode-mcp"`,
   `code_intelligence_port_fake::PROVIDER_ID = "fake-inprocess"`). La
   capacidad vive en el tipo; la identidad, en el valor.
3. **`ProviderKind::External` es obligatoria, no cosmética.** Sin ella, quitar
   la variante del producto deja al adaptador real sin forma de decir «hubo un
   proveedor», y el camino que queda es reportar `Null` o `Fake` — **las dos
   son falsas**. Un enum que no puede expresar la verdad es peor que uno
   demasiado ancho. Por eso el guard nuevo comprueba también que `External`
   existe.
4. **`Null` no lleva `provider_id`.** `provider_id: String::new()` con
   comentario, porque un id junto a «no hay proveedor» es la misma mentira que
   este cambio elimina.
5. **El guard se reescribe, no se añade.** `t_ar_5c_no_cognicode_type_in_sddk_engine`
   pasa a `t_ar_5c_no_provider_name_in_the_core_port` y comprueba tres cosas:
   ningún nombre de proveedor como variante del enum; ningún tipo `pub` del
   puerto llamado como un proveedor; y que la identidad **siga siendo
   expresable** (`provider_id` existe y `External` existe). El tercer punto es
   deliberado: un guard que prohíbe el nombre sin comprobar que quede forma de
   declararlo empuja al primero que llega a escribir una falsehood.
6. **El lint `no_knowledge_to_provider_sdk` no se extiende.** Mide lo que dice
   medir —los módulos de knowledge no dependen de SDKs de proveedor— y el puerto
   **debe** hablar con el proveedor: es la costura. Lo que se corrige es la cita
   que lo-fixaba como guard de esta propiedad. Ampliar el lint al puerto sería
   hacer que una comprobación correcta dijera algo falso.

## Consecuencias

- **Es un cambio incompatible** de un tipo público (`ProviderKind` pierde una
  variante; `ObservationSet` gana un campo). Todo adopter que nombre la variante
  `CogniCode` deja de compilar. **Se acepta a propósito**: la variante estaba
  `#[allow(dead_code)]` y su único consumidor en el repo era el adaptador, que
  cambia con este mismo ADR. Lo que se acepta es el coste, no el defecto.
- **No se toca el otro `ProviderKind`.** `circuit_breaker::ProviderKind` ya es
  neutral y lo usa el router de failover con `ProviderIdentity`. Unificar los
  dos homónimos es trabajo con su propio radio y **no se hace aquí**; lo que se
  deja escrito es que a partir de ahora el del puerto es el único que significa
  «estado de capacidad de un proveedor de inteligencia de código», y el otro
  significa «categoría de ruta de proveedor». Si quedan igual de confusos, la
  convergencia de ambos es la candidata natural a ADR-0156.
- **El digest no cambia.** `provider_build` ya era una `String` con el build del
  proveedor y sigue mezclándose en el digest. Este ADR no toca la derivación del
  hash, sólo de dónde sale el valor: de un string declarado por el adaptador en
  lugar de un enum del core.

## Criterios de aceptación, medidos uno a uno

Cada criterio se ejecutó por separado. `NO MEDIBLE` no es `CUMPLIDO`.

| # | Criterio | Resultado |
|---|---|---|
| 1 | `ProviderKind` del puerto no contiene ningún nombre de proveedor como variante | **CUMPLIDO** — variantes `['Null', 'Fake', 'External']` |
| 2 | La identidad del proveedor es expresable como dato | **CUMPLIDO** — `ObservationSet::provider_id: String`, y el guard lo exige |
| 3 | El enum puede expresar «hubo un proveedor real» | **CUMPLIDO** — `External`; el guard exige su presencia |
| 4 | `Null` no afirma proveedor | **CUMPLIDO** — 3 constructores con `provider_id: String::new()` y comentario |
| 5 | El adaptador real declara su id | **CUMPLIDO** — `PROVIDER_ID = "cognicode-mcp"`, 2 sitios |
| 6 | El guard ya no afirma la presencia del nombre | **CUMPLIDO** — `assert!(src.contains("ProviderKind::CogniCode"))` **eliminado** |
| 7 | El guard no nombra al proveedor que vigila | **CUMPLIDO** — renombrado a `t_ar_5c_no_provider_name_in_the_core_port`; el token se busca por **lista** (`CogniCode`, `Chronos`, `CodeIntelligence`), no por el nombre del producto que el fichero tenía |
| 8 | El lint citado no se usa como guard de esta propiedad | **CUMPLIDO** — su alcance real queda escrito en el guard y en este ADR |
| 9 | El otro `ObservationSet` (payload `v1` congelado) **no** fue tocado | **CUMPLIDO** — ver abajo, y por qué se comprobó antes de empezar |
| 10 | Workspace verde | **CUMPLIDO** — `fmt` 0, `clippy -D warnings` 0, **5435 passed / 0 failed** |

### Falsificación del guard: 8 de 8

El guard reescrito es el único mecanismo que impide que el nombre vuelva al
enum, así que se falsificó con **8 mutaciones al source real** sobre una copia del
repo (`falsificar-guard.sh`, `shellcheck` limpio):

| Mutación | Qué reintroduce | Resultado |
|---|---|---|
| M1 | la variante `ProviderKind::CogniCode` | **detectada** |
| M2 | la variante con **otro** nombre (`Chronos`) | **detectada** |
| M3 | un tipo `pub CogniCode` en el puerto | **detectada** |
| M4 | desaparece `provider_id` — la identidad deja de ser expresable | **detectada** |
| M5 | desaparece `ProviderKind::External` | **detectada** |
| M6 | `Null` vuelve a declarar un id | **detectada** *(tras corregir el guard, ver abajo)* |
| M7 | el guard se debilita a un solo nombre | **detectada vía verificador** *(no es detectable desde el guard, por construcción)* |
| M8 | guard debilitado **y** `Chronos` vuelve a la vez | **detectada** |

**Resultado: 8 detectadas, 0 sobrevividas, 0 no medibles.**

**La primera ronda dio 5 de 7, y las dos supervividas eran defectos del guard, no
del repo** — que es donde está la parte que conviene leer:

- **`Null` con un id sobrevivió** porque el código estaba escrito y comentado, pero
  **nada lo vigilaba**. Se añadió el punto (4) al guard. *Un comentario no es un
  guard.*
- **El guard debilitado a un solo nombre sobrevivió**, y **no se corrigió porque no
  se puede**: un guard más débil sobre un fuente sano **debe** dar verde, y eso es
  lo correcto. Se midió **desde el verificador**, que trata la lista de nombres
  como propiedad *del guard* y no del puerto, y se declaró explícitamente
  `NO MEDIBLE` para el guard en vez de contarlo como una sobrevida que no lo era.
  M8 existe por eso: la combinación que sí importa, un guard más débil **y** un
  defecto reintroducido.

**Y el guard falló dos veces contra sí mismo antes de estar en verde**, y ninguna de
las dos se resolvió bajando el listón:

1. **Contra su propio doc.** El doc de `External` explica por qué se quitó la
   variante y al hacerlo menciona el nombre, así que la primera versión del check
   —que buscaba el token en el cuerpo crudo— falló. El arreglo fue **parsear
   declaraciones de variante**, y añadir un helper `sin_comentarios()` que
   distingue **citar lo retirado de usarlo**.
2. **Contra el formateador.** El check de `Null` contaba tres líneas hacia adelante y
   `cargo fmt` partió `provider_id: String::new(),` en varias, con lo que encontró
   una cadena vacía. Se reescribió sobre texto normalizado: *un check atado al
   formato no es un check sobre la propiedad, es un check sobre cómo está escrito
   hoy.*

### Criterio 9, y por qué casi se convierte en un incidente

`ObservationSet` **existe dos veces en `sddk-engine`**:

| Tipo | Deriva | Dónde se usa |
|---|---|---|
| `code_intelligence_port::ObservationSet` | `Debug, Clone, Default, PartialEq, Eq` — **no serializable** | 16 ficheros |
| `observation::types::ObservationSet` | `Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize` | 20 ficheros |

El segundo está dentro de un **payload de ledger `v1` congelado**, registrado y
validado por `EventSchemaRegistry`, con un test de durabilidad de roundtrip
(`tests/observation_set_durability.rs`). Añadir un campo al tipo equivocado —y un
cambio por nombre es exactamente eso, porque el nombre es el mismo— habría roto
un contrato de durabilidad versionado a cambio de una mejora de neutralidad.

Se comprobó **antes** de editar, no después, y por eso el cambio se hizo sobre el
tipo del puerto de forma explícita y no con una sustitución por nombre. El
homónimo queda **sin converger**: renombrar cualquiera de los dos es un cambio
con su propio radio y su propio contrato, y no se arrastra dentro de este ADR.

## Lo que este ADR NO hace

- **No unifica** los dos `ProviderKind` (§ Consecuencias).
- **No renombra** los dos `ObservationSet` (§ Criterio 9).
- **No toca** `circuit_breaker::ProviderIdentity` ni la ruta de failover, que ya
  es neutral.
- **No cambia** la derivación del digest de análisis.
- **No declara** que nombrar al proveedor en un *adaptador* sea un defecto: es
  donde el nombre vive, y por eso el puerto deja de hacerlo.
