# RECEIPT — cl-doctor-build-identity

**Cycle:** `p-63676b11dc0ef88f/cl-doctor-build-identity`, `B-direct`, `OPEN/build`
**WorkItem:** el hueco que deja **INC-DEBT-064** (`open`, high/P1).
**Date:** 2026-10-03

---

## 1. Qué se cierra

El detector de identidad ya existía y funcionaba: `dev build-id --check`
distinguía un binario al día de uno atrasado, con códigos de salida distintos y
comprobados. **No había ningún sitio del producto donde se mirara.**

Medido, no supuesto:

```text
$ grep -n 'build_id|BuildIdentity|SDDK_BUILD_SHA|SDDK_GIT_SHA|identity' \
      crates/sddk-cli/src/dev/doctor.rs
(sin salida)
```

Y `binary.bundle_coherence` (`doctor.rs:420-468`) valida el recibo de
instalación, el directorio versionado y la compatibilidad del manifiesto —
**ningún commit**. Eso cumple exactamente la condición de escalada que el
propio documento de deuda declara: *«si `dev doctor` declara coherencia donde no
la hay»*.

**Por qué no es «un detalle más».** La nota (a) de INC-DEBT-061 ya resolvió una
vez que «el arreglo no está roto, no está desplegado», y esa lección se aplicó
como **dato de un caso**. Esto es la repetición, y lo que faltaba no era el
diagnóstico —ya estaba— sino el **mecanismo**: el lugar del producto donde la
condición se mira sola.

## 2. Lo entregado

Un check nuevo en `dev doctor`, `binary.build_identity`, y **cuatro estados, no
uno** — porque solo uno es un fallo:

| Estado | Condición | Veredicto |
|---|---|---|
| al día | `Matches` | **verde** |
| atrás o divergido | `Behind` / `Diverged` | **ROJO** ← la detección |
| sin checkout de sddk | no hay repo de este proyecto | **verde + N/A** |
| repo de git ajeno | hay repo, no es este | **verde + N/A** |

Y una identidad **no concluyente** (la del fallback `.git`) → **verde con
detalle**, por STOP 6 del SCOPE de `cl-build-identity`: la procedencia `git` es
dato de diagnóstico y le está prohibido decidir. Un doctor que decidiera con
ella reproduciría, en un segundo sitio, el defecto que el módulo existe para
quitar.

**Los dos estados «N/A» gobiernan el diseño y no estaban en el encargo.** STOP
1 del SCOPE dice que si el check no puede distinguir «checkout de
sddk-framework» de «repo de git cualquiera» el trabajo se descarta entero.
`resolve_root` sube desde el cwd buscando un marcador de proyecto, luego sin esa
guarda compararía el commit de sddk contra la historia de **otro** repo y
declararía `diverged` con toda la apariencia de un hallazgo. Casi todo
directorio está dentro de algún repo de git.

El marcador es el manifiesto del paquete propio, `crates/sddk-cli/Cargo.toml`:
un repo ajeno puede tener ese directorio —y ahí está el impostor— pero no un
paquete que se llame `sddk-cli`.

**Un check rojo que se equivoca entrena a ignorar los rojos**, que es peor que
no tener check. El precedente está en el propio doctor: la rama `flat_install`
reporta en verde con detalle en vez de acusar una coherencia que la instalación
nunca prometió.

## 3. El cambio de contrato, declarado

**`dev doctor` puede salir con código 1 donde antes salía con 0.** Eso es el
objetivo y no un efecto colateral. Medido:

```text
# el repo real, binario construido en HEAD
binary.build_identity: present
all_present: true
EXIT=0

# el MISMO repo, binario construido en HEAD~1
binary.build_identity: missing — FALLO: el commit del binario (68874b35) es
  ancestro del HEAD del checkout, luego el checkout tiene trabajo que el
  binario no contiene
all_present: false
EXIT=1
```

**Y desde un repo impostor** —con `crates/sddk-cli/Cargo.toml` de otro paquete—
el check sale `present`, sin falso positivo. Esa es la medición que demuestra
que el rojo del caso anterior es una detección y no una alarma.

## 4. Falsificación: 7 guards, 7/7, con una supervivencia

`26-falsify-doctor-identity.py` aplica cada mutación al **source real** y
ejecuta solo el guard afectado.

**R5 SOBREVIVIÓ A LA PRIMERA PASADA — sexta vez en la serie que un guard solo
fija el caso donde el defecto no se manifiesta, y este lo acababa de escribir
yo.** R5 usaba un repo que **no tiene** `crates/sddk-cli/Cargo.toml`, luego un
marcador demasiado permisivo nunca se distinguía del correcto: sin el fichero,
`read_to_string` falla y `unwrap_or(false)` responde igual en las dos variantes.
Le faltaba el **impostor**, un repo que tiene el directorio y no es este
proyecto, que es el único caso donde «acepta cualquier manifiesto» y «acepta
solo el nuestro» dan respuestas distintas.

Corregido **en el guard**, y el caso viejo se conserva porque R5 tiene que
seguir cubriendo el repo que no tiene nada. Que M5 caiga con el guard nuevo y
**no** con el viejo es la prueba de que añadirlo aportó.

El falsificador falló dos veces y las dos por su cuenta: una mutación no
aterrizó porque `cargo fmt` había cambiado la indentación del `match`, y otra no
compilaba. **NO MEDIBLE no es DETECTADA**, y reportarlas como tales habría
inventado una capacidad que el guard no tiene.

| Mutación | Guard | Resultado |
|---|---|---|
| M1 renombrar el check | R1 | detectada |
| M2 binario al día marcado como FALLO | R2 | detectada |
| M3 binario atrasado pasa por bueno | R3 | detectada |
| M4 acusar sin checkout | R4 | detectada |
| M5 marcador acepta cualquier manifiesto | R5 | detectada **solo tras corregir el guard** |
| M6 identidad no concluyente decide | R6 | detectada |
| M7 veredicto sin motivo | R7 | detectada |

## 5. Lo que este trabajo NO hace

- **No toca `--version`**, `install.sh` ni el contrato del bundle (STOP 4).
- **No conecta nada a la red**: solo git local.
- **No fuerza a `dev doctor` a pasar.** Si el check falla, falla.
- **No cierra INC-DEBT-064**, y el changelog no dice que lo cierre.

## 6. Por qué la deuda sigue `open`

Tres razones concretas, todas en el documento de deuda:

1. **Los binarios ya instalados declaran `source: git`**, porque se construyeron
   antes del cambio de `release.sh` que exporta `SDDK_GIT_SHA`. Para ellos el
   check es N/A y **no tiene dientes hasta la próxima release**, que está
   bloqueada por la clave KMS. No es un fallo del check; es que el check es
   honesto con lo que sabe.
2. **El binario del PATH sigue obsoleto**, y lo seguirá hasta que se publique.
3. La fase `verify` de este ciclo está abierta, como es normal: esto se
   entrega en `build`.

**Severidad sin cambio: `high`, y no `critical`.** No hay pérdida de datos; lo
que se rompe es un contrato, y en silencio. La condición de escalada del
documento —*«si `dev doctor` declara coherencia donde no la hay»*— **sigue sin
cumplirse**, y por eso la severidad no se toca: este commit le quita una vía
para que se cumpla, no la cumple.
