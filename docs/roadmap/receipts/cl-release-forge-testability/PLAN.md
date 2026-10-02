# PLAN — cl-release-forge-testability

**Cycle:** `p-63676b11dc0ef88f/cl-release-forge-testability`
**Date:** 2026-10-03
**Alcance:** el cableado de testabilidad de la ruta forge, y nada más

---

## Lote 1 — tests RED (`test(release)`)

1. **Nuevo** test en el `mod tests` de `crates/sddk-cli/src/release_cmd.rs`,
   reutilizando el patrón de los tests que ya viven ahí (mismo módulo, `super::`,
   `MockForge`), **sin ensanchar ninguna visibilidad**.

   - **R1** la rama delega: la función `apply_release_forge` recibe
     `&mut dyn Forge` y la CLI la invoca con `GitHubForge`.
   - **R2** comportamiento: con `MockForge`, la función llega a `apply_release` y
     devuelve un `ReleaseOutcome` **con los pasos aplicados**, no un `Err`.
   - **R3** **estructural**: un test lee el fuente y exige que la rama
     `ReleaseRoute::Forge` **no** construya `GitHubForge::new` en línea, sino que
     delegue. Sin este guard, re-inlinear deja la rama inalcanzable **con todos
     los tests en verde**.
   - **R4** **estructural de no-regresión**: mismas capacidades
     (`pr.create`, `pr.merge`, `release.create`), mismo orden
     `CreatePr → MergePr → CreateRelease`, y el `AdmissionTicket` sigue envolviendo
     la cadena completa.
   - **R5** el doc de `release_cmd.rs` ya **no** afirma que la ruta forge «no tiene
     test» ni que «no es alcanzable sin red», o afirma algo cierto si se
     contradice.
2. Ejecutar **solo** ese test: **R1, R3, R4 y R5 caen** porque la función no
   existe todavía. Árbol rojo a propósito y declarado.

## Lote 2 — implementación (`fix(release)`)

3. `crates/sddk-cli/src/release_cmd.rs`:
   - Extraer el cuerpo de la rama `ReleaseRoute::Forge` a
     `fn apply_release_forge(gateway, forge: &mut dyn Forge, project_id, args, root, timestamp, actor) -> anyhow::Result<ReleaseOutcome>`.
   - La rama queda en tres líneas: resolver `--repo`, construir
     `GitHubForge::new(repo)`, delegar.
   - **Sin** tocar `GitHubForge`, `apply_release`, `plan_release` ni
     `authorize_release`.
   - Corregir los dos comentarios de `release_cmd.rs` que afirman que la rama no
     tiene test.
4. `cargo test -p sddk-cli` → verde, **sin reescribir ningún verde**.

## Lote 3 — falsificación (`test(release)`)

5. **Instrumento nuevo** que este plan escribe en
   `/var/home/rubentxu/f63/15-falsify-forge.py`, que **muta el producto** y exige
   que caigan los guards:

   - **M1** la rama vuelve a construir `GitHubForge::new` en línea → **R3** cae.
   - **M2** se invierte el orden de los pasos → **R4** cae.
   - **M3** se quita una autorización de capacidad → **R4** cae.
   - **M4** se quita el `AdmissionTicket` → **R4** cae.
   - **M5** la función devuelve el outcome sin llamar a `apply_release` → **R2**
     cae.

   Restaura **por bytes**. El falsificador del ciclo de `ledger watch` se llevó por
   delante trabajo sin commitear por usar `git checkout`, que repone el último
   commit; ese defecto ya está corregido en la serie y no se repite.

## Lote 4 — cierre

6. `cargo test --workspace --no-fail-fast` — **sin reescribir ningún verde**.
7. `cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings`
8. `bash tests/test_changelog_coverage.sh` · `test_debt_index_coherence`
9. Recibo con SHA, comandos, resultados y UAT; `CURRENT.md`, `STATE.yaml` y
   `SESSION-JOURNAL.md` **en la misma concernia**.
10. `git push origin main` **sin `--no-verify`**.

## Lo que este plan NO hace

- **No** ejecuta la ruta forge contra un GitHub real: tres escrituras
  privilegiadas sobre un repositorio ajeno. Sigue siendo `NOT_RUN` declarado.
- **No** toca la ruta local, ni `authorize_release`, ni el gateway.
- **No** bumpea la versión: workspace **2.5.3** sobre tag publicado `v2.5.2` → la
  siguiente release **es 2.5.3**.
- **No** desbloquea la release 2.5.3, que sigue bloqueada por la clave KMS.

## Stop

Un STOP del SCOPE-CONTRACT que se dispare **termina el ciclo** con el motivo
escrito. STOP 1 en particular: si hacer la rama alcanzable exige debilitar una
comprobación de capacidad, reordenar los pasos o mover el `AdmissionTicket`, el
arreglo se **descarta aunque los tests pasen**, porque la testabilidad no compra
permiso para relajar un control.
