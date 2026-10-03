# PRE-FLIGHT — cl-doctor-build-identity

**Cycle:** `p-63676b11dc0ef88f/cl-doctor-build-identity` (por abrir)
**WorkItem:** cerrar el hueco que deja **INC-DEBT-064** (high/P1, `open`) — el
detector existe y funciona, pero **nadie lo consulta**.
**Date:** 2026-10-03

---

## 1. Premisas, y cómo se comprobó cada una

Ninguna se da por buena. Las que se comprobaron ejecutando llevan el comando.

| # | Premisa | Comprobación | Resultado |
|---|---|---|---|
| Q1 | `dev doctor` **no sabe nada** de la identidad del binario | `grep -n 'build_id\|BuildIdentity\|SDDK_BUILD_SHA\|SDDK_GIT_SHA\|identity' crates/sddk-cli/src/dev/doctor.rs` | **cero coincidencias** |
| Q2 | `binary.bundle_coherence` solo mira recibo, bundle y manifiesto | lectura de `doctor.rs:420-468` | no compara ningún commit |
| Q3 | Por tanto, un binario obsoleto **pasa** el doctor en verde | Q1 + Q2 | confirmado por lectura, no supuesto |
| Q4 | Es la condición de escalada que el propio INC-DEBT-064 declara | `docs/debt/INC-DEBT-064-*.md`, sección de severidad | «si `dev doctor` declara coherencia donde no la hay» |
| Q5 | El doctor corre en el sistema **instalado**, donde no hay checkout | `release.sh` paso 11: `dev doctor --prefix $SDDK_PREFIX` | el cwd es el repo en release, pero el uso normal es fuera |
| Q6 | `resolve_root` encontraría el checkout de **cualquier** repo | lectura de `crate::canonical_root` / `resolve_root` en `build_id.rs` | sube desde el cwd buscando un marcador de proyecto |
| Q7 | Por tanto, comparar contra un repo ajeno daría `Diverged` en falso | Q5 + Q6 + la semántica de `compare` (`build_id.rs:225-283`) | un commit de sddk no está en la historia de otro repo |
| Q8 | Existe un precedente para «no aplicable» | `doctor.rs:435-450`, rama `flat_install` | reporta en **verde** con detalle, sin acusar lo que nunca se prometió |
| Q9 | Los checks no-briefness son **fatales** (`all_present` → exit 1) | `doctor.rs:809-830` | `framework_warnings == 0` y, si no, `status = 1` |

**Q7 es la premisa que gobierna el diseño.** Sin ella, cablear el detector al
doctor produciría falsos positivos en cualquier directorio queResultara ser un
repo de git, que es casi todo. La comprobación tiene que distinguir «checkout de
*sddk-framework*» de «checkout de algo».

## 2. Guards

Cada guard dice qué fija y **cómo se rompe**. Un guard sin modo de fallo escrito
no es un guard: es una aserción.

| Guard | Qué fija | Cómo se rompe |
|---|---|---|
| R1 | El doctor **sí** emite un check de identidad de build | se renombra el `tool:` del check |
| R2 | Un checkout de sddk-framework con el binario al día → **verde** | `is_current` devuelve falso para `Matches` |
| R3 | Un checkout de sddk-framework con el binario **atrás** → **rojo**, y es la detección de INC-DEBT-064 | el check se informa `present: true` siempre |
| R4 | **Sin** checkout de sddk-framework → **verde con detalle N/A**, nunca rojo | se quita la rama N/A y cae en el `else` rojo |
| R5 | Un repo de git **ajeno** → **verde con detalle**, no `Diverged` en falso | se quita el marcador de proyecto y cualquier repo cuenta |
| R6 | Identidad **no concluyente** (`source: git`) → **verde con detalle**, porque STOP 6 prohíbe que decida | se compara sin mirar `is_conclusive()` |
| R7 | El texto nombra la **procedencia** del veredicto, no solo `present` | el `detail` pasa a `None` |

**R4, R5 y R6 son la parte que importa.** R2 y R3 son el caso feliz y el
triste; se fijan casi solos. Las tres que hay que falsificar con cuidado son las
que evitan que el detector se convierta en una máquina de fallar: un check rojo
en un sistema instalado sin checkout, o en el repo equivocado, es peor que no
tener check, porque entrena al operador a ignorar los rojos.

## 2bis. Mapa objetivo → guard

Se declara de forma explícita para que la cobertura sea un artefacto
verificable y no una lectura de buena voluntad del redactor.

| Objetivo | Guards que lo mandan |
|---|---|
| **O1** el check existe y se nombra | **R1** |
| **O2** al día → verde | **R2** |
| **O3** atrasado → rojo, y lo nombra | **R3** |
| **O4** sin checkout → verde con N/A | **R4** |
| **O5** repo ajeno → verde con N/A | **R5** |
| **O6** identidad no concluyente → verde con detalle | **R6** |
| **O7** el texto nombra la procedencia | **R7** |

**O1** → R1 · **O2** → R2 · **O3** → R3 · **O4** → R4 · **O5** → R5 ·
**O6** → R6 · **O7** → R7

## 3. Riesgos declarados antes de empezar

- **R-a. El doctor pasa de informativo a verificador de identidad.** Un
  `all_present == false` nuevo hace que `dev doctor` salga con 1 donde antes
  salía con 0. Es el objetivo, pero es un **cambio de contrato observable** y
  se declara como tal en el changelog, no como un detalle.
- **R-b. Identidad no concluyente en los binarios ya instalados.** Los binarios
  publicados antes del cambio de `release.sh` declaran `source: git`, luego el
  check será N/A para ellos y solo tendrá dientes a partir de la próxima
  release. Se declara; no es un fallo del check.
- **R-c. Un checkout en un commit futuro declara al binario `Behind`.** Es
  correcto — el checkout tiene trabajo que el binario no contiene — y es
  exactamente lo que hay que avisar.
- **R-d. `resolve_root` con `--no-infer` y sin `--root` es un caso que este
  check no resuelve.** Se declara, no se adivina.

## 4. Fuera de alcance

- **No se cambia `--version`**, ni `install.sh`, ni el contrato del bundle.
- **No se toca `BUNDLE.toml`** ni la superficie del manifest.
- **No se publica nada.** El cambio no toca el release pipeline más allá de lo
  ya hecho en `c30dcf89`.
- **No se forcea `dev doctor` para que pase.** Si el check nuevo falla, falla.

## 5. Criterio de «hecho»

1. `dev doctor` emite el check y distingue los cuatro casos (al día, atrasado,
   sin checkout de sddk, repo ajeno).
2. Los cuatro tienen guard, y cada guard se ha **visto caer** con una mutación
   aplicada al producto.
3. La suite completa sigue en verde, y el changelog declara el cambio de
   contrato de `dev doctor`.

---

**Readiness: READY**
