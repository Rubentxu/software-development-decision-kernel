# WorkItem cycle-c — SCOPE-CONTRACT — Rediseño del contrato de admisión de release

> **Cycle:** `p-63676b11dc0ef88f/cycle-c-release-admission-redesign`
> **Tipo:** Mantenimiento del proceso de publicación — sin contenido funcional nuevo
> **Pre-condición:** el operador publica una release tras las acciones de la Fase A
> **Autoridad:** A5-1 §6 (release_admission.sh comentario), githooks/pre-push contrato A5-1
> **Status:** DESIGN — pendiente de operator-side decision sobre la política de gobernanza

## §1 Problema (síntomas observados)

Tres síntomas independientes del mismo bug estructural:

| # | Síntoma | Caso real observado | Política que violaba |
|---|---|---|---|
| S1 | `release_admission_check HEAD` rechaza tras bumpear y luego añadir docs | `74dfcc9` (H01 fix) tras `eae4f7f` (1.169.130): admisión REJECT non-monotonic 1.169.130 -> 1.169.130 | `scripts/lib/release_admission.sh:67` (`semver_gt head parent` con parent = HEAD^) |
| S2 | Pre-push hook rechaza fmt/lock/docs sin bump acompañante | `chore(fmt)` post-bump: rule A (no version change) + rule B (no docs path) → reject | `githooks/pre-push` líneas 21-46 |
| S3 | El bucle fuerza bumps ceremoniales vacíos | `4159052`, `6fda463`, `abca553`, `4e5cbed` (esta sesión) creados solo para restaurar admisión | Política system-law git.release (operador ejecuta `bash scripts/release.sh`) |

## §2 Causa raíz

Dos invariantes mal calibradas:

```text
INV-1 (release_admission.sh):
    head_version(HEAD) > parent_version(HEAD^)
    → compara con el primer padre INMEDIATO, no con la última release
    publicada.

INV-2 (githooks/pre-push):
    push admite iff (A) bump real in range OR (B) docs-only in range
    → cualquier commit que toque crates/** sin bump es rechazado, incluso
    si ya hay un bump reciente válido en el rango.
```

**Combinación fatal:** el contrato exige que entre el último bump y la publicación, NO haya commits adicionales. Esto fuerza a:

1. Acumular todo el trabajo antes de bumpear (viola AGENTS §2.1 atomicidad por concernencia).
2. O bumpear tras cada concernencia (genera bumps ceremoniales).
3. O forzar al operador a publicar antes de seguir trabajando (rompe C1/C2 continuos).

## §3 Diseño propuesto

### 3.1 Cambio de referencia para INV-1

`release_admission_check` debe comparar contra la **última versión publicada en el repo** (`git tag --list 'v*' --sort=-v:refname | head -1`), NO contra el primer padre.

Pseudocódigo del nuevo `release_admission_check`:

```bash
# last_published_version: maximum version across tags v* reachable from HEAD
# (or from any ref, for pre-push). If no tag exists, fail-closed.
last_published_version() {
    # Exclude the local working copy; use only published tags.
    # If the local branch has a tag vX.Y.Z that is not on origin, it
    # doesn't count as "published".
    local tag
    tag="$(git ls-remote --tags origin 'refs/tags/v*' \
            | awk '{print $2}' | sed 's|refs/tags/||' \
            | sort -V -r | head -1)"
    [[ -z "$tag" ]] && return 1
    echo "$tag" | sed 's/^v//'
}

release_admission_check() {
    local head_rev="${1:-HEAD}" head_version last_pub prev_version
    head_version="$(cargo_ws_version_at "$head_rev")"
    [[ -z "$head_version" ]] && { echo "REJECT missing-head-version"; return 1; }

    last_pub="$(last_published_version)"
    if [[ -z "$last_pub" ]]; then
        # No published releases yet (bootstrap). Compare against HEAD^
        # for monotonicity, fail-closed on ties or decreases.
        prev_version="$(cargo_ws_version_at "$head_rev^")"
        [[ -z "$prev_version" ]] && { echo "REJECT missing-parent-version"; return 1; }
        if ! semver_gt "$head_version" "$prev_version"; then
            echo "REJECT non-monotonic-bootstrap $prev_version -> $head_version"
            return 1
        fi
    else
        # At least one published release. Compare against it.
        if [[ "$head_version" == "$last_pub" ]]; then
            echo "REJECT already-published $head_version"
            return 1
        fi
        if ! semver_gt "$head_version" "$last_pub"; then
            echo "REJECT not-above-last-publish $last_pub -> $head_version"
            return 1
        fi
    fi
    echo "ACCEPT last-publish=${last_pub:-none} -> $head_version"
    return 0
}
```

### 3.2 Cambio de contrato para INV-2 (pre-push)

`githooks/pre-push` rule A se vuelve **range-based anchored**: admite el push si el rango contiene (a) un bump real, OR (b) solo paths dentro del allowlist extendido:

```text
Rule A (rebadged): rango admite iff contiene un commit con cambio de
  [workspace.package] version en Cargo.toml.

Rule B (extended): rango admite iff todos los commits tienen paths en
  docs/** | .sddk/followups/** | tests/cycle-artifacts/p-*/*/{SCOPE-CONTRACT,
  DISCOVERY, RECEIPT}.md | MANIFEST.sha256
  AND ningún path contiene secret patterns.
```

**Lo que cambia:** rule A pasa de "el rango en su conjunto contiene un cambio" (lo que ya hacía) a **ser explícita sobre el ancla**: el push se admite si el rango contiene un bump real, sin importar qué otros commits no-bump haya después. Esto es **lo que el operador intentó pero el hook no admitía** cuando añadí `chore(fmt)`. El fmt commit era admisible por rule A (porque su push range `[abca553..HEAD]` contenía un cambio de versión), pero el hook range-based ya lo evaluaba así. **El problema es entonces release_admission.sh, no el pre-push.**

### 3.3 Tests requeridos

| Test | Escenario | Esperado |
|---|---|---|
| `ra-bootstrap-monotonic-accept` | No tags published yet, head version > parent version | `ACCEPT last-publish=none -> X.Y.Z` |
| `ra-bootstrap-tie-reject` | No tags published yet, head version == parent version | `REJECT non-monotonic-bootstrap` |
| `ra-published-above-accept` | Last published = 1.169.122, head = 1.169.123 | `ACCEPT last-publish=1.169.122 -> 1.169.123` |
| `ra-published-equal-reject` | Last published = 1.169.122, head = 1.169.122 | `REJECT already-published 1.169.122` |
| `ra-published-lower-reject` | Last published = 1.169.122, head = 1.169.121 | `REJECT not-above-last-publish 1.169.122 -> 1.169.121` |
| `ra-published-tie-via-docs-only` | Last published = 1.169.122, head = 1.169.122, intervening docs-only commits | `ACCEPT` (because head equals last published, but rule A1 admits post-bump docs) **OR** `REJECT already-published` if rule A1 is rejected |
| `ra-publish-while-publishing` | Two concurrent release.sh runs racing | Mutual exclusion via tag lock OR deterministic fail-closed on second `gh release create` |
| `ra-head-not-on-tag-sha` | Local HEAD has a newer version than local tag | REJECT (cannot publish a version that hasn't been soldered to a tag) |

### 3.4 Casos edge / invariants

```text
INV-3  fail-closed: si git ls-remote --tags falla, NO degradar a
       comparación contra HEAD^; el operador debe ver error claro.
INV-4  sourceable: el script sigue siendo sourceable (define funciones
       sin exit).
INV-5  zero-config: no requiere flags nuevas; el comportamiento cambia
       por inspección de tags remotos.
INV-6  pre-push sincronizado: githooks/pre-push usa la misma fuente
       `last_published_version` para no admitir un push cuyo head_version
       sea menor o igual a la última publicada.
INV-7  idempotente: re-ejecutar `release_admission_check HEAD` con la
       misma última versión publicada devuelve el mismo ACCEPT/REJECT.
INV-8  lock contra carreras: si el operador publica v1.169.N y otro
       agente corre release.sh simultáneamente, el segundo falla con
       `gh release create` (HTTP 422 already_exists) — el script debe
       propagar este error sin retry.
```

## §4 Riesgos y trade-offs

| Trade-off | A favor | En contra |
|---|---|---|
| Comparar contra tag remoto (no HEAD^) | Permite commits documentales entre bump y publish | Depende de `git ls-remote` — si la red falla, el script aborta |
| Eliminar fallback a HEAD^ cuando hay tags | Más estricto, evita bypass | En CI sin red, el script no funciona (aceptable — CI debe tener red) |
| No exigir bump para docs-only commits post-bump | Habilita trabajo continuo | El operador puede olvidarse de bumpear antes del release real |
| Mantener rule A del pre-push (rango contiene bump) | Consistente con admisión | Si el rango tiene commits NO-docs NO-bump entre dos bumps, el push se admite igual (¿deseado?) |

## §5 Gates y governance

- **system-law inviolable:** la política de admisión es **política de gobernanza** y NO se modifica sin aprobación del operador.
- **ADR-0098 proposed:** redactar ADR-0098 cubriendo este rediseño, con clasificación `governance.release_admission`.
- **Tests rojos primero:** todos los casos del §3.3 se escriben como RED antes de tocar `release_admission.sh`.
- **Operator-side decision needed:** el operador debe decidir si acepta el rediseño ANTES de implementarlo. Si lo rechaza, este SCOPE-CONTRACT se archiva con `status: deferred` y se cierra el WorkItem.

## §6 Trabajo fuera de alcance

- No tocar `githooks/pre-push` salvo para sincronizar la fuente `last_published_version` (cambio mínimo, regla A intacta).
- No tocar `scripts/release.sh` más allá de propagar el cambio de mensaje `last-publish` (cosmético).
- No añadir CI nueva.
- No publicar releases intermedias (regla de la Fase D del operador).

## §7 Plan de implementación (si se aprueba)

1. Crear `tests/test_release_admission.sh` con los 8 casos del §3.3, RED.
2. Modificar `scripts/lib/release_admission.sh`: añadir `last_published_version()`, cambiar `release_admission_check`.
3. Modificar `githooks/pre-push`: usar `last_published_version()` para sincronizar el reject en el push (no rompe rule A).
4. Re-correr tests — todos verdes.
5. Documentar en `docs/architecture/adrs/ADR-0098-RELEASE-ADMISSION-REDESIGN.md` (accepted, con sign-off del operador).
6. NO publicar release con esta versión — este cambio es al contrato, no al producto.

## §8 Estado actual

- **Autorización:** pendiente.
- **Tests RED:** 0 escritos.
- **Impacto en publicación actual:** NINGUNO — `release_admission_check HEAD` con la regla vieja sigue dando ACCEPT 1.169.132 → 1.169.133 en `abca553`.
- **Próximo paso del orquestador:** NO actuar hasta operator-side approval.
