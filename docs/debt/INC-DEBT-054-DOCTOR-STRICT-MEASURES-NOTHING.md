---
id: INC-DEBT-054-DOCTOR-STRICT-MEASURES-NOTHING
title: "`sddk dev doctor --strict` pasaba sin medir nada, y ningún gate lo ejecutaba"
severity: high
priority: P1
status: resolved
opened: session-65d
resolved: session-65d
resolution: Ruta 1 (dejar de fabricar el veredicto, no bajar el gate)
component: sddk-cli
surface: crates/sddk-cli/src/dev/doctor.rs, crates/sddk-cli/tests/cli.rs
---

# INC-DEBT-054: `doctor --strict` pasaba sin medir nada

> **RESUELTA (session-65d).** Los checks de `surface.briefness` se anclaban a
> `std::env::current_dir()` y cada enumeración iba dentro de un
> `if let Ok(entries) = read_dir(..)`. Un cwd sin `agents/`, `skills/` ni
> `prompts/sddk/` se saltaba todas en silencio y `--strict` salía con **exit 0
> sin haber medido nada**. Ahora se mide el árbol que contiene las superficies y,
> si no hay ninguno, se falla cerrado nombrando que el presupuesto es
> *inverificable* en ese sitio, no *satisfecho*.

## Criterio verificable

Medido sobre el binario publicado `v2.5.2` (`818d4ff9`), **antes** del cambio:

```
$ cd /tmp/brevity-probe          # directorio sin superficies
$ sddk dev doctor --strict --format json | jq '.all_present'
true
$ echo $?
0
$ sddk dev doctor --strict --format json | jq '[.checks[] | select(.tool|startswith("surface.briefness"))] | length'
0
```

Cero checks de briefness emitidos y, aun así, `all_present: true` y exit 0. El
gate no podía fallar: no tenía nada que mirar.

En el repo, el mismo comando sí encuentra 19 superficies fuera de presupuesto
(2 agents >300, 14 skills >150, 3 prompts >200). La diferencia entre «19
incumplimientos» y «0印发» no era el estado de las superficies: era dónde se
miraba.

## Causa raíz

Dos cosas distintas, y ambas hacen falta para que el gate sea inservible.

**1. Raíz equivocada.** Los checks de layout usan `framework_root`, resuelto con
`resolve_active_framework_root(environment)`. Los de briefness usaban
`std::env::current_dir()`. Dos autoridades para «qué superficies existen» en el
mismo fichero.

`resolve_active_framework_root` **ignora el cwd**: resuelve
`$SDDK_FRAMEWORK_DIR` → `$SDDK_DATA_DIR/framework` → `$XDG_DATA_HOME/sddk/framework`
→ `$HOME/.local/share/sddk/framework` y sigue el symlink `current`. Es decir, el
caso normal de auditar un prefijo instalado (`sddk dev doctor --prefix ~/.local`
desde el directorio de tu proyecto) es exactamente el cwd sin superficies.

**2. Error tragado.** `if let Ok(entries) = std::fs::read_dir(root.join("agents"))`
no distingue «no hay `agents/`» de «no se pudo leer». `read_dir` falla con
`NotFound`, el `if let` no entra, y no se emite ningún check. Lo mismo para
`skills/`, `prompts/sddk/`, el bucle de `surface.empty_dirs` y el
`responsibilities.yaml` del arch-lint.

El resultado es que *«no había superficies»* era indistinguible de *«todas las
superficies estaban dentro de presupuesto»*.

## El defecto estaba además fijado por tests

Los dos únicos sitios del repo que invocan `--strict` son
`crates/sddk-cli/tests/cli.rs:10051` y `:10188`, y ambos usan `run_doctor_from`
sobre una raíz que **sí** monta superficies (`agents/`, `skills/`, `prompts/sddk/`).
Ninguno podía alcanzar la ruta vacua. Un defecto con test no es un defecto sin
cubrir: es un defecto ratificado, y aquí el test ratificaba la ruta-afirmativa
mientras la ruta-neutra pasaba sin comprobar nada.

## Ningún gate lo ejecutaba

`grep -rn -- '--strict' .github/` → **cero coincidencias**. Ningún workflow de
CI, y ningún paso de `scripts/release.sh`, invocan `dev doctor --strict`. Los
criterios de ADR-016 (agent ≤300, skill ≤150, prompt ≤200) sólo se ejecutaban
desde esos dos tests de integración.

Los criterios siguen vigentes —no hay waiver registrado y el ADR-016 no ha sido
sustituido— pero su única aplicación automática era un test que nunca tocaba el
camino defectuoso.

## El arreglo

- **Una raíz.** `has_surface_dirs()` decide si un root puede medirse. Se prefiere
  el cwd cuando contiene las superficies —un desarrollador que acaba de editar un
  agent quiere que se mida *ese* agent, no la copia instalada— y se cae al
  framework root activo, que lleva las mismas carpetas en layout plano de bundle
  (`agents/` 70, `skills/` 96, `prompts/sddk/` 25 en `2.5.2`).
- **Fallo cerrado.** Sin superficies se emite `surface.briefness.root` con
  `present: false` y un `detail` que dice *unverifiable*, y cuenta como violación
  de brevedad. En modo advisor sigue siendo advisory (exit 0, ADR-016 §4); con
  `--strict` sale 1. Una medición vacía no es una medición aprobada.
- **Una sola autoridad.** `SURFACE_DIRS` sustituye a la lista literal
  `["agents", "skills", "prompts/sddk"]` que el bucle de `empty_dirs` repetía.

## Dientes

1. Se vuelve a anclar a `current_dir()` y se tira el fallback al framework root →
   `cli_dev_doctor_brevity_mide_el_bundle_instalado_cuando_el_cwd_no_es_un_arbol`
   falla.
2. Se devuelve el `if let Ok(read_dir(..))` sin más, con el cwd sin superficies →
   `cli_dev_doctor_strict_no_pasa_sobre_una_ausencia_de_medicion` falla.

## Consecuencia aceptada

`--strict` ahora **puede** fallar donde antes pasaba, y va a fallar: el bundle
publicado incumple 19 presupuestos. Ese es el aviso que el gate silencioso
suprimía. La salida correcta no es relajar el umbral ni seguir contando 19 como
si fueran ruido, sino adelgazar esas 19 superficies; cabe en su propio ciclo y
es trabajo de contenido, no de este arreglo.

Mismo género que [INC-DEBT-053](./INC-DEBT-053-CHAIN-AND-DEBT-GATES-VERIFY-NOTHING.md),
que documenta la misma falla —un gate que responde sin examinar nada— en la
superficie del ledger. Los dos se encontraron por separado y a la vez.
