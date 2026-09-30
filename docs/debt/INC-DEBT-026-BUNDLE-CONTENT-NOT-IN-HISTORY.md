---
id: INC-DEBT-026-BUNDLE-CONTENT-NOT-IN-HISTORY
title: "El bundle runtime local mezcla skills de otro proyecto (gentle-ai/sdd) y no satisface su manifest"
status: closed
severity: medium
priority: P2
created: 2026-09-28
discovered_by: session-29 (coherencia del bundle instalado)
cluster_id: CL-SUPPLY-CHAIN
fingerprint: "sddk_runtime_bundle_diverges_from_published_manifest"
---

## Resolucion del alcance (session-29, verificada contra el asset publicado)

**El bundle publicado en GitHub Releases v2.0.1 esta limpio.** Descargado y
verificado fichero a fichero:

```console
$ gh release download v2.0.1 --pattern 'software-development-decision-kernel.tar.gz*'
$ sha256sum software-development-decision-kernel.tar.gz
d31b9a534fdc0badc0f394998d6129870057087a93cada28d743107a160aef10   # coincide con el .sha256 publicado

$ # recorriendo las 377 lineas de MANIFEST.sha256
  ok=377  mismatch=0  faltantes=0
```

El repositorio coincide con el publicado. **La divergencia es exclusivamente
local**, y la causa esta identificada: el `skills/` del bundle local es
contenido de **otro proyecto**.

```console
$ for f en los 12 divergentes + el ausente:
    local == ~/.config/kilo/skills/<f>     -> 11 de 12 IDENTICOS
    local == skills/<f> del repo           ->  0 de 12

$ grep -c 'agent opencode'  bundle local .../_shared/review-ledger-contract.md   -> 1
$ grep -c 'agent kilocode'  ~/.config/kilo/.../_shared/review-ledger-contract.md -> 1
```

El 12 diverge de kilo solo en el token del editor (`kilocode` vs `opencode`):
el bundle local guarda la variante de un editor y kilo la suya.

`~/.config/kilo/` se declara `__managed_by: gentle-ai/sdd` en su
`opencode.json` — un proyecto **distinto** de sddk-framework.

### Por que baja de high/P1 a medium/P2

- No hay incidente de distribucion: lo publicado verifica 377/377.
- No hay contenido ajeno al ecosistema: es contenido de un proyecto hermano,
  no desconocido.
- Si el paquete o la instalacion de `gentle-ai/sdd` escribe en
  `~/.local/share/sddk/framework/`, eso **si** es una intrusion en el bundle
  de otro proyecto y contradice la regla de cero intrusion. Ese angulo sigue
  abierto y es la accion recomendada.

## Qué es

El bundle instalado en
`~/.local/share/sddk/framework/2.0.1/` **no** satisface su propio
`MANIFEST.sha256`:

- **12 ficheros** existen pero con un hash distinto al declarado.
- **1 fichero** declarado no existe: `skills/_shared/SKILL.md`.
- `BUNDLE.toml` declara un `manifest_sha256` que no es el hash del manifest
  (defecto de calculo aparte, registrado en
  `INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE`).

Y, mas importante que el recuento: el contenido que hay dentro **no
corresponde a ninguna version de esos ficheros en el historial de este
repositorio**, y si a un proyecto ajeno.

## Evidencia (verificada, session-29)

Verificación completa del manifest del bundle instalado:

```console
$ cd ~/.local/share/sddk/framework/2.0.1
$ ok=366; mismatch/faltantes=13   # recorre las 379 lineas del manifest
  FALTA: skills/_shared/SKILL.md
  MISMATCH: skills/_shared/engram-convention.md
  MISMATCH: skills/_shared/persistence-contract.md
  MISMATCH: skills/_shared/review-ledger-contract.md
  MISMATCH: skills/_shared/skill-resolver.md
  MISMATCH: skills/chained-pr/SKILL.md
  MISMATCH: skills/judgment-day/SKILL.md
  MISMATCH: skills/judgment-day/references/prompts-and-formats.md
  MISMATCH: skills/skill-creator/references/skill-style-guide.md
  MISMATCH: skills/skill-improver/SKILL.md
  MISMATCH: skills/skill-improver/references/skill-style-guide.md
  MISMATCH: skills/skill-registry/SKILL.md
  MISMATCH: skills/work-unit-commits/SKILL.md
```

El contenido **no está en el historial**. Para
`skills/_shared/engram-convention.md`:

```console
$ sha256sum ~/.local/share/sddk/framework/2.0.1/skills/_shared/engram-convention.md
7c16af275a7ea5590dc961793bfcb7c324beeae4ddc004772d45ea40c114d9cd   # el bundle

$ git log --all --oneline -- skills/_shared/engram-convention.md
34d68c21 feat(bootstrap): import inicial del framework Software Development Decision Kernel
   # un unico commit en todo el historial, en todas las refs
   # su contenido hasheado NO coincide con 7c16af27…
```

Y no es una versión más antigua: el texto del bundle y el del repo son
**distintos en contenido**, no solo de formato.

```console
# bundle 2.0.1
NOTE: Critical engram calls (mem_search, mem_save, mem_get_observation) are
inlined directly in each skill's SKILL.md. This document is supplementary
reference — sub-agents do NOT need to read it to function.

# repositorio (HEAD)
Engram is parallel SDDK memory, never the authority for durable knowledge or
```

Los 20 ficheros tocados por esta divergencia son todos de `skills/`.

## Cuándo ocurrió

```console
$ stat -c '%y' ~/.local/share/sddk/framework/2.0.1/      # instalación
2026-09-27 21:27:25 +0200
$ stat -c '%y' .../2.0.1/skills/skill-registry/SKILL.md  # fichero divergente
2026-09-28 08:35:12 +0200
```

La instalación es del 27 a las 21:27. Los ficheros divergentes se
modificaron el 28 a las 08:35, **once horas y ocho minutos después de
instalarse**. El directorio del bundle es propiedad del usuario y los
ficheros son reales, no symlinks al repositorio:

```console
$ ls -ld .../2.0.1/skills/skill-registry/SKILL.md
-rw-r--r-- 1 rubentxu rubentxu 3715 sep 28 08:35 skill-registry/SKILL.md
```

## Qué se sabía cuando se abrió, y qué se sabe ahora

- ~~No se sabe si el bundle publicado en GitHub Releases v2.0.1 tiene el
  mismo contenido.~~ **Resuelto: el publicado verifica 377/377.** La
  hipótesis de incidente de distribución queda descartada.
- ~~No se sabe qué herramienta tocó el bundle a las 08:35.~~ **Parcial:
  el contenido coincide con `~/.config/kilo/skills/` (11 de 12 ficheros
  idénticos), que pertenece a `gentle-ai/sdd`.** Sigue sin haber un log
  que lo confirme; es correlación de contenido, no prueba de causalidad.
- **No se afirma** que haya un compromiso, ni que nadie adversarial
  escribiera ahí. Una escritura local por una herramienta, un
  `sddk dev install` parcial, un copiado manual, o un experimento de
  bootstrap producen la misma firma temporal.

## Comprobacion pendiente (la siguiente accion, no una conclusion)

**Resuelta en session-29**: el asset publicado verifica 377/377. Ver
"Resolucion del alcance" arriba. No queda pendiente comprobar el remoto.

Queda pendiente identificar **que proceso** escribio en el bundle a las
08:35. La evidencia apunta a `gentle-ai/sdd` / `~/.config/kilo/`, pero eso
es una correlacion de contenido, no un log: ningun registro dice que kilo
escribiera ahi. Para cerrarlo hace falta el mecanismo (ver accion 1).

## Relación con otros INCs

- `INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE` — el campo de anclaje está mal
  calculado. Es un defecto del repositorio, independiente de este.
- `INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY` (code-closed, distribution-open)
  — autenticidad: el checksum viene del mismo origen que el bundle. Distinto
  de este, que es un contenido que no corresponde a lo publicado.
- `INC-AUDIT-S14-DEFAULT-GATE-DISCONNECTED` (open) — la clase de fallo
  "infraestructura correcta no conectada al punto que se ejecuta". Aquí
  `sddk dev doctor` **sí** detecta la divergencia (`content.manifest:
  missing`, `all_present: false`): el gate funciona, lo que falla es que
  nadie actúa sobre él. Eso hace que este INC sea más de proceso que de
  código.

## Accion recomendada

1. **Determinar el mecanismo**: comprobar si el `bootstrap.sh` de
   `gentle-ai/sdd` (o su instalador) escribe en
   `~/.local/share/sddk/framework/`. Si lo hace, está escribiendo en el
   bundle de otro proyecto y hay que actuarlo: los dos proyectos comparten
   `~/.local/share/sddk/`, que es territorio compartido por construcción.
   Es la acción que decide si esto es ruido cosmético o una intrusión real.
2. `sddk dev install` para restaurar el bundle desde el origen de verdad y
   confirmar que la divergencia desaparece. Si reaparece, ya no es un
   incidente aislado sino algo que escribe en el bundle de forma repetida.
3. Explicar por que `sddk dev doctor` reporta `content.manifest: missing`
   durante varias sesiones sin que nadie lo trifique. La senal estaba
   disponible; la respuesta fue descartarla como "instalacion vieja" — y
   esa descriptora era incorrecta, como se ha visto.

## Cierre por caducidad de criterio (session-45, 2026-09-30)

El criterio de session-29 era: «el bundle local no satisface su
`MANIFEST.sha256`: 12 ficheros divergentes y 1 ausente». Ese criterio
**ya no se sostiene**, y una alerta cuyo criterio no se verifica no es
deuda real.

Verificado contra el bundle local instalado hoy
(`~/.local/share/sddk/framework/2.2.27`, `readlink` a
`/var/home/rubentxu/...`):

- **377/377 ficheros verificados por sha256 contra `MANIFEST.sha256`,
  0 problemas** (relectura completa del manifest, no una muestra).
- `sddk dev doctor --prefix ~/.local/bin` ⇒ `content.manifest: present`,
  `binary.bundle_coherence: present`, `all_present: true`.

Lo que cambió no fue el repo sino **el bundle instalado**: los releases
posteriores (v2.2.17 en adelante, publicados por CI) sustituyeron la
instalación contaminada de session-29. La hipótesis de entonces —que
`bootstrap.sh` de `gentle-ai/sdd` escribía en
`~/.local/share/sddk/framework/` del bundle de otro proyecto— **queda
sin refutar**, pero **ya no tiene consecuencia observable**: no hay
contenido ajeno en el bundle vigente.

Se cierra como **caducada por observación**, no como corregida: el
mecanismo de intrusión, si existió, no se identificó. Si reaparece un
bundle divergente, la incidencia reabre con esta evidencia como
punto de partida.
