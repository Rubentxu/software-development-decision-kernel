# DESIGN — cl-release-forge-testability

**Cycle:** `p-63676b11dc0ef88f/cl-release-forge-testability`
**Date:** 2026-10-03

---

## El defecto, en una frase

El adaptador `GitHubForge` **ya sabe** recibir un runner inyectado y el motor
**ya acepta** cualquier `dyn Forge`; lo único que impide probar la rama es que
el call site de la CLI construye el adaptador con el runner real y no deja
sustituirlo. **No es una limitación de la red: es un cableado que falta.**

## Decisión

Extraer el cuerpo de la rama `ReleaseRoute::Forge` a una función que reciba el
forge como parámetro:

```rust
fn apply_release_forge(
    gateway: &mut CapabilityGateway,
    forge: &mut dyn Forge,
    project_id: &str,
    args: &DistArgs,
    root: &Path,
    timestamp: ...,
    actor: ...,
) -> anyhow::Result<sddk_gateway::ReleaseOutcome>
```

Y la rama queda en tres líneas: resolver `--repo`, construir
`GitHubForge::new(repo)`, delegar.

La CLI la llama con el runner real; el test la llama con `MockForge`. Es
inyección de dependencia por parámetro, sin estado global, sin `#[cfg(test)]` sobre
el estado del proceso, y **sin ensanchar visibilidad**: la función es privada del
módulo y los tests viven en el `mod tests` del propio módulo.

## Alternativas descartadas, y por qué

### 1. Ejecutarla contra un GitHub real — DESCARTADA

Serían `pr.create`, `pr.merge` y `create.release`: tres escrituras privilegiadas
sobre un repositorio ajeno. AGENTS.md §1 lo prohíbe, y con razón: un producto que
publica en el repositorio de otro durante una prueba **es** el producto haciendo
algo irreversible. No es un coste que se pueda negociar por valor.

### 2. Un override global `#[cfg(test)]` del runner — DESCARTADA

Permitiría no mover una sola línea y añadir un test. Se descarta porque
introduce **estado mutable global**: dos tests que se pisen, orden dependiente, y
un fallo que solo aparece en la suite completa. Un arreglo que hace la rama
alcanzable a costa de la deterministicidad de la suite cambia un defecto por otro,
que es exactamente lo que pasó con la constante `OMITTED_NODE_FIELDS` en el ciclo
de `cl-vault-node-projection`.

### 3. Ensanchar `pub` para testear desde `tests/` — DESCARTADA

Permitiría un test de integración, a costa de ampliar la superficie pública del
crate por un motivo que no es de uso. STOP 3 lo prohíbe. Los tests de este módulo
ya viven dentro del crate y llegan con `super::`, luego no hace falta.

### 4. Un segundo camino de prueba que reimplemente la rama — DESCARTADA

Daría verde sin ejecutar nada del producto. Es la forma de «un test que no mide
lo que dice medir», y es lo que R2 prohíbe explícitamente: el test invoca **la
función extraída**, no una copia suya.

## El invariante que la extracción debe preservar

El orden de la cadena y su envoltura no cambian **por nada**:

```
version_authority_or_fail  →  plan_release  →  AdmissionTicket {
                                                       create_pr
                                                       merge_pr
                                                       create_release
                                                    }  →  mapeo de errores
```

Y `authorize_release` sigue exigiendo `["pr.create", "pr.merge", "release.create"]`
para la ruta forge. Si la extracción relajara cualquiera de estas cosas, STOP 1
descarta el arreglo **aunque los tests passen**: la testabilidad no compra
permiso para relajar un control.

## Por qué esto y no «añadir un test que no llega»

Un test que no alcanza la rama es decorativo: pasa, y el defecto sigue. La
razón de que hoy no haya test **no es** que no se pueda escribir —es que la
escritura natural llevaría a las alternativas 2, 3 o 4—, y las tres están
descartadas por lo que cuesta. La función extraída es la quinta forma, y la
única que no compra nada.
