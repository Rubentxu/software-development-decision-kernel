# RECEIPT — el camino remote de la identidad no tenía golden pin (INC-DEBT-050, remedio de fondo)

**Slice:** `session64-remote-identity-golden-pin`
**Fecha:** 2026-10-01 · **Baseline:** `352077eb` (HEAD == origin/main al abrir)
**Workflow:** `A-lite` · **Deuda:** INC-DEBT-050 (remedio 3 de 4) · **Semver:** PATCH (sólo tests)

---

## §1 Qué se cierra y qué no

INC-DEBT-050 documentó que `project_id = hash(remote normalizado, scope)` y que
**cambiar el normalizador reasigna el id de todo proyecto ya adoptado sin
migración**. Su remedio 3 era el de fondo: que ningún cambio en esa derivación
pudiera pasar inadvertido.

**Este slice hace exactamente eso y nada más.** No migra receipts, no toca
código de producción y no cambia comportamiento observable. Lo que cambia es
que la próxima reasignación **falla un test**.

## §2 Por qué el property test no servía

`crates/sddk-domain/tests/properties.rs:20` verifica:

```rust
fn stable_project_id_is_deterministic(remote, scope) {
    let first  = stable_project_id(&remote, &scope);
    let second = stable_project_id(&remote, &scope);
    prop_assert_eq!(first, second);
}
```

Eso es `f(x) == f(x)`. **Sigue siendo cierto si `f` se sustituye entera.**
Un cambio de normalizador, de dominio o de framing deja el determinismo intacto
y el property test verde. Por eso el defecto pasó: el único test de la
derivación afirmaba una propiedad que el defecto no tocaba.

La asimetría era la clave: `stable_fallback_seed` **sí** tenía golden pin desde
INC-DEBT-028, con el razonamiento escrito — *"cambiar el dominio reasignaría
silenciosamente el `project_id` de todo proyecto sin remote, y ningún test
estructural lo detectaría"*. Ese mismo razonamiento vale para el camino del
remote, **el que recorre todo proyecto real**, y ahí no había pin.

## §3 Los tres tests

Todos en `crates/sddk-domain/src/identity.rs`, siguiendo la convención de
`fallback_seed_is_pinned_to_known_value`.

**`project_id_is_pinned_to_known_values`** — valor absoluto de la derivación en
tres formas, para que un Arrangement no cubra sólo un caso:

```text
stable_project_id("https://github.com/rubentxu/example", ".")  = p-fd57187922005b40
stable_project_id("https://github.com/acme/widgets", "acme")   = p-09add0c4901adb20
stable_project_id("https://gitlab.com/g/sub/p", "sub")        = p-8a1e919e1cbfb921
```

**`remote_normalization_is_pinned_to_known_values`** — la salida exacta del
normalizador, que es la capa que *causó* el defecto: casse mixta, `.git`,
puerto por defecto que debe desaparecer, puerto no-por-defecto que debe
quedarse, forma scp/ssh, y credenciales + query + fragment.

**`case_normalization_reassigned_real_project_ids_without_migration`** — el más
importante: fija **los dos ids reales** que convivieron en esta máquina
(`p-63676b11dc0ef88f` con 65 ciclos y 3.911.680 B de ledger, frente a
`p-995939af668a53d8` vacío). Afirma el **daño**, no la intención.

El comentario del golden pin dice lo que **no** hay que hacer si falla:

> If one of these fails, DO NOT just copy the new value over. That reassigns
> the identity of every project derived from the same remote.

Copiar el valor nuevo es exactamente lo que hizo D2, en silencio.

## §4 Falsificadores OBSERVED

| # | Mutación | Resultado |
|---|---|---|
| **F53** | quitar el `.to_lowercase()` del path en `normalize_remote_path` | **OBSERVED** — `remote_normalization_is_pinned_to_known_values` FAILED: `left: "https://github.com/Acme/Widgets"` frente a `right: "https://github.com/acme/widgets"` |
| **F54** | dominio del hash `sddk.project.remote.v1` → `.v2` | **OBSERVED** — FALLAN `project_id_is_pinned_to_known_values` **y** `case_normalization_reassigned_real_project_ids_without_migration` |
| **F55** | invertir el framing (remote y scope intercambiados) | **OBSERVED** — FALLAN los mismos dos |

### Un falsificador Mal Diseñado, corregido antes de ejecutarlo

El F53 que había **diseñado** en la incidencia mutaba `to_lowercase` →
`to_ascii_lowercase`. Para entradas ASCII eso produce **el mismo resultado**, así
que el golden pin no habría fallado y la prueba no habría probado nada. Se
sustituyó por una mutación que sí cambia el valor derivado **antes** de
ejecutarla. Un falsificador que no puede fallar no es un falsificador, y la
chequeo es leerlo, no correrlo.

### Dos lecturas que sólo se obtienen ejecutándolos

1. **Las capas quedan pinadas por separado.** Con F53 (el normalizador) el
   golden del **hash** sigue verde, porque `stable_project_id` recibe el remote
   ya normalizado. Con F54/F55 (el hash) el golden del **normalizador** sigue
   verde. Cada pin protege **la suya**. Ese reparto es precisamente lo que
   hacía falta: el defecto original cruzaba las dos capas, y un único pin en
   cualquiera de ellas lo habría dejado pasar por el otro lado.
2. **El test histórico fija más de lo previsto.** Con F54 y F55 falla también,
   porque contiene los ids reales. Avisa de que un cambio en el dominio o en el
   framing no es "otro hash": es la misma reasignación silenciosa de la vez
   anterior.

## §5 Perfil de verificación

```text
cargo test -p sddk-domain --lib identity::      30 passed; 0 failed (3 nuevos)
cargo test -p sddk-domain                       559 passed; 0 failed (lib)
cargo fmt --all -- --check                      limpio
cargo clippy -p sddk-domain --all-targets -- -D warnings    exit 0
cargo test --workspace                          5245 passed; 0 failed
```

**El total cuadra con aritmética:** 5242 de session-63 + 3 tests nuevos = 5245.
Cambio sólo de tests, sin tocar producción, así que el perfil completo no es
estrictamente necesario — pero se corrió para no dar por buena una cifra
heredada, y porque el repo ya pagó una vez por confiar en el perfil parcial
(C3l.5: sólo el perfil completo caza los contratos cross-crate).

## §6 Lo que sigue abierto

- **La migración de los 25 receipts huérfanos.** Destructiva, requiere al
  operador, **no ejecutada**. El golden pin no la reemplaza: impide el
  siguiente, no arregla el anterior.
- ~~La regla "tocar el normalizador es BREAKING CHANGE"~~ — **retirada en esta
  misma sesión.** Session-63 la dejó pendiente como *"gate automático de CI"*.
  Verificado contra `.github/workflows/ci.yml` antes de invertir en ella:
  `cargo test --workspace` **ya** ejecuta el golden pin en cada perfil
  completo, y el encabezado del workflow declara *"cloud CI is an optional
  on-demand check … never a gate"* (AGENTS.md §2.5). El gate autoritativo es
  el perfil local y el pin ya está dentro de él. Habría sido redundante **y**
  contrario a la política del repo. Es la regla del operador — *alerta de
  deuda sin verificar si sus criterios siguen vigentes no es deuda real* —
  aplicada a una alerta escrita por mí mismo media hora antes.

## §7 Estado del release

v2.5.0 sigue **BLOQUEADO** por `x86_64-linux-musl-gcc`. Nada publicado, ningún
tag nuevo. Este slice es sólo tests: no cambia el binario publicado ni su
comportamiento, así que **no activa por sí solo el disparador de release**.
