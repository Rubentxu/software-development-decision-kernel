---
id: INC-AUDIT-S14-RELEASE-FORCE-VERSION-ERGONOMICS
title: "release.sh override de versión funciona pero su warning se pierde en un log de 400 líneas"
status: open
severity: low
priority: P2
created: 2026-09-27
discovered_by: session-14 release execution (OBSERVED, no teórico)
cluster_id: CL-RELEASE-DISCIPLINE
fingerprint: "sddk_release_force_version_ergonomics_v1"
---

## Qué pasó (observado, no hipotético)

Session-14 launching `v1.173.0` con
`SDDK_RELEASE_ADMISSION_MODE=v2 bash scripts/release.sh` (sin
`--force-version`). Resultado: se publicó **v2.0.0**, no v1.173.0.

Motivo: `release.sh:361-363` sí plumbéa `--force-version` a
`release-bump.sh`, pero el operador no lo pasó. Sin el flag,
`release-bump.sh` re-deriva `major` desde el `!` de `refactor(cli)!` y
el paso 2.5 sobrescribe el `TAG` derivado del workspace
(`release.sh:373-375`):

```
semver tag overrides workspace-derived tag: v1.173.0 → v2.0.0
```

**El pipeline NO tiene el bug.** Implementación correcta, invocación
incorrecta. El override de versión en `Cargo.toml` es decorativo si no
se pasa también el flag por el mismo comando.

## Por qué importa (y por qué es low, no high)

El daño real fue pequeño: el bundle y el binario instalados reportan
`1.173.0`, `binary_min_version/max = 1.173.0`, `dev doctor
all_present: true`, round-trip de distribución OK. La release es
funcionalmente correcta y el gate público 9b pasó 9/9.

Lo que falló fue una **afirmación pública**: el tag `v2.0.0` anuncia una
ruptura mayor de API que no ocurrió para el consumidor (sin registry,
sin clientes externos). Es ruido documental, no un defecto de software.

Evidencia: log de release en session-14, `v2.0.0` publicado
2026-09-27T18:19:02Z, tag SHA `db043b4` == HEAD.

## Agravante de detectabilidad

El warning `operator forced version override: …` solo aparece cuando
el flag SÍ se pasa. Cuando NO se pasa y el paso 2.5 sobrescribe el tag,
la señal es un único `warn` de una línea en un log de ~400 líneas con
14 pasos. Es exactamente la clase de señal que se pasa por alto en un
log largo — y por eso casi no lo detecto: creí que el pipeline estaba
roto.

## Riesgo de recurrencia

Cualquier release futuro cuyo rango contenga un `!` interno
(refactor público sin ruptura de consumidor) publicará un major
involuntario mientras el operador olvide el flag. El algoritmo es
correcto; la trampa es de ergonomía, y ya se ha materializado una vez.

## Opciones

- **(a) `--force-version` obligatorio cuando el operador overridea**:
  derivar el tag en 2.5 solo si `FORCE_VERSION` está vacío; si no,
  usar el flag. Reduce el camino de fallo a cero. Coste bajo, cambio
  pequeño.
- **(b) `warn` → `ok` con el tag final en stdout resumido** al cierre
  de cada paso, para que la decisión de versión sea visible sin grep.
  Coste bajo, mejora de señal.
- **(c) Pre-flight que compare el tag que se publicará con el que el
  operador espera**, fallando si difieren sin flag explícito.

## Nota sobre este INC

Este hallazgo **no** es un bug del código: el código hace lo correcto.
Se registra porque el modo de fallo (decisión de versión tomada
implícitamente, señal enterrada en un log largo) es un riesgo
operativo real y merece una guarda, no una corrección de
implementación.

## Estado

Abierto. No ejecutado en esta sesión: publicar un release es una
decisión del operador, y el release v2.0.0 ya está público. La opción
(a) es la que más valor da por LOC tocada; se implementa cuando se
abra un WorkItem de release-discipline.
