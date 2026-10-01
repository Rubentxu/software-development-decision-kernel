---
id: INC-DEBT-045-PUSH-FAILCLOSED-CASE-MEASURED-WRONG-REPO
title: el caso que certifica el fail-closed del pre-push preguntaba al repositorio equivocado, asi que nunca lo verifico
status: resolved
severity: high
priority: P1
detected_at: 2026-10-01
detected_in_session: session-57
closed_in_session: session-57
component: release-governance
surface: tests/test_push_prevention_hook.sh
references:
  - githooks/pre-push
  - docs/debt/INC-DEBT-040-PREPUSH-BUMP-PREDICATE-UNSATISFIABLE-FOR-DECLARED-RELEASE.md
  - docs/architecture/adrs/
fingerprint: "failclosed_case_hook_runs_in_runner_cwd_not_fixture"
---

## Qué es

`tests/test_push_prevention_hook.sh` tiene un caso que afirma certificar que el
pre-push **falla cerrado** cuando la consulta de tags al remote no puede
responder (INC-DEBT-040 variante 3). Ese caso **nunca exertitó ese
comportamiento**: preguntaba al hook sobre otro repositorio.

Tres defectos encadenados, los tres en el test:

1. **Scope del `cd`.** El `cd "$dir/clone"` vivia dentro del cuerpo de
   `if ( ... )`. Al terminar ese subshell, `hook_direct_case` se ejecutaba en el
   **CWD del runner**. La maniobra `git remote rename origin
   origin-unreachable`, de la que depende el caso, nunca afectaba a la llamada;
   el hook evaluaba el rango **real** del repo, que si contiene un bump
   legitimo => ACCEPT. El caso reportaba el veredicto de otro repositorio.

2. **stdin malformado.** `git rev-parse origin/main` sobre un ref inexistente
   imprime el NOMBRE del ref en stdout y aun asi sale con codigo distinto de
   cero, asi que el `||` fallback concatenaba el zero sha detras de ese nombre y
   partia la entrada del hook en dos lineas. Ahora usa `--verify -q`, que no
   imprime nada al fallar.

3. **Veredicto contaminado.** El comando de limpieza esta enrutado por un
   wrapper en algunos entornos que imprime a stdout, y estaba dentro del
   `$( )`, de modo que `res` nunca podia casar exactamente con PASS/FAIL: el
   caso caia siempre en la rama de "fixture error" **con independencia de lo
   que hubiera decidido el hook**. El veredicto se extrae ahora como ultimo
   token, que es donde va.

## Por qué importa mas de lo que parece

Este caso es la **unica** verificacion del fail-closed del pre-push, que es lo
unico que impide que un rango sin contrato de release llegue a `main`. Un caso
que no ejercita su escenario no es un caso que pasa: es una afirmacion sin
respaldo con apariencia de evidencia. Es exactamente la clase de defecto que la
via C3l existe para eliminar, y en este caso se presenta en el propio artefacto
que supposedmente verifica el control de admision.

Ademas, el fallo era **silencioso y dependiente del entorno**: con un `rm` que
no imprimiera, el caso habria caido en la rama `FAIL` y el gate se habria
manifestado como "el hook no falla cerrado" — una conclusion opuesta y tambien
falsa. Un caso cuyo resultado depende de como el entorno enrute sus comandos no
puede usarse como evidencia en ninguna direccion.

## Alcance y limites honestos

- El **hook** nunca estuvo roto: `PASS=48 FAIL=0` tras el arreglo, y una
  reproduccion manual con el input bien formado rechaza correctamente. Lo que
  estaba roto era la verificacion.
- El caso es **falsable** y se comprobó: mutando el hook para admitir cuando la
  consulta de tags falla, el caso cae a
  `[direct-expected REJECT, got ACCEPT]` y la matriz a FAIL=1 (falsificador
  F10, OBSERVED en session-57).
- Este debt **no** cierra la variante (3) de INC-DEBT-040 (comparar contra el
  ultimo tag publicado en lugar de `origin/main..HEAD`), que sigue abierta y
  alteraria un gate de admision.
- El fixture seguia necesitando `git config core.hooksPath` porque el hook se
  invoca por ruta absoluta; eso no se toco.
