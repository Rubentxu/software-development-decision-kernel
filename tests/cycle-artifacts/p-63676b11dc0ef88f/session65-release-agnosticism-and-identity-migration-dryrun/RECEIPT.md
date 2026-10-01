# RECEIPT — no-agnosticidad de `release plan`/`apply` (INC-DEBT-051) y dry-run de la migración de identidad

**Slice:** `session65-release-agnosticism-and-identity-migration-dryrun`
**Fecha:** 2026-10-01 · **Baseline:** `ff10f662` (HEAD == origin/main al abrir)
**Workflow:** `A-lite` · **Deuda:** INC-DEBT-051 (nueva) · **INC-DEBT-050:** addendum
**Autorización del operador:** backup + dry-run **sin ejecutar la migración**

---

## §1 Qué se entrega y qué no

Se entrega dos cosas, y explícitamente **no** una tercera:

1. **INC-DEBT-051** registrada: `sddk release plan` y `release apply` no son
   utilizables en ningún proyecto que no sea Rust.
2. **`scripts/migrate_project_identity.py`** con `audit`, `plan`, `backup` y
   `apply`, más `tests/test_migrate_project_identity_mirror.py` que fija el
   espejo contra el Rust real.

**No se ejecuta la migración.** El operador autorizó backup y dry-run. `apply`
permanece fail-closed y sin ejecutar, y las tres vías de rechazo se han
comprobado de forma explícita (§5).

## §2 INC-DEBT-051 — por qué es un defecto y no una limitación

`sddk release plan --tag v0.45.0` sobre PipelineK (Kotlin/Gradle) falla:

```text
error: VERSION LOCKSTEP ERROR: could not read …/pipeline-kotlin/Cargo.toml:
No such file or directory (os error 2)
```

`ensure_version_lockstep` (`crates/sddk-engine/src/version.rs`) **hardcodea**
`root.join("Cargo.toml")` y no consulta ningún adapter; lo invocan
`release_cmd.rs:668` (`plan`) y `:847` (`apply`).

AGENTS.md §2.3 declara que la política SDDK *"es agnóstica de
lenguaje/build/test runner y debe funcionar igual en repos JVM, JS/TS, Python,
Go, .NET, C/C++, **Bazel** o polyglot mediante adapters/capabilities"*. Gradle
y Bazel son el mismo caso. Una desviación de una convención dura se corrige o
se deroga explícitamente; no se acepta en silencio.

**Medido, no generalizado:** `plan` y `apply` abortan; `dist` y `verify`
fallan antes por argumentos o ruta y quedan **no evaluados**.

## §3 El espejo: lo que la verificación diferencial encontró

El script deriva `project_id` reimplementando `sddk_domain`. La primera
versión de ese espejo **no coincidía con el Rust**. Comparación caso a caso
contra el binario real (`sddk project resolve --remote … --format json`, y el
binario debug del HEAD, que coinciden entre sí):

| forma de remote | Rust | espejo v1 | veredicto |
|---|---|---|---|
| `git@github.com:owner/repo.git` | `https://github.com/owner/repo` | `None` | **rechazada** la forma de remote más común |
| `https://github.com:22/owner/repo` | conserva `:22` | lo borraba | id distinto |
| `git@github.com:443/owner/repo` | `…/443/owner/repo` | `None` | id distinto |
| `https:///owner/repo` | rechaza | aceptaba | **mintaba un id** |
| `https://github.com:/owner/repo` | rechaza | aceptaba | **mintaba un id** |
| `https://[::1]x/owner/repo` | rechaza | `UnboundLocalError` | **crash** |

Causas: un conjunto global `{"443","22"}` de puertos por defecto en vez de
por esquema (`https`→443, `ssh`→22, `scp`→ninguno); rechazo de `@` en la
forma scp; `str.isdigit()` que acepta no-dígitos Unicode donde el Rust exige
ASCII; y la rama del dígito vacío, que en Rust es *vacuo-cierto* y en Python
falso.

**Consecuencia si no se hubiera detectado:** un `apply` habría escrito ids
equivocados en ledgers reales. Es exactamente la clase de corrupción que el
propio script existe para evitar.

Dos defectos más, ambos silenciosos:

- **`audit` no encontraba ningún receipt.** El patrón se construía con `/` y
  `p-*` se tomaba como nombre literal de directorio: `glob` devolvía lista
  vacía **sin error** y `audit` imprimía `selfcheck: ok` con 0 receipts —
  un inventario vacío indistinguible de "no hay nada que migrar". Corregido a
  `share.glob("projects/p-*/workspaces/*/adoption.json")` y con guarda que
  **falla con exit 4** si el inventario sale vacío.
- **`audit` siempre imprimía JSON.** `set_defaults(func=audit)` pasaba el
  `Namespace` de argparse a un parámetro `bool`, y un `Namespace` siempre es
  truthy: el informe legible era inalcanzable.

Y un defecto del propio control de confianza: el selfcheck anterior pasaba el
remote **crudo** a `stable_project_id`, así que **nunca ejercitaba
`normalize_remote_url`**, que es justo la función capaz de cambiar un
`project_id` en silencio.

## §4 Remedición y corpus dorado

El corpus (21 normalizaciones, 8 rechazos, 2 ids heredados) se **generó desde
el binario real**, no se escribió a mano. Escribirlo a mano ya falló una vez:
puse `:443` donde el Rust dice `/443`, y el selfcheck lo detectó.

El corpus incluye a propósito las formas donde una traducción ingeniosa
diverge, y los rechazos que un espejo mal escrito *aceptaría*.

**Dos trampas de verificación que hubo que corregir durante el trabajo:**

- `Über` (U+00DC+ber) y `über` (U+00FC+ber) **se ven iguales** en pantalla y
  en `repr`, pero dan `project_id` distinto. La distinción ASCII/Unicode del
  host es observable y **no estaba pineada**: el corpus no tenía ningún host
  no-ASCII. Se añade `https://ÜBER.example/Owner/Repo` y
  `https://MÜNCHEN.de/…`; el test compara por codepoint.
- El harness de depuración leía un `.pyc` obsoleto de
  `scripts/__pycache__/` y por eso `_ascii_lower` *parecía* aplicar Unicode.
  Ejecutar el script directamente (`__main__`) no usa caché; los harnesses que
  importan por `importlib`, sí. Caché eliminada y `PYTHONDONTWRITEBYTECODE=1`
  en las ejecuciones posteriores.

## §5 Falsificadores y gates

**Falsificadores F60–F67** (mutación → el corpus debe fallar):

| id | mutación | veredicto |
|---|---|---|
| F60 | puerto por defecto global en vez de por esquema | OBSERVED |
| F61 | rechaza credenciales en forma scp | OBSERVED |
| F62 | acepta authority vacío | **NO-APLICA**: mutación sin efecto |
| F63 | acepta puerto vacío | OBSERVED |
| F64 | acepta resto IPv6 sin `:` | OBSERVED |
| F65 | minúsculas Unicode en el host | OBSERVED |
| F66 | acepta segmento vacío en el path | OBSERVED |
| F67 | no quita el sufijo `.git` | OBSERVED |

F62 se registra como **NO-APLICA y no cuenta**: quitar `if not authority` es
comportamentalmente neutro porque `if not host` ya cubre ese caso. Es una
rama redundante en el espejo **y en el Rust**, y un falsificador que no puede
fallar no es un falsificador. Se conserva la rama por fidelidad al original.

Cada falsificador tiene que cumplir tres cosas: que el mutante cambie el
fichero, que el comportamiento cambie en el caso concreto y que el corpus lo
pille. El primer intento de este harness fallaba las dos primeras: su `sed`
buscaba el texto ya corregido, no mutaba nada y "observaba" un PASS vacío.

**Gates ejecutados:**

```text
selfcheck (espejo vs Rust)              21 norm + 8 rechazos + 2 ids   OK
corpus diferencial in-process           agree=23  diverge=0
tests/test_migrate_project_identity_mirror.py                        10/10 OK
test falsificado (romper el espejo)      FAILED (failures=3)  ← verificado
audit                                    117 receipts · 25 huérfanos
tests/test_debt_index_coherence.sh       PASS=10 FAIL=0
git diff --check                         limpio
```

## §6 Backup y dry-run (lo que el operador autorizó)

```text
plan:    15 project_id a migrar
digest:  b98e9a8d2a76bf8d2797744d6de29028aa8bcf6c8df82d481d5d9c8053b6292a
backup:  /var/home/rubentxu/.sddk-migration-backup-20261001/20261001T145101Z
         7033 ficheros copiados y verificados con sha256 (535 MiB)
```

Verificación independiente del backup: `find -type f` → 7035 = 7033 copiados +
`BACKUP-MANIFEST.json` + `BACKUP-COMPLETE`, que escribe el propio script.

**Las tres vías de `apply` se comprobaron rechazando, sin escribir nada:**

```text
A) --confirm vacío      -> exit 3, "--confirm incorrecto. No se escribe nada."
B) --confirm deadbeef   -> exit 3, idem
C) backup inexistente   -> exit 3, "falta BACKUP-COMPLETE. No se escribe nada."
```

Storage tras el ejercicio: 117 receipts y 248 directorios `p-*`, sin cambios.

## §7 Remedición de la magnitud de INC-DEBT-050 (addendum, no reescritura)

```text
                            session-63      session-65
receipts revisados:            104            117
ids que NO coinciden:           25 (24%)       25 (21%)
project_id viejos:              16             15
remotos con casse distinta:     13             15
```

Los **25 huérfanos se confirman y son estables**; el total crece porque los
gates del repo crean adopts de prueba. El **"16 / 13" de la frase de resumen
era incorrecto desde que se escribió**: la lista de detalle del propio
documento ya enumeraba 15 ids sobre 15 remotos, que es lo que reproduce la
remedición. La lista era correcta; la frase que la resumía, no.

## §8 Lo que este slice NO hace

- **No ejecuta la migración.** `apply` no se ha ejecutado y no debe sin
  autorización nueva y explícita.
- **No recalcula `configuration_hash`.** Cubre `project_id`, `workspace_id` y
  paths; replicar la serialización de `serde_json` a mano es una fuente de
  corrupción. Se regenera con `sddk adopt apply`.
- **No reescribe evidencia histórica.** Los `.md`/`.json` de `cycle-artifacts`
  que citan el id viejo se dejan como están.
- **No arregla `release plan` para proyectos no-Rust.** Eso es un contrato
  nuevo (dónde vive la versión, qué pasa si no se expone, si el lockstep sigue
  siendo exigible) y exige SCOPE + ADR.

## §9 Riesgo residual

Un espejo fiel hoy depende de que `sddk_domain` no cambie sin que se actualice
el corpus. El riesgo real no es que diverja hoy —el corpus lo detecta en el
primer `audit`— sino que alguien añada un caso al normalizador en Rust sin
regenerar el corpus, momento en el que `audit` seguiría verde sobre un espejo
obsoleto. `apply` re-verifica el plan contra el estado real antes de escribir,
pero esa re-verificación usa el mismo espejo. **Cerrarlo exige regenerar el
corpus desde el Rust como paso de integración, no como disciplina.**

## §10 Estado

- `git diff --check` limpio · `test_debt_index_coherence` PASS=10 FAIL=0
- Sin cambios de código de producción; sin cambios de comportamiento del CLI
- Sin release: la de v2.5.0 sigue bloqueada por `musl-gcc` ausente
- PipelineK 0.45.0 sin publicar: publicar la RC requiere autorización del
  operador y es condición previa del harness
