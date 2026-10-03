# SCOPE-CONTRACT — cl-doctor-build-identity

**Cycle:** `p-63676b11dc0ef88f/cl-doctor-build-identity`, `B-direct`
**Date:** 2026-10-03
**WorkItem:** el hueco que deja **INC-DEBT-064** (`open`, high/P1).

---

## 1. El problema, medido

El detector existe y funciona: `sddk dev build-id --check` distingue un binario
al día de uno atrasado, con códigos de salida distintos y comprobados. **Pero
nadie lo consulta.** Medido: `doctor.rs` no menciona la identidad ni una vez
(`grep` de `build_id|BuildIdentity|SDDK_BUILD_SHA|SDDK_GIT_SHA|identity` → cero
coincidencias), y `binary.bundle_coherence` solo valida el recibo de
instalación, el directorio versionado y la compatibilidad del manifiesto.

Eso cumple exactamente la condición de escalada que el propio documento de
deuda declara: *«si `dev doctor` declara coherencia donde no la hay»*. Un binario
de 1,24 días puede hoy pasar `dev doctor` en verde.

**Por qué esto no es «un detalle más»:** la nota (a) de INC-DEBT-061 ya resolvió
una vez que «el arreglo no está roto, no está desplegado», y esa lección se
aplicó como **dato de un caso**. Este documento es la repetición, y lo que
faltaba no era el diagnóstico —ya estaba— sino el **mecanismo**: el lugar del
producto donde la condición se mira sola.

## 2. Decisión

Un check nuevo en `dev doctor`, `binary.build_identity`, que comparará la
identidad embebida en el binario en ejecución contra el checkout de
`sddk-framework` que se pueda resolver desde el directorio actual.

**Y cuatro estados, porque cuatro es lo que hay, no tres:**

| Estado | Condición | Verde/rojo | Por qué |
|---|---|---|---|
| al día | `Matches` | **verde** | el caso feliz |
| atrás o divergido | `Behind` / `Diverged` | **ROJO** | **esto es la detección** |
| sin checkout de sddk | no hay repo de *este* proyecto | **verde + N/A** | el sistema instalado no tiene checkout, y acusar eso sería mentir |
| repo ajeno | hay repo de git que no es sddk-framework | **verde + N/A** | comparar el commit de sddk contra la historia de otro repo daría `Diverged` en falso |

**El cuarto estado es el que gobierna el diseño**, y no estaba en el encargo
inicial. `resolve_root` sube desde el cwd buscando un marcador de proyecto, así
que sin esa guarda el detector se volvería una máquina de fallar: casi todo
directorio está dentro de algún repo de git, y el resultado sería rojo en
casi todas partes. **Un check rojo que se equivoca entrena a ignorar los rojos**,
que es peor que no tener check.

**Y el estado «identidad no concluyente» va a verde con detalle**, por STOP 6
del SCOPE de `cl-build-identity`: la procedencia `git` existe como dato de
diagnóstico y le está prohibido decidir. Un check que decidiera con ella
reintroduciría el defecto que la biseca 2 ya midió —el detector que declara un
valor obsoleto sin señal—.

## 2bis. Objetivos falsables

Cada uno tiene un resultado que puede ser **falso**, que es lo que lo convierte
en requisito y no en nota. Un objetivo sin guard es una nota.

1. **O1.** `dev doctor` emite un check `binary.build_identity` y lo nombra en la
   salida de texto y en el JSON. *Falsable:* el `tool:` no aparece.
2. **O2.** Con un checkout de `sddk-framework` en el HEAD y un binario al día,
   el check sale **verde**. *Falsable:* sale rojo.
3. **O3.** Con un checkout de `sddk-framework` en un commit posterior al del
   binario, el check sale **ROJO** y nombra la relación. *Falsable:* sale verde
   o sale rojo sin nombrar nada.
4. **O4.** **Sin** checkout de `sddk-framework` —sistema instalado, tarball,
   directorio sin repo— el check sale **verde con detalle N/A**. *Falsable:* sale
   rojo, que es lo que rompería un uso legítimo.
5. **O5.** Con un repo de git **ajeno** a sddk-framework, el check sale **verde
   con detalle**, nunca `diverged` en falso. *Falsable:* compara contra la
   historia de otro repo.
6. **O6.** Con identidad **no concluyente** (`source: git`), el check sale
   **verde con detalle** y dice por qué no puede decidir. *Falsable:* decide, que
   es lo que STOP 6 prohíbe.
7. **O7.** El texto que `dev doctor` imprime nombra la **procedencia** del
   veredicto, no solo el color. *Falsable:* el `detail` va a `None`.

## 3. Qué NO hace

- **No reescribe `--version`**, ni `install.sh`, ni el contrato del bundle.
- **No toca `BUNDLE.toml`** ni la superficie del manifest (STOP 4 del ciclo
  anterior, que aquí se hereda).
- **No se conecta a la red** ni consulta nada externo: solo git local.
- **No fuerza a `dev doctor` a pasar.** Si el check nuevo falla, falla, y esa
  es la razón de existir.

## 4. STOP conditions

- **STOP 1 — Si el check no puede distinguir «checkout de sddk-framework» de
  «repo de git cualquiera», se descarta el trabajo entero.** Un detector que no
  sabe contra qué compara es peor que ninguno.
- **STOP 2 — Si el caso «sin checkout» acaba en rojo, se descarta.** El doctor
  se usa en máquinas instaladas; un rojo ahí rompe un uso legitimo.
- **STOP 3 — Si hacer el caso «repo ajeno» verde exige no mirar la
  identidad, se descarta.** No se resuelve con `present: true` a ciegas: sería
  el patrón que este mismo ciclo critica.
- **STOP 4 — Si el cambio obliga a tocar `--version` o el bundle, se corta.**

## 5. Criterio de aceptación

1. Los cuatro estados distinguidos, cada uno con guard, y **cada guard visto
   caer** con una mutación aplicada al producto.
2. La suite completa en verde.
3. El changelog declara el **cambio de contrato**: `dev doctor` puede salir con
   1 donde antes salía con 0. Eso es el objetivo, y se dice en voz alta en vez
   de descubrirlo en producción.
