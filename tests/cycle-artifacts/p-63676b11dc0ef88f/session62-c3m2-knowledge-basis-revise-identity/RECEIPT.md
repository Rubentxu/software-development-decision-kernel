# RECEIPT — C3m.2: `KnowledgeBasis::revise` decía una cosa y hacía otra

**Slice:** `session62-c3m2-knowledge-basis-revise-identity`
**Fecha:** 2026-10-01 · **Baseline:** `0a2ac6e0` (HEAD == origin/main al abrir)
**Workflow:** `A-lite` · **UAT cubierto:** AT-UAT-019 · **R2** de la matriz

---

## §1 El defecto: el contrato de `revise` era falso

La matriz R2 nombra el defecto: *"`KnowledgeBasis::revise` con docs/código
discrepantes (C3m.2)"*. Medido en `crates/sddk-engine/src/knowledge.rs`:

**Lo que el doc afirmaba** (antes de esta slice):

```rust
/// Produce a new basis at a strictly greater `at` time. … a fresh basis is
/// returned with the same assertion set and a new basis hash (because the
/// `revised_at` participates in the hash).
pub fn revise(self, at: EventTime) -> Result<Self, KnowledgeError> {
    …
    new_basis.revised_at = at;
    new_basis.basis_hash = derive_basis_hash(&new_basis.assertions);
```

**Lo que el código hacía:**

```rust
fn derive_basis_hash(assertions: &BTreeMap<KnowledgeId, KnowledgeAssertion>) -> BasisHash {
    let mut hasher = Sha256::new();
    hasher.update(b"sddk.knowledge.basis.v1\n");
    hasher.update((assertions.len() as u64).to_le_bytes());
    for (id, assertion) in assertions { … }
```

La función **sólo recibe `assertions`**. `revised_at` no tenía forma de
participar: no era un parámetro. La frase "because the `revised_at`
participates in the hash" describía una propiedad que el código no tenía.

### RED empírico (antes del fix)

```text
PROBE before=4de2152916049f2b…  after=4de2152916049f2b…  equal=true
assertion `left != right` failed: revise() must change the basis hash
test result: FAILED. 0 passed; 1 failed
```

Dos bases con **contenido idéntico** revisadas a **tiempos distintos** eran
**indistinguibles por hash**.

## §2 Por qué importa más allá de la doc

`KMT::evaluate` compara los hashes **antes** que los timestamps:

```rust
if observed.basis_hash == expected.basis_hash {
    return KmtStatus::Fresh { … };
}
```

Como el hash era la identidad del basis, **una revisión puramente temporal era
invisible al freshness**: el evaluador veía "mismo hash" y devolvía `Fresh`
antes de mirar `revised_at`. La monotonía del tiempo sí era correcta y sí
estaba testeada (`…_revise_with_stale_time_is_rejected`) — por eso el defecto
era invisible a la suite: el test existente comprueba la mitad del contrato que
funcionaba, y la mitad que fallaba no tenía aserción.

## §3 La corrección

`derive_basis_hash` pasa a ser `derive_basis_hash_at(assertions, revised_at)`.
Todos los sitios vivos pasan `Some(revised_at)`: `empty`, `insert` y `revise`.

**El dominio es `v2` y el tag es load-bearing.** Con `v1` las identidades
persistidas de un despliegue anterior compararían **distintas** contra un basis
v2 — que es el comportamiento correcto: **fallan cerradas** hacia
re-verificación, en vez de coincidir por accidente y dar un `Fresh` falso. Un
tag de versión hace que los dos dominios nunca colisionen.

`revised_at: None` se conserva **sólo para reproducir el dominio v1** y poder
afirmar en un test que sigue siendo reproducible y que no colisiona con v2.
**Ningún camino vivo lo construye**; por eso clippy no lo marca (se usa en
`#[cfg(test)]`) y su doc dice explícitamente que existe para reproducibilidad,
no para acuñar identidad.

## §4 GREEN y falsificadores

`knowledge::` → **24 passed / 0 failed / 1 ignored** (era 20 antes de añadir 4).

| # | Falsificador | Resultado |
|---|---|---|
| **F19** | Revertir `revise` al hash legacy (v1, sin tiempo) | **OBSERVED** — 2 FAIL |
| **F20** | Que la derivación **ignore** el tiempo por completo (el defecto raíz) | **OBSERVED** — **4 FAIL** |

**F20 es el que cubre el defecto real**: los cuatro tests de identidad caen
cuando el tiempo deja de participar, no sólo cuando `revise` deja de pasarlo.

Los cuatro tests nuevos son un *property-set*, no un caso:

1. `revise_changes_the_basis_hash_when_content_is_unchanged` — el contrato del doc.
2. `revisions_at_different_times_have_distinct_identities` — dos revisiones distintas no colisionan.
3. `an_untouched_basis_is_distinguishable_from_a_revised_one` — la identidad de contenido no sustituye a la de revisión.
4. `falsifier_f19_identity_is_load_bearing` — el dominio v1 sigue siendo reproducible y no colisiona con v2.

## §5 Perfil de verificación

```text
cargo test -p sddk-engine --lib knowledge::     24 passed; 0 failed; 1 ignored
cargo test -p sddk-engine                      2376 passed; 0 failed; 11 ignored
cargo fmt --all -- --check                     limpio
cargo clippy -p sddk-engine --all-targets -D warnings   exit 0
cargo test --workspace                         (ver §7)
```

**`sddk-engine` completo verde (2376/0)**: el cambio de dominio no rompió
ninguno de sus consumidores — incluidos los tests de integración que comparan
`basis_hash()` entre bases distintos
(`a6_s2_uat_c05_c07_c09.rs:334-359`, `a4_4c_arch_spec_046_acceptance.rs`,
`a4_closeout_milestone_audit.rs`).

**Clippy encontró un defecto propio del refactor**: el wrapper `derive_basis_hash`
quedó sin uso tras pasar todo por `derive_basis_hash_at`. No se dejó código
muerto ni se silenció el lint: se eliminó el wrapper.

## §6 Límites declarados, y un conflicto de autoridad que encontré al escribirlos

### 6.1 El cambio contradice la spec vigente (descubierto durante el cierre)

`docs/architecture/specs/arch-spec-A3-S1-knowledge-substrate.md`
(`status: proposed`) fija en **REQ-A3S1-021**:

> `KnowledgeBasis::basis_hash` SHALL be deterministically derived from the
> sorted `(id, inner_basis_hash)` pairs (test asserts insertion-order
> independence).

Es decir, **la spec también dice que el hash deriva sólo del conjunto de
assertions**. Mi cambio hace que `revised_at` participe, lo que **contradice
REQ-A3S1-021**.

Esto cambia el relato del defecto y hay que decirlo con precisión:

- **El doc de `revise` estaba mal** (afirmaba una participación del tiempo que
  el código no tenía). Eso es un defecto real, corregido.
- **Pero el código era *consistente con la spec*.** No era un descuadre
  docs/código sobre este punto: los tres (doc de `revise`, implementación,
  REQ-A3S1-021) describían la derivación por assertions, salvo el doc de
  `revise`, que mentía.

**Por qué el cambio se entrega de todos modos, y por qué queda abierto:**

- El **doc de `revise` es la especificación operativa** de esa función: es lo
  que un lector consulta para saber qué garantiza. Un contrato que afirma una
  propiedad falsa es peor que no tenerlo, y era el único lugar donde se leía
  qué significaba `revise`.
- El defecto **de comportamiento** que sobrevive a la spec es real e
  independiente: `KMT::evaluate` compara hashes **antes** que timestamps, así
  que con la spec vigente una revisión temporal es invisible al freshness. Eso
  es incoherente con REQ-A3S1-033, que gobierna `Fresh`/`Stale`/`Unknown`.
- **REQ-A3S1-021 no se ha modificado.** Cambiar una spec en `status: proposed`
  es un acto normativo que no corresponde a una slice que arregla un doc de
  código. Lo que corresponde es **dejar constancia del conflicto** y que la
  autoridad lo resuelva.

**Acción requerida, explícita:** alguien con autoridad normativa debe decidir
entre (a) actualizar REQ-A3S1-021 para incluir `revised_at` en la derivación, o
(b) revertir este cambio y reescribir el doc de `revise` para que describa la
semántica real. Hasta entonces el repo tiene una spec y un código que no
coinciden, y eso **no se maquilla como cerrado**.

### 6.2 Sin impacto en datos persistidos (verificado, no inferido)

El límite temido — hashes v1 guardados que ahora no coincidan — **se midió en
vez de temerse**:

- `grep basis_hash` en `crates/sddk-storage/src/` → **0 resultados**. La tabla
  SQLite no tiene columna de hash de basis.
- `grep KnowledgeBasis` en `crates/sddk-storage/` y en rutas de persistencia
  del engine → **0 resultados**. `KnowledgeBasis` no se persiste.

Por tanto el cambio de dominio `v1 → v2` **no invalida ninguna identidad
almacenada**: el fallo cerrado contra una identidad v1 persistida sólo es
una propiedad teórica del tag, no un riesgo operativo actual. El tag
sigue siendo correcto porque hace que los dominios no colisionen si
mañana alguien lo persiste.

### 6.3 AT-UAT-019 no es PASS

Su criterio es *"revise con mismo contenido / distinto tiempo ⇒ comportamiento
coincide con el ADR de identidad"*. Verificado: **no existe tal ADR**
(`docs/adr/` y `docs/architecture/adrs/` no contienen ninguno de identidad de
`KnowledgeBasis`; lo más cercano es ADR-0147, sobre la seam del intelligence
loop). El criterio remite a una autoridad que no existe.

Se reporta **PASS PARCIAL**: el comportamiento es ahora coherente consigo mismo
y falsificable (4 tests + 2 falsificadores), pero **no** verificado contra una
especificación normativa, porque la que existe dice lo contrario (§6.1) y la que
el UAT cita no existe.

### 6.4 Alcance

No se toca C3m.0 (tres significados de KMT coexistiendo) ni C3m.1
(invalidación incremental). Esta slice arregla la identidad de un basis, no el
árbol que la consume.

## §7 Estado del release

v2.5.0 sigue **BLOQUEADO** (`x86_64-linux-musl-gcc` ausente; `rpm -q` en vivo:
"el paquete musl-gcc no está instalado"). `bash scripts/release.sh --dry-run`
se lanzó y **excedió el presupuesto de 600 s**: el dry-run ejecuta el perfil
completo del workspace (pasos 0-8). No es un fallo del gate, es coste. No se
publicó nada, ningún tag nuevo, y no se forzó build glibc.

## §8 Trazabilidad de la evidencia

Todas las cifras de §4 y §5 se midieron sobre `7bbeee3d`, que es el commit de
**código** de esta slice. El commit documental que añade este recibo es
posterior y no toca `crates/`, `scripts/` ni shell tests, así que el árbol
ejecutable es el que se midió.

No se pide que se confíe en esa frase:
`git diff 7bbeee3d HEAD --name-only -- crates/ scripts/ tests/'*.sh'` debe salir
**vacío** (el patrón es `tests/'*.sh'` y no `tests/` porque este `RECEIPT.md`
vive bajo `tests/cycle-artifacts/`). Si algún día no lo está, la evidencia de
esta slice **no** aplica a HEAD y hay que volver a correr el perfil.

La primera redacción de esta comprobación decía `-- crates/ scripts/ tests/`,
que **no** salía vacío: listaba este mismo fichero. Se detectó al ejecutar la
comprobación que el propio texto exigía, antes del push.
