---
id: INC-DEBT-026-BUNDLE-CONTENT-NOT-IN-HISTORY
title: "El bundle runtime instalado contiene texto que no corresponde a ningún commit del repositorio"
status: open
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-29 (coherencia del bundle instalado)
cluster_id: CL-SUPPLY-CHAIN
fingerprint: "sddk_runtime_bundle_diverges_from_published_manifest"
---

## Qué es

El bundle instalado en
`~/.local/share/sddk/framework/2.0.1/` **no** satisface su propio
`MANIFEST.sha256`:

- **12 ficheros** existen pero con un hash distinto al declarado.
- **1 fichero** declarado no existe: `skills/_shared/SKILL.md`.
- `BUNDLE.toml` declara un `manifest_sha256` que no es el hash del manifest
  (defecto de cálculo aparte, registrado en
  `INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE`).

Y, más importante que el recuento: **el contenido que hay dentro no
corresponde a ninguna versión de esos ficheros en el historial del
repositorio**.

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

## Qué NO se afirma y qué NO se sabe

- **No se afirma** que haya un compromiso, ni que nadie adversarial
  escribiera ahí. Una escritura local por una herramienta, un `sddk dev
  install` parcial, un copiado manual, o un experimento de bootstrap producen
  exactamente esta firma temporal. El contenido no está en el historial de
  *este* repositorio, pero eso solo descarta "vino de un commit de aquí";
  no identifica el origen.
- **No se sabe** qué herramienta tocó el bundle a las 08:35. No hay log que
  lo registre.
- **No se sabe** si el bundle publicado en GitHub Releases v2.0.1 tiene el
  mismo contenido. Esto solo se ha verificado contra la copia local. La
  diferencia importa: si el remoto está limpio, el INC baja a
  "entorno local modificado" y la prioridad cae; si el remoto tiene lo
  mismo, es un incidente de distribución y sube a severidad crítica.

## Comprobación pendiente (la siguiente acción, no una conclusión)

Descargar el asset publicado de v2.0.1 y verificar su `MANIFEST.sha256`
contra los ficheros del tarball:

```bash
gh release download v2.0.1 --pattern 'bundle.tar.gz' --dir /tmp/sddk-verify
cd /tmp/sddk-verify && sha256sum -c <(...)   # y el gate 9b ya exige HTTP 200
```

Mientras eso no se haga, el alcance correcto es **"copia local
divergente, alcance remoto desconocido"**, y así queda escrito.

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

## Acción recomendada

1. `sddk dev install` para restaurar el bundle desde el origen de verdad y
   confirmar que la divergencia desaparece. Si reaparece, ya no es un
   incidente aislado sino algo que escribe en el bundle de forma repetida.
2. Verificar el asset remoto de v2.0.1 (comprobación pendiente de arriba).
3. `reconcile`/documentar por qué `sddk dev doctor` reporta
   `content.manifest: missing` durante varias sesiones sin que nadie lo
   trifique. La señal estaba disponible; la respuesta fue descartarla como
   "instalación vieja".
