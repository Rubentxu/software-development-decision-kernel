# RECEIPT — session-53: C3l.2 Producer→L0 wiring real (AT-UAT-004/005)

- **Fecha (UTC):** 2026-10-01T00:29Z → 00:50Z
- **Baseline:** `origin/main` con v2.4.1 publicada (session-52)
- **Slice:** C3l.2 del paquete acceptance-truthfulness
- **Ledger del operador:** intacto (slice IN_PROCESS; pruebas del gateway con engines en memoria)

## Defecto (verificado en código antes de tocar)

`ProducerToL0Adapter::dispatch` (crates/sddk-gateway/src/producer_l0_adapter.rs:97) evaluaba contra `SecretaryL0Engine::new()` **fresco en cada llamada**: ninguna regla productiva registrada podía dispararse por la ruta pública. Defecto declarado en C3l.2; por eso `AIW-S7a` estaba `NOT_VERIFIED`.

**Hallazgo secundario (misma clase que INC-DEBT-043/044):** el test S7a existente **fijaba el defecto como esperado**:

```rust
assert!(signals.is_empty(),
    "fresh engine has no rules; dispatch must be side-effect free");
// y después "demostraba" G04 reconstruyendo el ReactiveEvent a mano
```

Es decir: el test pasaba porque el wiring NO existía, y su prueba de "la evidencia llega a L0" reconstruía a mano el evento — exactamente el patrón que C3l.2 prohíbe ("reconstruir manualmente un `ReactiveEvent` desde el test para demostrar la ruta" está prohibido).

## Resolución (opción 2 del paquete)

`ProducerToL0Adapter` compone `Option<Arc<SecretaryL0Engine>>`:
- `new()` / `with_now(ms)` conservan un engine fresco (compat; tests de silencio/race).
- **`with_engine(Arc<SecretaryL0Engine>, now_ms)`** — ruta pública de composición.
- `dispatch` evalúa contra el engine inyectado; el cooldown queda en el engine persistente (diseño pretendido, antes imposible de observar).

Restricciones del paquete respetadas: sin authority dentro del adapter; sin reglas CogniCode/Chronos hardcodeadas en el motor (las registra el caller); `Unknown` sigue mudo (G06); determinismo y cooldown preservados.

## Evidencia

| Qué | Resultado |
|---|---|
| RED antes del fix | los tests S7a no compilaban: `with_engine` inexistente (el wiring público era lo que faltaba) |
| GREEN después | `aiw_s7a_producer_l0` **5/5** (cognicode y chronos-crash disparan por `dispatch`; race y Unknown siguen mudos) |
| Exit gate de C3l.2 | test propio `exit_gate_fresh_engine_cannot_fire_registered_rules`: engine vacío inyectado ⇒ 0 señales aunque exista regla registrada en otro engine |
| Suite gateway | **133/0** |
| Perfil completo | `cargo test --workspace` **5195 passed / 0 failed / 19 ignored** (+1 exacto: el exit-gate), TEST_EXIT=0 |
| fmt / clippy | limpios (`-p sddk-gateway -p sddk-engine --all-targets -D warnings`) |

## Matriz / trazabilidad

- **AIW-S7a**: `NOT_VERIFIED` → **IMPLEMENTED, re-verificable** (el estado definitivo lo fija C3n.2 re-ejecutando el suite S7a sobre el SHA nuevo).
- **AT-UAT-004/005**: PASS en UAT-MATRIX.

## Límites declarados

- Los tests `chronos_race_e2e` y `unknown_event_e2e` siguen usando el constructor con engine fresco: verifican silencio, no disparo, que es lo que deben verificar.
- El estado de cooldown ahora persiste entre dispatches del mismo adapter; no hay test dedicado que lo fije en esta slice (Queda para C3l.4/C3n si el paquete lo exige).
- Sin consumidores de `ProducerToL0Adapter` fuera del propio test S7a (verificado por grep): el cambio no altera ninguna ruta de producción todavía — el adapter queda listo para composición por el application layer, que es la siguiente capa.

## Commits

- `5e56b4c4` fix(gateway): adapter evalúa contra el engine inyectado
- `790ad830` docs(c3l): C3l.2 cerrada, matriz y AT-UAT actualizadas
- `98cdd2f4` chore(release): bump 2.4.2

## ADDENDUM — release v2.4.2 PUBLICADA (misma sesión)

- **SemVer derivado:** fix → PATCH → **2.4.2**. Perfil completo antes del commit: **5195/0/19** (+1 = exit-gate).
- **Flujo local** 0–8c: preflight ACCEPT 2.4.1→2.4.2, HEAD pusheado, binario y artefactos construidos, parada en 8c **por diseño** (INC-DEBT-024). Publicación por **CI**: run **36782347136 completed success** (13/13 jobs, firma cosign + smoke E2E).
- **Tag:** anotado, objeto `aea49ba8`, peel `98cdd2f4` == origin/main (push → sync → tag → CI).
- **Release:** isDraft=false, isPrerelease=false, publishedAt 2026-10-01T00:02:10Z, **27 assets**.
- **9b OBSERVED:** 27/27 HTTP 200; `test_release_public_gate.sh` PASS=13 FAIL=0; anclaje verificado.
- **9c OBSERVED:** sha CDN `ed1c4e5f64d3f3ba…` == declarado; cosign **Verified OK** identity `release.yml@refs/tags/v2.4.2`.
- **10–12 OBSERVED:** install exit 0; `sddk 2.4.2`; current → 2.4.2; doctor **all_present: true**; prune removed 2.4.1 kept 2.4.2.
