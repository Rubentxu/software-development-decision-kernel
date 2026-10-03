# Follow-up — session-69s bis 7

Hallazgo **fuera del WorkItem** `c3n-production-boundary-certification`, clasificado
como **follow-up** según el §4 del protocolo de scope discipline. No se implementa
aquí; se registra con su evidencia reproducible.

## FU-1 — `cycle lock acquire` sin `--cycle` es insatisfacible por construcción

**Medido**, con el binario de `HEAD` (`a5c9b4bb`), sobre el ledger real:

```text
$ sddk cycle start --root . --scope . --name c3n-production-boundary-certification --path a-full
cycle_id: p-63676b11dc0ef88f/c3n-production-boundary-certification
status: OPEN   phase: explore   lease: none

$ sddk cycle status
error: no active cycle found for project p-63676b11dc0ef88f

$ sddk cycle lock acquire --owner <actor>
error: no active cycle found for project p-63676b11dc0ef88f

$ sddk cycle lock acquire --owner <actor> --cycle p-63676b11dc0ef88f/c3n-...
lease: owner=<actor> fencing_token=1 acquired_at_ms=... expires_at_ms=...

$ sddk cycle status
cycle_id: p-63676b11dc0ef88f/c3n-production-boundary-certification
phase: explore   path: A-full
lease: owner=<actor> fencing_token=1 ...
```

**La causa está en el código, no es uso.** `InferenceError::NoActiveCycle` está
documentado en `crates/sddk-cli/src/cycle.rs:44` como *«Zero active **leases** for
the resolved project (S3)»*: un ciclo es «activo» **si y solo si tiene una lease
viva**. Y `cycle lock acquire` resuelve el ciclo destino **por esa misma lease viva**
—la que el comando debe crear—.

**Consecuencia:** la **primera** lease de un ciclo no es adquirible por la vía
implícita. Todo ciclo nuevo nace inerte y sólo se activa pasando `--cycle`
explícito. No es un bloqueo (hay ruta), pero es exactamente la clase de fricción
que el propio repo ya pagó en `gap6-lock-repair` y en `gap6-foreign-cycle-typed-error`.

**Lo que este follow-up NO afirma:** que `lock acquire` sin `--cycle` deba
inventarse un objetivo. La semántica correcta (adquirir sobre un OPEN sin lease,
o sobre el lease expirado) es decisión de diseño, no un bug de una línea.

**Coste de no resolverlo:** cada agente que abra un ciclo tiene que descubrir que
`--cycle` es obligatorio en la primera adquisición. Es un paso que no se puede leer
en `--help` (el hint dice `start one with: ...`, que es lo ya hecho).

## FU-2 — `gap6-lock-repair` declara `artifacts: 1` y su directorio está vacío

```text
$ sddk cycle status --cycle p-63676b11dc0ef88f/gap6-lock-repair
status: OPEN   phase: specify   path: A-min   artifacts: 1   lease: none

$ ls -la ~/.local/share/sddk/projects/p-63676b11dc0ef88f/cycle-artifacts/\
      p-63676b11dc0ef88f/gap6-lock-repair
total 0        (cero ficheros)
```

El recuento dice 1 y el disco dice 0. Es la misma clase que `INC-DEBT-063`
(un `cycle_id` declarado que no corresponde a filas reales) y que
`PROCESS/SQLITE_DURABLE` (una declaración que no resiste un `ls`). No se investiga
aquí: es tangencial al WorkItem.

## FU-3 — RETIRADO: el guard ya existía, y mi hallazgo era falso

**Este follow-up se escribió y se retiró en la misma sesión.** Se deja el texto
retirado porque borrar la corrección sería repetir el error que la corrige.

**Lo que afirmé:** que los 27 caracteres no latinos de `SESSION-JOURNAL.md` eran un
hallazgo nuevo, y que la comprobación de alfabetos «no estaba cableada a ninguna
parte» porque yo la corría a mano.

**Las dos cosas son falsas, y ambas se desmienten mirando antes de escribir:**

1. **El guard existe**, es `tests/test_docs_script_contamination.py`, de session-66,
   y es **mejor** que el que iba a escribir: allowlist por `ruta:línea` con motivo
   individual, tres reglas anti-pudrimiento (una entrada que ya no coincide es FAIL;
   una entrada cuyo fichero ya no tiene el carácter es FAIL; un fichero contaminado
   sin entrada es FAIL), y el recuento sale del contenido, no de una constante.
2. **Está cableado**: `scripts/release.sh:275`, dentro del bucle de gates.
   `python3 tests/test_docs_script_contamination.py` → **`RESULT: PASS`**.
3. **Las 27 ocurrencias del diario están excluidas a propósito.** `EXCLUDED_FILES`
   del propio guard contiene `docs/roadmap/SESSION-JOURNAL.md`, con el motivo
   escrito: el diario es **append-only** y una entrada antigua no se reescribe nunca.
   **No es un hueco: es una política declarada.**

**Por qué importa más que el error.** Es la **cuarta** vez seguida en esta sesión
que afirmo algo sobre el repo sin medirlo, y las cuatro son mías y de los commits
inmediatamente anteriores:

| # | Afirmación | Realidad |
|---|---|---|
| 1 | «`PROCESS/SQLITE_DURABLE` no aparece en ningún otro documento» | aparece en 5 sitios más, uno en `crates/` |
| 2 | «tres de cinco tests `*_e2e`» | son **cuatro** de cinco: leí los que inspeccioné y no conté |
| 3 | «los 27 caracteres son un hallazgo, y el check no está cableado» | el guard existe, pasa, y está en `release.sh:275` |
| 4 | el guard nuevo que iba a escribir, sin duplicar | habría shadowado uno mejor, con 285 falsos positivos sobre `Δ`/`Σ`/`θ` legítimos |

**El patrón tiene nombre: generalizo desde los casos que examiné y lo escribo como
si fuera un hecho medido.** En los cuatro casos la lecturaPreliminary era
correcta —el `SQLITE_DURABLE` estaba en la tabla, los `_e2e` estaban en el fichero,
el guard de C3n.1 no leía la otra matriz— y lo que falla es el salto de «esto que
miré» a «esto que es». La defensa no es leer más: es que **ninguna afirmación sobre
el repo salga de un fragmento sin el `grep` que la convertiría en medida**, y eso
incluye las afirmaciones sobre lo que *no* existe.

**Lo que sí queda de este intento, y es útil:** la medición de que **0** caracteres
del repo están *estrictamente dentro de una palabra* (letra latina a ambos lados), y
que los otros **313** son símbolos sueltos. Eso dice que la regla «no pegado dentro
de una palabra» **no es implementable** —daría 0 aciertos y 313 ruido— y que el
criterio que de verdad funciona es el del guard existente: *prosa en español con un
término sustituido por otro sistema de escritura*, con allowlist y motivo, y sin
intentar distinguir la notación matemática legítima de la corrupción.
