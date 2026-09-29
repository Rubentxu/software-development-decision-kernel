# C0-RECEIPT — Baseline reproducible re-anclado (T01/T02)

> **Slice id:** `p-63676b11dc0ef88f/c0-reanchor-at-2.2.21`
> **Baseline SHA:** `28ea2910` (workspace 2.2.21, == origin/main al emitir)
> **Date (UTC):** 2026-09-29T12:54:00Z
> **Status:** VERIFIED (observado hoy, en este SHA)

## T01 — HEAD, versión Cargo, tag y release comprobados independientemente

Observado 2026-09-29 (todos los comandos ejecutados en esta sesión):

| Verificación | Valor observado |
| --- | --- |
| HEAD local | `28ea2910` (docs-only sobre el release) |
| origin/main | `28ea2910` (push confirmado) |
| tag v2.2.21^{commit} | `11a166f15ce60bcad1b92e83703778e6234cf6a0` |
| GH Release v2.2.21 | 27 assets, isDraft=false, isPrerelease=false, run `36569406144` success, sha del run == tag |
| Cargo.toml workspace | `2.2.21` |
| manifest.toml / Cargo.lock | `2.2.21` alineados (guard del puntero lo verifica) |
| cosign binary | Verified OK, identidad `release.yml@refs/tags/v2.2.21` |
| digest instalado vs publicado | `a1288be8…` idéntico bit a bit |
| install | `all_present: true`, `current -> framework/2.2.21` |

**Falsaciones del guard de puntero observadas hoy (el guard en acción, no
invocado a propósito):**

1. `12:37:17` — STATE apuntaba a un SHA NO publicado aún →
   `[FAIL] current_sha=8db72dbb NO esta en origin/main`.
2. `12:47:46` — STATE iba 4 commits por detrás (tolerancia 3) →
   `[FAIL] el puntero va 4 commit(s) por DETRAS de main`.
3. Tras cada reconciliación → `RESULT: PASS` (última: 12:53:49).

El guard de puntero se dispara en cada ronda de suite (22/22 x2 hoy) y
en cada verificación manual: el código dev y el artefacto publicado se
distinguen porque el guard compara SHA exactos, no "parece actualizado".

## T02 — Mapeo requisito→commit→test→receipt

Cadena verificada para C1 (recibo `c1/89f60190/UAT-EVIDENCE.yaml`):
H01/H02 → commits SAW en `structured_work.rs` → 18 tests → recibo;
H05 → `89f60190` → 1 rust + 2 checks binarios → recibo;
H06 → 26 tests gateway → recibo. Evidencia ausente se marca NOT_RUN
(patrón del test de coherencia, session-33b), nunca PASS.

Para C2/C3 los recibos históricos (`c2a`, `c2b`, `c2c`, `c3a`–`c3h`)
existen pero quedan anclados a sus SHAs del 21-22/09: se necesita
re-anclaje propio antes de declararlos vigentes (siguiente trabajo).

## Estado y riesgos

- El baseline del roadmap (fotografía 21/09) queda sustituido como
  "última fotografía coherente" por esta: release v2.2.21 verificado por
  cinco vías + suite 22/22 x2 + punteros PASS.
- Deuda de re-anclaje pendiente: C2a/C2b/C2c y C3a–C3h (recibos
  históricos). No bloquean: son trazabilidad, no estado operativo.
