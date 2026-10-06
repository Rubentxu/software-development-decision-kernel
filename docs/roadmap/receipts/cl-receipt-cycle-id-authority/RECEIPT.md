# RECEIPT — `cl-receipt-cycle-id-authority`

**Bloque:** cierre de INC-DEBT-063 — ningún recibo declara un ciclo que la autoridad no tiene
**Ciclo SDDK:** ninguno. Este bloque es de roadmap, no un ciclo del ledger; y su
propio encabezado lo declara, que es justamente lo que ahora exige el guard.
**Cerrado:** 2026-10-06
**Workspace:** 2.12.1 (declarada, no publicada; último tag remoto `v2.12.0`)

---

## §1 — Qué se cambió

- **13 documentos de resultado** dejan de afirmar un ciclo inexistente:
  11 con un `cycle_id` completo que el ledger no tiene, y 2 cuyo campo
  `**Cycle:**` contenía un hito del roadmap (`C3h`, `C4`) en vez de un ciclo.
- **Guard nuevo** `tests/test_receipt_cycle_id_authority.sh`: lee el ledger real y
  falla si cualquier `RECEIPT*.md` / `CLOSURE.md` declara un `cycle_id` con 0
  filas, y si un campo `**Cycle:**` no trae id de forma completa.
- **Autofalsación** `tests/test_receipt_cycle_id_authority_mutation.sh`.
- Deuda madre a `resolved` con las cifras reales, y deuda de seguimiento nueva
  para lo que queda fuera de alcance.

## §2 — Medición

Antes (sobre el árbol original) y después, con el mismo guard:

```
antes    PASS=6   FAIL=7
después  PASS=20  FAIL=0   SKIP=0

autofalsación   PASS=4 FAIL=0 SKIP=0
  M1  un cycle_id fabricado sobrevive en el campo nuevo  → cae
  M2  el campo **Cycle:** sin cycle_id                   → cae
  M3  autoridad ausente                                  → cae (fail-closed)
  M4  comentario inocuo                                 → no cambia
shellcheck  limpio en ambos
```

Las 20 afirmaciones del veredicto verde son 11 ids que **sí** existen (comprobados
uno a uno contra `cycles` en el ledger real) y 9 declaraciones explícitas de «no
hay ciclo».

## §3 — Lo que este bloque NO cierra

1. **Los documentos de intención** (`PRE-FLIGHT.md`, `SCOPE-CONTRACT.md`) siguen
   declarando ciclos inexistentes: **27 ficheros** y **31** afirmaciones
   `**Cycle:** C3e` sin id. Es una decisión de alcance y está escrita en la
   cabecera del guard con los números, para que el hueco sea visible y no
   heredado. Seguimiento:
   [`INC-DEBT-063-FU-PLANNING-DOCUMENTS.md`](../../../debt/INC-DEBT-063-FU-PLANNING-DOCUMENTS.md).
2. **No se creó ningún ciclo.** Fue la línea que esta deuda trazaba: enmendar el
   encabezado, no escribir historia en la autoridad. Los bloques no se mapearon a
   los dos ciclos `vault-*` reales porque ese linaje no está medido y asignarlo
   sería inventarlo.
3. **No corrige el trabajo que esos recibos describen.** Solo que su afirmación de
   ciclo sea comprobable. Que el trabajo estuviera bien sigue sin verificar.

## §4 — Tres defectos que encontró el propio guard, y por eso el guard es el que es

**Falló en su primera ejecución, por dos causas propias.** `STATE_BASE` no estaba
exportada al probe, y los ids se leían de `stdin` cuando el heredoc del programa
ya ocupaba `stdin` — es decir, el guard se comprobaba a sí mismo. Falló cerrado,
que era lo correcto, pero por su propio bug. Es la forma más barata de la que
este repo avisa: un gate que se pone rojo antes de tener nada que decir.

**Se quedó ciego con el arreglo que él mismo había provocado.** Al corregir los
encabezados, el campo `**Cycle:**` pasó a llamarse `**Ciclo SDDK:**`… y la
primera versión del guard solo leía `**Cycle:**`. Un guard que vigila el nombre
viejo del campo queda verde justo porque se corrigió lo que vigilaba: es
INC-AUDIT-S14-DEFAULT-GATE-DISCONNECTED, cometido por el guard que lo evita. Por
eso ahora lee **todo campo que empiece por «Ciclo» o «Cycle»**, y M1 existe
precisamente para que ese hueco no vuelva.

**Se ensanchó hasta meterse con la prosa y por eso se étroitó con regla.** Al
ampliar el alcance a todo el directorio, el patrón `**[^*]*[Cc]iclo[^*]*:**`
atrapó tres líneas de prosa en negrita que *hablaban* de un ciclo
(`**Conclusión, y es el resultado del ciclo:**`). Un guard que se ensancha hasta
producir falsos positivos acaba callándose, que es la forma que tiene un guard de
seguir verde sin vigilar nada. La regla final exige que el campo **empiece** por
«Ciclo» o «Cycle».

Y el recorte de alcance —de todo el directorio a los documentos de resultado— está
escrito en la cabecera con sus cifras, porque un guard al que se le recorta el
alcance para no tener que lidiar con lo que mide es el mismo defecto por el otro
lado. Si mañana se hace el sweep, el guard se amplía; y por eso el recorte está
declarado y no ejecutado en silencio.

## §5 — Contar mal, dos veces

La deuda decía **tres** recibos. La medición dio **13** en los documentos de
resultado. Dos de los errores fueron míos y ninguno lo detectó la lectura del
documento:

1. Un extractor `sed` con `.*` codicioso capturó en un solo token los tres
   `cycle_id` de `cl-doctor-build-identity` y lo marcó como inexistente. **Sí**
   existe. El guard no hereda el error porque extrae token a token.
2. Conté seis donde había cinco, porque el mismo bug de captura iba al revés en
   la primera pasada.

Y un detalle de la propia deuda: sus `references:` incluían
`cl-ledger-watch-total`, cuyo `cycle_id` **sí** existe. Una referencia que
sostiene la prueba de la deuda y no la sostiene.
