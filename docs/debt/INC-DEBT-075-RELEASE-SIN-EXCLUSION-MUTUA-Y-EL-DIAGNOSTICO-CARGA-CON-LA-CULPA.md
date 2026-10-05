---
id: INC-DEBT-075
title: "release.sh no tiene exclusion mutua: dos releases del mismo tag pueden coexistir, y cuando lo hacen el diagnostico de retencion dice la verdad mientras el test que lo vigila le echa la culpa al producto"
status: open
severity: high
priority: P1
fingerprint: "release_has_no_mutual_exclusion_and_retention_diagnostic_is_blamed_for_real_contention"
fingerprint_aliases: []
cluster_id: CL-AUTHORITY-SPLIT
created: 2026-10-05
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-05
detected_in_session: session-82
component: release pipeline / diagnostico
surface: scripts/release.sh y tests/test_release_diagnostics.sh (caso C9)
related: [INC-DEBT-073, INC-DEBT-074]
references:
  - scripts/release.sh
  - tests/test_release_diagnostics.sh
  - scripts/lib/release_diagnostics.sh
---

# INC-DEBT-075 — release.sh no tiene exclusion mutua, y el diagnostico de retencion carga con la culpa de un entorno contendsido

## Que se midio

**MEDIDO, no hipotetico.** Intentando publicar la 2.11.0 (session-82)
convivieron **dos ejecuciones del release sobre el MISMO checkout, el MISMO
HEAD y la MISMA version**:

| pid | sesion | log | PPID |
|---|---|---|---|
| 38340 | session-82 (este bloque) | `release-2110-try4.log` | 26812 |
| 99022 | otra sesion | `/tmp/rel-2.11.0.log` | 3001 (`systemd --user`) |

Ambas con `HEAD = a0a59dd2` y workspace `2.11.0`.

## La consecuencia, medida

El `cargo test --workspace --offline` del release ajeno (pid **100388**,
`PPID 99022`) retuvo el `CARGO_TARGET_DIR` compartido mientras corria
`test_release_diagnostics.sh`. Las 4 aserciones de **C9** cayeron con:

    el target dir compartido esta retenido: pid 100388, lleva 98s esperando

**El diagnostico no mintio: habia retencion real.** Lo que fallo fue el
*supuesto* del test, que afirma «sin retencion no inventa ninguna» sin
comprobar que el target dir compartido este libre. El release de esta sesion
murio con `RELEASE_RC=1` en el paso 1b por eso; el de la otra sesion paso esa
misma prueba porque la contencion ya no existia.

## La DIRECCION, medida, porque cambia el arreglo

`CURRENT.md` (session-83) atribuye el proceso competidor a session-82: *«dejo
un release.sh corriendo en background ... que sobrevivio al cierre»*. **Es al
reves**, y la distincion decide el remedio:

| hecho | medicion |
|---|---|
| El C9 que cayo fue el de session-82 | pid `100388` aparece **5 veces** en `release-2110-try4.log` y **0** en `/tmp/rel-2.11.0.log` |
| El `cargo` retenedor era de la otra sesion | `PPID 100388 = 99022 = bash scripts/release.sh`, cuyo `PPID 3001` es `systemd --user` |
| El release de session-82 no sobrevivio | pid 38340 no existe; su log cierra `RELEASE_RC=1` a las 09:31:22 |
| El C9 de la otra sesion pasa | `autofalsacion del diagnostico de release: PASS=20 FAIL=0 SKIP=0` |

Las dos sesiones **lanzaron un release por su cuenta** —session-82 a las 09:24
y la otra a las 09:29:30— y **nada impidio que coexistieran**. La lectura
«un agente dejo un huerfano» se arregla con higiene del agente y vuelve a
pasar la proxima vez que dos agentes trabajen a la vez. La lectura medida se
arranca con **exclusion mutua en el release**, y sobrevive a que el operador
cambie de sesion.

Lo que **si** coincide con session-83, y no se pisa: el diagnostico no
mintio. La contencion era real.

## Lo que NO es este defecto

- **No** es un falso positivo del diagnostico: existia un `cargo` real
  esperando el lock. La deteccion fue correcta.
- **No** es un fallo de `test_release_diagnostics.sh` sobre lo que mide: sabe
  nombrar el pid ajeno y distinguir un descendiente propio. Lo que no sabe es
  que la contencion pueda provenir de **otro release**, y por eso el caso
  «sin retencion» necesita una precondicion que nadie comprueba.
- **No** se ha medido que dos releases hayan llegado ambos al paso 9 ni que
  un tag se haya publicado dos veces. La ventana existe; el incidente, no.

## Por que es P1

1. **Consecuencia destructiva posible**: dos releases del mismo tag pueden
   coexistir y ambos llegar al paso 9. Quien gane escribe el artefacto; quien
   pierda, falla o sobrescribe con `--force`.
2. **Envenena el diagnostico hacia el conjunto de la herramienta**: un guard
   que blames al producto cuando el entorno esta contendsido **entrena a
   ignorar al guard**. Este fallo se leyo como «el diagnostico no dice la
   verdad» (FAIL=4 en C9) cuando la causa raiz eran dos sesiones
   publicando la misma version.
3. **Deja procesos huerfanos**: al morir un release, su `cargo` encolado puede
   sobrevivir (medido: pid 100388, **4+ minutos despues** de que su release
   murio). El `trap` de cleanup no alcanza a un proceso que cargo dejo
   esperando el lock, y el siguiente intento hereda una contencion que ya no
   pertenece a nadie.

## Salidas (para el bloque que lo cierre)

1. **Exclusion mutua en `release.sh`**: lock por repo+version, y el segundo
   intento ABORTA nombrando el pid del primero, no compite por el lock de
   cargo.
2. **Precondicion declarada en C9**: antes de afirmar «sin retencion»,
   comprobar que el target dir esta libre; si no lo esta, declarar el caso
   **NO APLICABLE con su motivo**, nunca `[FAIL]`.
3. **Reaper de procesos huerfanos** al morir un release en el paso 1.

## Falsador previsto

- **M1**: lanzar dos `release.sh` a la vez -> el segundo aborta con el pid
  del primero (no compite por el lock de cargo).
- **M2**: con un `cargo` ajeno reteniendo el target dir, C9 declara el caso
  **no aplicable** con motivo, nunca `[FAIL]`.
- **M3**: matar un release a mitad del paso 1 y comprobar que no queda
  ningun `cargo` suyo vivo esperando el lock.

## Requisito de correccion

Ninguno de los tres se resuelve con `--force` ni repitiendo el release. Si la
2.11.0 llegara a publicarse mientras corrian dos releases, habria que
verificar el artefacto contra la API y el CDN antes de aceptarlo, igual que
con cualquier release. La 2.11.0 **si** se publico y **si** se verifico
contra API y CDN (9 assets, `draft=false`, `prerelease=false`, sha256
coincidente), pero bajo concurrencia: ese es el estado en que quedo, y por
eso esta deuda es P1 y no P3.
