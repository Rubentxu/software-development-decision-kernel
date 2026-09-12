# Session close — 2026-09-12 (turno 3: ARCH-LINT-AX-S5 promotion) → próxima sesión

> **Sesión cerrada por:** usuario (modo auto / fin de turno)
> **Estado final del repo:** `main` en `d94b10a` (HEAD); tag `v1.168.29` peels al mismo SHA
> **Working tree:** limpio
> **Próxima sesión:** arrancar con `git status --short` (espera `clean`) + `sddk --version` (espera `sddk 1.168.29`) + `sddk dev doctor` (espera `all_present: true`) + `sddk dev lint deprecated-patterns --enforce` (espera exit 0)

---

## 1. Lo que se cerró en este turno

Este turno aplicó el ciclo **ARCH-LINT-AX-S5 promotion** — el survey de los 4 lints `asset_*` advisory (promovidos desde AX-S5 en v1.168.18) para evaluar promoción a `default: deny`. Tres de los cuatro pasan las 3 condiciones de aceptación documentadas y se promueven; el cuarto (`asset_unregistered_cli_example`) se queda en `allow` por unsafe-by-design regex.

### Cycle summary

| Hito | Cycle id | Resultado |
|---|---|---|
| (prior turno 2) | `ax-s3-depth-2-followup` | v1.168.28 — AX-S3 depth-2 derivador puro |
| **ARCH-LINT-AX-S5 (este turno)** | `arch-lint-ax-s5-promotion` | **v1.168.29** — 3 of 4 AX-S5 lints a `default: deny`; 1 stays allow con rationale |

### Commits de este turno

```
d94b10a chore(release): bump version to v1.168.29
cd9b6a3 feat(lint): promote 3 of 4 AX-S5 asset_* lints to default: deny (ARCH-LINT-AX-S5)
20c497a chore(release): bump version to v1.168.28 + AX-S3 depth-2 archive handoff  ← turno 2
```

### Evidencia global

- `cargo test -p sddk-cli --lib` → 695/695 PASS (incl. renamed pin `live_registry_asset_lints_are_promoted_or_advisory_and_clean`)
- `cargo fmt --check` → clean
- `cargo clippy -p sddk-cli --lib --no-deps -- -D warnings` → clean
- `sddk dev lint deprecated-patterns --enforce` → exit 0; 9 lints totales; 4 deny + 0 hits
- `sddk dev doctor` → `all_present: true` (338 checks)
- GH Release v1.168.29 → published; 9 assets live en el CDN
- 1 release en este turno: v1.168.29 (vía `scripts/release.sh --skip-tests` + manual `git tag -f v1.168.29 HEAD && git push origin main v1.168.29 --force`)
- 1 archive manifest escrito a `~/.sddk-knowledge/sddk-framework/cycles/arch-lint-ax-s5-promotion/`

### Estado del lint registry tras este turno

| Lint | Default | Hits | Estado |
|---|---|---|---|
| `agent_result_used` | deny | 0 | (v1.168.12 promoted) |
| `evidence_kind_v1` | allow | 22 | ADR-0100 gated |
| `orchestration_synthesis_no_dissent` | allow | 19 | ADR-0101 gated |
| `execution_outcome_as_synthesis` | allow | 0 | ADR-0101 gated |
| `transition_outcome_used` | allow | 24 | ADR-0101+0096 gated |
| `asset_deprecated_namespace` | **deny** | 0 | **v1.168.29 promoted** |
| `asset_raw_store_reference` | **deny** | 0 | **v1.168.29 promoted** |
| `asset_authority_language` | **deny** | 0 | **v1.168.29 promoted** |
| `asset_unregistered_cli_example` | allow | 0 | Unsafe-by-design regex; AX-S1 pin responsable |

---

## 2. Convenciones reforzadas

- **Honest disclosure over silent promotion.** El lint runner se ejecutó contra el workspace live antes de promover; 0 hits confirmados para los 3 candidatos. Si un futuro lint tiene hits no triviales, NO se promueve — la audit trail en el toml lo explica.
- **Per-lint rationale, not blanket decisions.** `asset_unregistered_cli_example` se queda en allow no por "todavía no", sino porque su regex es unsafe-by-design (first-letter sieve matchearía comandos válidos). Promoverlo garantizaría falsos positivos.
- **Pin tests con nombre descriptivo.** El test renombrado `live_registry_asset_lints_are_promoted_or_advisory_and_clean` codifica el invariante futuro: 3 deny + 0 hits + 1 allow + 0 hits. Si alguien reintroduce el legacy namespace en un asset, el test falla con mensaje claro.
- **`enforcement_status` field del toml** debe reflejar la verdad: tras este turno dice "agent_result_used: deny; asset_*: deny; remaining: advisory" — un consumer externo ve el estado de un vistazo.

---

## 3. Próxima sesión

### Quick resume commands

```bash
cd ~/Proyectos/agentesIA/sddk-framework
git status --short                            # espera: clean
git log --oneline -5                          # espera: HEAD = d94b10a (chore(release): bump version to v1.168.29)
sddk --version                                # espera: sddk 1.168.29
sddk dev doctor | grep all_present            # espera: all_present: true
sddk dev lint deprecated-patterns --enforce   # espera: exit 0
```

### Próximo ciclo recomendado

Único hilo restante del handoff previo (`HANDOFF-2026-09-12-ax-s3-depth-2.md`):

1. **Promote `InstructionsRenderer`** (AX-S4 spike v1.168.17) a first-class — solo si surge un segundo provider real. Hoy solo el adapter sectioned/flat cubre el corpus. **No trigger presente, no work to do en este turno.**

Si el usuario quiere algo accionable inmediato, hay dos micro-opciones de housekeeping:

2. **Update M9 row en `docs/architecture/README.md`** para reflejar el promotion state actual (5 of 9 lints a deny; 4 restantes gated on ADR-0100/0101/0096 acceptance + 1 unsafe-by-design). Documentación-only, scope: 1 commit.
3. **Survey `asset_unregistered_cli_example`** — evaluar si una regex con `git grep` lookup table (regex expandida a las letras de comandos reales: `cdfghi...`) reduciría el false-positive risk. Si sí, promover también esta última. Si no, mantener el rationale actual.

Trade-off: el #2 es strictly cheaper y honest; el #3 podría cerrar el último hilo pero requiere experimentación.

### Open threads (no work yet, just awareness)

- Debt ledger a v1.168.29: **29 closed + 0 open + 2 resolved** (sin cambios desde v1.168.27; este ciclo no tocó debt).
- AGENTS.md §2.6 prohíbe expandir test scope para enmascarar incertidumbre — mantener en cada `apply`.
- AGENTS.md §2.5 cloud CI está agotado en el plan free de GitHub; confiar en gates locales.
- El helper `scripts/release-receipt.sh` debe permanecer en lockstep con `crates/sddk-engine/src/authority.rs::infer_actor_kind`. Si la engine side cambia el prefix heuristic, este script debe cambiar también (documentado en ADR-069 §5 como contrato bloqueado).
- `asset_unregistered_cli_example` se queda advisory; si en el futuro el corpus crece y la sieve regex matchea demasiado, hay que considerar AST-level detection (cycle documentado en `acceptance_for_m9_blocking_enforcement`).

---

## 4. Archivos importantes para referencia

- `docs/architecture/lints/deprecated_patterns.toml` — 3 lints a deny; `enforcement_status` actualizado; audit comment al final
- `crates/sddk-cli/src/dev/lint/deprecated_patterns.rs` — pin test renombrado y reestructurado
- `~/.sddk-knowledge/sddk-framework/cycles/arch-lint-ax-s5-promotion/archive-manifest.md` — archive completo

---

## 5. Trunk actual (HEAD = d94b10a)

```
d94b10a chore(release): bump version to v1.168.29
cd9b6a3 feat(lint): promote 3 of 4 AX-S5 asset_* lints to default: deny (ARCH-LINT-AX-S5)
20c497a chore(release): bump version to v1.168.28 + AX-S3 depth-2 archive handoff
62ee181 chore(release): bump version to v1.168.28
71f93f5 feat(spec): derive related-command depth-2 from depth-1 table (AX-S3 D2)
```

---

## 6. Resumen en una línea

> Este turno aplicó **ARCH-LINT-AX-S5 promotion** (survey de los 4 lints `asset_*` advisory): 3 promovidos a `default: deny` (asset_deprecated_namespace, asset_raw_store_reference, asset_authority_language) tras confirmar 0 hits live y pasar las 3 acceptance criteria; `asset_unregistered_cli_example` queda advisory por regex unsafe-by-design. Pin test renombrado (`_are_promoted_or_advisory_and_clean`). Release v1.168.29 shippeado + installed + pruned, doctor `all_present: true`, lint `--enforce` exit 0. Próxima sesión: promote `InstructionsRenderer` (gated a 2º provider real) o update M9 row en `docs/architecture/README.md` (housekeeping doc).