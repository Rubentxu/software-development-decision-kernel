# RECEIPT — VA17 `cl-almacenes-de-sesion-por-worktree`

**Bloque:** repair de VA16 — los cuatro almacenes de sesión pasan a ser **por worktree**
**Ciclo SDDK:** ninguno. Este bloque es de roadmap; su `PRE-FLIGHT.md` está en este
mismo directorio y es la evidencia de arranque, no un ciclo del ledger.
**Cerrado:** 2026-10-06
**Workspace:** 2.12.1 (declarada, no publicada; último tag remoto `v2.12.0`)

---

## §0 — Cómo se cerró este bloque, que no es lo normal

El `apply` **lo escribió otra sesión en paralelo** y murió sin commitear: a las
10:20 dejó el código y a las 10:43 seguía tocando ficheros. La medición de este
recibo la hizo **otra sesión distinta**, sobre el árbol congelado, y no hereda
ninguna afirmación de la anterior.

Eso no es un detalle de forma. Significa que **el código de este bloque fue
escrito por alguien que nunca lo ejecutó**, y que las tres afirmaciones que
importan —que compila, que los guards sobreviven, que el canario vigila— no
tenían ninguna medición detrás hasta hoy.

## §1 — Qué se cambió

| fichero | qué |
|---|---|
| `crates/sddk-engine/src/paths.rs` | `AdoptionPaths.session_root` = `project_data/workspaces/<workspace_id>/context`. Los cuatro almacenes cuelgan del ancla que **ya era** por worktree (`adoption.json`), no de una segunda autoridad. |
| `crates/sddk-engine/src/agentic_session_binding.rs` | `workspace_id: Option<String>` con `#[serde(default)]`, enum `WorkspaceAdoption` (`Same`/`Foreign`/`Unknown`), `attach_in_workspace`, `adoption_verdict`, y los errores `WorkspaceMismatch` / `WorkspaceUnknown`. |
| `crates/sddk-cli/src/context_cmd.rs` | Las cuatro raíces cuelgan de `session_root`; camino de reattach fail-closed con `--rebind` y receipt `context-bootstrap-rebind-workspace-*`; aviso `legacy_deltas_not_visible` cuando el drain del layout nuevo no ve nada y el árbol viejo sí tiene contenido. |
| `crates/sddk-cli/src/lib.rs` | Flag `--rebind` en `context bootstrap` + render del aviso de deltas. |
| `tests/uat_ctx_007_context_expand.sh` | La aserción **localiza** el read log con `find` y **afirma la propiedad nueva** (`*/workspaces/*/context/reads/*`), en vez de llevar el path viejo dentro. |
| `tests/test_worktree_isolation_canary.sh` | `EXPECT_LEAK="si"` → `"no"`: de canario que media un defecto a guard estricto. Nueva rama `empeora`, y M1/M2/M3 sustituyen a las mutaciones de VA16. |
| `crates/sddk-engine/tests/va17_legacy_binding_serde.rs` | **Nuevo.** El contrafactual del `#[serde(default)]`. |

## §2 — Medición, hecha hoy sobre el árbol congelado

```
tests/test_worktree_isolation_canary.sh          PASS=4  FAIL=0 SKIP=0   (base: 0 violaciones)
  base          el aislamiento por worktree se sostiene, 0 violaciones
  M1            las cuatro raíces vuelven a colgar de project_data   → empeora
  M2            workspace_id deja de tolerar bindings antiguos       → 0 → 1 violaciones
  M3            comentario inocuo (control negativo)                 → no cambia

cargo test -p sddk-engine --test va17_legacy_binding_serde
  2 passed; 0 failed
    serde_deriva_option_ausente_a_none_sin_atributo
    un_campo_no_option_si_exige_el_atributo      ← el contrafactual

tests/uat_ctx_001..007                            0 / 7 rojos
```

**Lo que M2 midió, y es el corazón del bloque:** al quitar el
`#[serde(default)]`, un binding sin `workspace_id` pasa a **adoptarse en
silencio** (`rc=4`, el mismo código que un ciclo ausente). Es exactamente el
leak que VA16 encontró, y el `None` del campo está codificado como *«worktree
desconocido»*, no como comodín. La tercera clase de la tripartición
(proyecto / worktree / **desconocido**) es lo que hace que el fail-closed tenga
dientes sobre los 91 bindings reales que ya estaban en disco sin `workspace_id`.

## §3 — Lo que este bloque NO demuestra

1. **No migra datos.** Hay ~212 ficheros en el layout viejo y ninguno lleva
   `workspace_id`. Migrarlos exigiría **inventar** un `workspace_id`, así que
   quedan inertes y recuperables, y el layout nuevo empieza limpio. Declarado, no
   disimulado.
2. **`PRE-FLIGHT.md` y `SCOPE-CONTRACT.md` no se tocaron**, y tampoco hace falta:
   declaran intención. El guard de recibos que nació en el bloque contiguo
   (`test_receipt_cycle_id_authority.sh`) tiene su alcance escrito por esa razón.
3. **El layout es rompiente por naturaleza.** Quien tenga un `sddk` viejo
   construido contra el layout anterior seguirá escribiendo donde el anterior
   leía. No hay migración ni aviso de vuelta atrás.
4. **`ledger.sqlite` sigue siendo por proyecto**, y `knowledge-profile`,
   `artifacts`, `cycle_artifacts` y `generated` también: eso es decisión, no
   olvido, y está en la cabecera del struct `AdoptionPaths`.

## §4 — Dos cosas que pasaron mientras este bloque se cerraba, y que quedan escritas porque son evidencia

**Un estado intermedio de `paths.rs` era un defecto, y alguien lo arregló antes
que esta medición.** A las 10:36:51 la línea era:

```rust
let project_data = data_home.join("sddk/projects").join(project_id).join(workspace_id);
```

Eso hace `project_data` **por worktree**, y arrastra en cascada los cuatro
almacenes que el propio struct declara por proyecto, y deja `receipt` y
`session_root` con el segmento de workspace **dos veces**
(`projects/<p>/<w>/workspaces/<w>/…`). Contradecía la cabecera del mismo struct y
el no-objetivo #2 del `PRE-FLIGHT`. A las 10:37:49 volvió a `join(project_id)`, y
el canario de hoy mide el layout bueno: `knowledge-profile.json` sigue en
`projects/<p>/` y el binding cuelga de `projects/<p>/workspaces/<w>/context/`.

**El canario hubo que cambiar de signo, no solo de constante.** `EXPECT_LEAK` se
invierte de `"si"` a `"no"` porque la base pasa a estar **verde sin leak**. Con
ese constante, la rama `cambia` (que exigía que las violaciones **bajasen** al
mutar) seguía teniendo la semántica de VA16, donde mutar era construir la
defensa. Con la propiedad ya existente, mutar **deshace** el aislamiento y tiene
que **subir** las violaciones: de ahí la rama `empeora`. Dejar `cambia` habría
sido un guard que solo sabe confirmar arreglos — y su comentario lo dice: *«eso
no es un guard: es un cheerleader»*.

## §5 — Commits

Este bloque se committea junto a la verificación, sin mezclarse con el bloque
contiguo de recibos: son dos concernencias y dos commits.

Gates de este bloque: canario `PASS=4 FAIL=0 SKIP=0` · test nuevo `2 passed` ·
`uat_ctx_001..007` **0/7 rojos** · `shellcheck` limpio en los guards tocados.
