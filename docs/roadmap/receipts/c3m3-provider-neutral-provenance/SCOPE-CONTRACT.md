# SCOPE-CONTRACT — C3m.3 Provenance provider-neutral

**Cycle:** `p-63676b11dc0ef88f/c3m3-provider-neutral-provenance`
**Estado:** `explore` → decisión tomada y aplicada · **ADR-0155 `accepted`**
**Baseline / HEAD al abrir:** `6266ec52`
**Autorizado por:** operador (session-69p) — *«desbloquea tareas sobre decisiones que quedaron pendientes»*

---

## 1. Qué se afirma

Que el core **no nombra proveedores como conceptos estructurales**, con la forma
que el roadmap fija:

```text
Capability → ProviderIdentity → Evidence
```

y no:

```text
engine → "cognicode"
```

**La medición lo encontró incumplido, en un solo sitio y de forma pública**, y
la decisión está en [ADR-0155](../../architecture/adrs/ADR-0155-CORE-DOES-NAME-PROVIDERS.md).

---

## 2. La medición

| # | Hecho medido | Dónde |
|---|---|---|
| 1 | `pub enum ProviderKind { Null, Fake, CogniCode }` — enum **público** con una variante que nombra un producto | `code_intelligence_port.rs:138` |
| 2 | La variante estaba `#[allow(dead_code)]`: superficie para un consumidor hipotético | ibidem |
| 3 | El doc decía que los valores de producción «**will be added in CC-S1+**»: el plan documentado era seguir añadiendo productos al enum | ibidem, doc |
| 4 | El nombre entraba en **evidencia durable**: `AnalysisBasis::provider_build` se mezcla en el digest del análisis | `code_intelligence_port_fake.rs:307` |
| 5 | El crate **ya tenía la vía neutral**: `circuit_breaker::ProviderIdentity { id, kind, credentials_route, model }` con un `ProviderKind { Llm, Tool, Mock, Deterministic }` que son **categorías** | `circuit_breaker.rs:27-41` |
| 6 | **Dos enums `ProviderKind` en el mismo crate, con significados distintos** | 1 y 5 juntos |
| 7 | El lint `no_knowledge_to_provider_sdk` que todo el mundo cita escanea **5 módulos de knowledge** y **nunca el puerto** | `context_fitness.rs:112-134` |
| 8 | El guard `t_ar_5c_no_cognicode_type_in_sddk_engine` **afirmaba que el nombre estuviera presente**: `assert!(src.contains("ProviderKind::CogniCode"))`, citando el lint de (7) como si lo aplicara «precisamente» | `a6_cc_s1_static_graph_completeness.rs:284-303` |

**Los puntos 6 y 8 son el hallazgo, y los dos son de la misma clase que
ADR-0154:** dos cosas con el mismo nombre y significados distintos, y un guard
que ocupa el sitio del que sí vigilar.

### 2bis. El hazard que se comprobó antes de editar

`ObservationSet` **existe dos veces** en `sddk-engine`:

| Tipo | Deriva | Sitios |
|---|---|---|
| `code_intelligence_port::ObservationSet` | sin `Serialize` | 16 ficheros |
| `observation::types::ObservationSet` | **`Serialize` + `Deserialize`**, dentro de un **payload de ledger `v1` congelado** validado por `EventSchemaRegistry` | 20 ficheros |

Añadir un campo al tipo equivocado —y un cambio por nombre es exactamente eso—
habría roto un contrato de durabilidad versionado a cambio de una mejora de
neutralidad. Se comprobó **antes** de editar, y por eso el cambio se hizo sobre
el tipo del puerto de forma explícita, no con una sustitución por nombre. El
homónimo **queda sin converger** y se dice en el ADR.

---

## 3. La decisión y lo que se aplicó

Resuelta en **ADR-0155**, con diez criterios medidos uno a uno. En el código:

1. `ProviderKind` del puerto pasa a ser **estado de capacidad**:
   `Null | Fake | External`. Sin nombres de producto.
2. `ObservationSet` gana `provider_id: String` — la identidad es **dato**, y lo
   declara el adaptador (`PROVIDER_ID` en el puerto MCP y en el fake).
3. `External` **no es cosmética**: sin ella, quitar la variante del producto deja
   al adaptador real sin forma de decir «hubo un proveedor», y el único camino
   que queda es reportar `Null` o `Fake` — **las dos falsas**.
4. `Null` **no** lleva `provider_id`.
5. El guard se **reescribe**, no se añade: `t_ar_5c_no_provider_name_in_the_core_port`.
6. El lint `no_knowledge_to_provider_sdk` **no se extiende**: mide lo que dice
   medir, y el puerto *debe* hablar con el proveedor — es la costura. Lo que se
   corrige es la cita que lo fixaba como guard de esta propiedad.

---

## 4. Un fallo del guard nuevo, y su resolución

El guard **falló contra su propio doc**. El doc de `External` explica por qué se
quitó la variante y, al hacerlo, **menciona el nombre**:

> Removing `CogniCode` and leaving only `Null | Fake` would have made it
> impossible to say "a real provider produced this"…

La primera versión del check buscaba el token en el cuerpo crudo del enum y
falló. **La resolución correcta no fue borrar la explicación ni bajar la
exigencia**: fue hacer el check preciso, **parseando declaraciones de variante**
en vez de texto. Es la misma distinción que ADR-0154 estableció para las
expansiones retiradas — *citar lo retirado es correcto; usarlo como nombre es
el defecto* — que en esta sesión se aplicó bien una vez y se violó al escribir
este guard.

El check precisa además **afirma que ha encontrado algo que comprobar**: un guard
que no extrae ninguna variante pasa por el motivo equivocado, que es la clase de
defecto que ha apareciéndose en cada falsificación de esta sesión.

---

## 5. Verificación

- `verificar-medicion.py` — comprueba una a una las afirmaciones de §2 contra el
  disco, con **dos controles** del propio instrumento. No publica nada si una no
  se sostiene.
- `falsificar-guard.sh` — aplica **7 mutaciones al source real** sobre una copia
  del repo y exige que el guard caiga en cada una. `shellcheck` limpio.
- Perfil completo: `cargo fmt --all`, `cargo build --workspace --all-targets`,
  `cargo test --workspace`, `cargo clippy -D warnings`.

---

## 6. Lo que queda abierto, y por qué

- **Los dos `ProviderKind` no se unifican.** El del puerto ya no nombra
  productos; el de `circuit_breaker` nunca los nombró. Convergerlos tiene su
  propio radio y su propio contrato. Candidato natural a ADR-0156, y el ADR
  deja escrito que a partir de ahora cada uno significa una cosa distinta.
- **Los dos `ObservationSet` no se renombran.** Mismo motivo, y el segundo tiene
  un payload de ledger `v1` congelado.
- **C3m.1 sigue bloqueada** por la superficie que produce `HostEvent`. Este ciclo
  no la toca.

---

## 7. Stop conditions

1. **Si el guard nuevo sobrevive a alguna de las 7 mutaciones**, se corrige el
   guard, no el listón, y se declara la corrección.
2. **Si aparece un tercer `ProviderKind` o un tercer `ObservationSet`**, la
   medición se repite antes de ampliar el cambio.
3. **Prohibido** tocar `observation::types::ObservationSet` en este ciclo: es
   contrato de durabilidad versionado y su convergencia tiene otro ADR.
