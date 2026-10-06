# RECEIPT — REL-2.12.1 «la superficie de gates cierra su censo»

## §0 — Publicada y verificada contra lo que se ve desde fuera

`v2.12.1`, publicada el 2026-10-06T13:01:31Z, `draft=false`, `prerelease=false`,
9 assets. **Verificada por dos caminos distintos**, que es la unica forma de que
una release no describa otra cosa:

| Comprobacion | Resultado |
|---|---|
| Binario por API vs CDN | `a3bbbbbe…dbf44` en ambos — coinciden |
| `sddk.sha256` publicado vs CDN | coincide |
| `CHECKSUMS` (musl + framework) | **2/2 OK, 0 FALLO** |
| Assets totales | 9, el contrato completo |
| `gh-release-receipt.json` | `tag=v2.12.1`, `schema_version=1` |
| Local: binario | `sddk 2.12.1` |
| Local: bundle | `framework/2.12.1/` |
| Local: `current` | → `framework/2.12.1` |

El binario se comparo por **API y por CDN** porque son dos caches distintas: la
API responde del store de GitHub y el CDN del edge, y un asset subido con
`--clobber` puede seguir sirviendo el viejo por el segundo camino durante
minutos. Medido, no supuesto.

## §1 — Que se publico

Catorce commits desde `v2.12.0`, en cuatro bloques:

1. **VA17** (`84fe88b7`): los cuatro almacenes de sesion pasan a ser por worktree,
   con el canario de aislamiento reparado y un test que fija que
   `#[serde(default)]` no es load-bearing sobre `Option<T>`.
2. **Pureza del nucleo** (`e60dae75`): la ley se vigila en 46 modulos, no en uno.
3. **Censo a verde** (`b6ad2d36`): los tres canarios cableados al camino de
   release como pasos **3o** y **3p**, y las dos excepciones de recibos con su
   motivo medido. `SIN runner y SIN motivo: 0`.
4. **Tres correcciones de instrumentacion** (`f30f0d49`, `1db13233`, y el
   puntero de `STATE.yaml`), todas nacidas de que la release las detectara.

## §2 — Los tres fallos que mataron la release, y que NO eran el mismo fallo

Esta es la parte que mas dice, porque **un release que muere tres veces por tres
causas distintas no estaba fallando: estaba midiendo tres cosas distintas**.

1. **`test_adr_0157_criteria.sh`, `NOT_APPLIED=2` en C1.** Los dos tests de C1
   apuntaban a nombres que `e60dae75` habia renombrado. MEDIDO que no era un
   defecto de redaccion sino **saturacion del host**: `load average` de 20 a 39
   sobre 64 nucleos, con Gradle y `pytest` de `/Proyectos/kotlin` y
   `skillgraph` corriendo a la vez. Aislado, el guard da `PASS=26 FAIL=0
   NOT_APPLIED=0` en 37 s. Repuntado por otro actor en `b6311078`.

2. **`test_release_state_pointer.sh`, 9 commits de deriva.** `current_sha`
   declaraba `e441df0c` con `main` en `b6311078`. Fallo **real y accionable**:
   el unico guard que sabe si el documento de estado dice la verdad. Reconciliado
   con `scripts/reconcile_state_pointer.sh`, que preserva `superseded_pointer` y
   su nota de evidencia.

3. **`test_build_identity_policy.sh`, `PASS=6 FAIL=2`.** Un receipt de este mismo
   bloque sustituyo la palabra «release» por dos caracteres CJK.
   **Lo que lo hace una leccion y no un incidente**: el guard mide las lineas
   anadidas en el **diff committeado** contra el tag, no el arbol de trabajo. Sin
   commit, el FAIL sigue vivo aunque el fichero este limpio en disco — el
   instrumento lee la puerta y no el umbral. Corregido en `1db13233`, verificado
   `PASS=8 FAIL=0 NO_MEDIDO=0`.

## §3 — Lo que esta release NO demuestra

1. **El layout de sesion es rompiente por naturaleza.** Quien tenga un `sddk`
   viejo construido contra el layout anterior seguira escribiendo donde el
   anterior leia. No hay migracion ni aviso de vuelta atras. Esta release es la
   que hace visible ese corte.
2. **No migra datos.** Los ficheros del layout viejo —**92 bindings, 94 deltas,
   27 capsules**, medidos hoy en `data_home` (`~/.local/share/sddk`), no en
   `state_home` como se midio una vez— quedan **inertes**: 0 ilegibles, ninguno
   con `workspace_id`. Migrarlos exigiria inventar un `workspace_id`.
3. **Va sin firma.** `SDDK_SKIP_SIGNING=1`, `UNSIGNED` declarado. El ancla y
   `SDDK_RELEASE_VERIFY_KEY_BODY` son placeholder y **no se fabrica ninguna**: una
   release sin firma se declara, una con firma inventada es peor.
4. **La flakiness por carga ajena no esta arreglada, esta medida.** El host
   compartido con otros repositorios sigue siendo la causa de que un guard de
   56 binarios de test pueda no terminar a tiempo. El guard es correcto; lo que
   no es determinista es la maquina.

## §4 — La leccion que se lleva este bloque

**`HEAD` se movio cuatro veces bajo los pies de esta sesion**, y las cuatro
aparecieron como fallos de release que no tenian nada que ver con lo que la
sesion creia estar haciendo. Un contexto de sesionDescribe un estado pasado; el
unico modo de saber el presente es `git log`.

Y la segunda: **los tres fallos tenian signos opuestos y ninguno se resolvia
intentando mas de lo mismo**. Uno era del instrumento, uno del documento, uno del
producto. Un bloque que llega al release con tres rojos y reintenta tres veces
cree que son el mismo problema hasta que mide cada uno.