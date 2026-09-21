# DELTA-CONF-003 — Universal Evidence Cutover

> Delta spec ejecutable de [SPEC-CONF-003-UNIVERSAL-EVIDENCE-CUTOVER.md](../SPEC-CONF-003-UNIVERSAL-EVIDENCE-CUTOVER.md).
> Ciclo: `p-63676b11dc0ef88f/conformance-closeout-2026-09-13` (fase specify).
> Cada Scenario es criterio de aceptación directamente ejecutable (dado/cuando/entonces).

## 1. Estado inicial (evidencia C0)

Estado medido contra `main` el 2026-09-13. Fuentes:

- `reference/09-09-SPEC-CROSSWALK.md:35` (M9 "remove planning Evidence duplicates": `PlanningEvidenceKind` migración diferida) y `:7` (SPEC-001 FAIL/PARTIAL, evidence cutover pendiente).
- `crates/sddk-domain/src/planning/mod.rs:278-306` — `PlanningEvidenceKind` vivo con 5 variantes (`Log`, `Metric`, `Snapshot`, `Reference`, `Approval`), claramente paralelo a `evidence::EvidenceKind`; test `evidence_kind_is_five_variants` (mismo fichero, macro `assert_variant_count_eq!`).
- `docs/architecture/lints/deprecated_patterns.toml:26` — `evidence_kind_v1` es **advisory** con 57 hits; `:72` definición; `:294` — "REMAIN `allow` (advisory). UNBLOCK = future evidence-migration-v2". `:276` — per-call-site migration deferred a evidence-migration-v2.
- `crates/sddk-engine/src/evidence_relation_mapping.rs:133` — `assert_planning_evidence_kind_mapping_total` (mapping de 5 variantes a relaciones universales ya existe y es total); `:29` — ejemplo `Metric -> Verifies`.
- `crates/sddk-engine/src/evidence_ref.rs` y `crates/sddk-storage/src/spine_import.rs` — `EvidenceAttachmentV1` presente en dominio/storage (migración a `EvidenceRef` + relación semántica pendiente).
- `crates/sddk-domain/tests/gate_receipt_pass_evidence.rs:24` — `passed_with_all_three_fields_is_accepted` valida evidencia de gates, no universalidad.
- `uat/CONFORMANCE-FITNESS-RATCHETS.md:11-12` — `conf09_universal_evidence_only` MISSING; `conf09_no_planning_evidence_new_writes` MISSING (parcial, advisory no deny, sin mutation test).

**Diagnóstico C0:** la estructura de migración existe (mapping total + lint + `EvidenceRef`), pero `PlanningEvidenceKind` sigue siendo autoridad de evidencia de planificación en producción, el lint es advisory y la migración está formalmente diferida a `evidence-migration-v2`. Incompatible con declarar M9 terminado.

## 2. Estado final verificable

1. `Evidence`/`EvidenceRef` universal es el único substrate de evidencia del core; ningún workflow (planning, ejecución, governance, contribuciones de agente, verification-like) define una segunda jerarquía de autoridad de evidencia.
2. No existe construcción de `PlanningEvidenceKind` (ni tipo equivalente de autoridad planning-only) fuera de módulos de migración/decoder read-only, demostrado por ratchet `conf09_no_planning_evidence_new_writes` promovido a **deny** con mutation test.
3. El mapping `PlanningEvidenceKind -> {supports, verifies, gates, produced_by, contradicts}` permanece total y sin pérdida (`assert_planning_evidence_kind_mapping_total` no se rompe).
4. Los bytes/referencias de fuente históricas se preservan donde el replay exacto importa (compat decoding solo en boundaries de lectura).
5. `EvidenceAttachmentV1` migrado a `EvidenceRef` + relación semántica/fact según baseline; decoder read-only documentado si persiste.
6. `evidence-migration-v2` deja de existir como "deuda futura": o se ejecuta dentro de este closeout, o se demuestra formalmente que el símbolo es inalcanzable en producción.
7. Cualquier path de compatibilidad restante cumple las 6 condiciones de SPEC-CONF-001 §Compatibility exception (`specs/SPEC-CONF-001-BASELINE-CONFORMANCE-CONTRACT.md:59-68`); el allowlist solo contiene locations explícitas de decoder/migración con los 8 campos de `CONFORMANCE-FITNESS-RATCHETS.md:29-40`.
8. ARCH-SC-008 (una sola autoridad de evidencia) es **blocking**, no advisory.

## 3. Tests de contención que NO se pueden romper

| Test | Localización | Qué fija |
|---|---|---|
| `assert_planning_evidence_kind_mapping_total` | `crates/sddk-engine/src/evidence_relation_mapping.rs:133` | mapping de las 5 variantes a relaciones universales es total |
| `passed_with_all_three_fields_is_accepted` | `crates/sddk-domain/tests/gate_receipt_pass_evidence.rs:24` | evidencia de gate receipt (UAT-04) sigue aceptada |
| `evidence_kind_is_five_variants` | `crates/sddk-domain/src/planning/mod.rs` (macro `assert_variant_count_eq!`, tras `:306`) | partición del enum durante la ventana de migración; al remover variantes este test se actualiza en el mismo commit con receipt |
| `evidence_kind_v1` lint registry | `docs/architecture/lints/deprecated_patterns.toml:72`; enforcement `crates/sddk-cli/src/dev/lint/deprecated_patterns.rs` (`live_registry_asset_lints_are_promoted_or_advisory_and_clean`) | el registry vivo y el lint siguen cargando; su promoción advisory→deny no puede dejar el registry en estado inconsistente |
| `command_spec_tests::clap_surface_and_command_specs_are_in_sync` | `crates/sddk-cli/tests/command_spec_tests.rs:187` | si el cutover añade flags/subcomandos de migración |
| `cli_golden::cli_golden_surface_matches_blessed_snapshot` | `crates/sddk-cli/tests/cli_golden.rs:71` | snapshot CLI |
| UAT-04, UAT-05, UAT-11, UAT-19 | `uat/UAT-09-09-CONFORMANCE-MASTER.md:10,11,17,25` | acceptance explícita de SPEC-CONF-003: deben permanecer green |

## 4. Ratchets activados y scenarios

### 4.1 Ratchet `conf09_universal_evidence_only`

**R-003.1** — Único substrate, demostrado por inyección.

```gherkin
Scenario: segunda jerarquía de evidencia es rechazada (mutation test)
  Dado el ratchet conf09_universal_evidence_only implementado como test de arquitectura/scan
    Y Evidence/EvidenceRef universal como substrate canónico del core
  Cuando se inyecta la mutación "definir un nuevo enum de autoridad de evidencia
    paralelo en un módulo de producción (p.ej. re-exportar PlanningEvidenceKind como raíz nueva)" en el fixture
  Entonces el ratchet falla nombrando el símbolo duplicado y su módulo
    Y el ratchet pasa sobre el árbol sin mutar
    Y la mutación queda registrada en el estilo de CONFORMANCE-FITNESS-RATCHETS.md §Mutation tests (:49)
```

**R-003.2** — Semántica de relaciones universales presente y tipada.

```gherkin
Scenario: las cinco relaciones semánticas obligatorias existen sobre el substrate universal
  Dado el substrate universal Evidence/EvidenceRef
  Cuando se inspecciona el modelo de relaciones expuesto a los workflows
  Entonces existen constructores tipados para al menos supports, verifies, gates, produced_by y contradicts
    Y cada variante PlanningEvidenceKind mapea sin pérdida a una de ellas
      (contrato cubierto hoy por evidence_relation_mapping.rs:133 assert_planning_evidence_kind_mapping_total)
    Y domain-specific code solo define builders tipados, nunca raíces de evidencia paralelas
```

**R-003.3** — Equivalencia de outputs tras migración.

```gherkin
Scenario: evidencia migrada produce outputs planning/WHY equivalentes
  Dado un repo con evidencia planning persistida bajo el formato legacy
  Cuando se ejecuta la migración a EvidenceRef + relación semántica
  Entonces los outputs de planning y WHY generados antes y después de la migración son equivalentes (fixture de parity)
    Y UAT-04, UAT-05, UAT-11 y UAT-19 permanecen green
```

### 4.2 Ratchet `conf09_no_planning_evidence_new_writes`

**R-003.4** — Construcción en producción rechazada.

```gherkin
Scenario: construcción de PlanningEvidenceKind en módulo de producción falla (mutation test)
  Dado el lint evidence_kind_v1 promovido de advisory a deny
    (hoy: docs/architecture/lints/deprecated_patterns.toml:26,72 — advisory, 57 hits)
  Cuando se inyecta la mutación "construir PlanningEvidenceKind en un módulo de producción
    fuera de migration/read-compat" en el fixture CLOSE-02
    (uat/UAT-09-09-CONFORMANCE-MASTER.md:37 "architecture/compile guard fails")
  Entonces el build/lint falla nombrando el call-site
    Y el allowlist de excepciones contiene solo locations explícitas de decoder/migración
      con los 8 campos de CONFORMANCE-FITNESS-RATCHETS.md §Allowlist policy
    Y el mismo guard pasa sobre el árbol sin mutar
```

**R-003.5** — Cero escrituras nuevas, ventana strangler.

```gherkin
Scenario: no hay escrituras nuevas de evidencia planning durante la ventana de migración
  Dado la ventana strangler abierta (existing data legible vía decoder, nuevos writes prohibidos)
  Cuando cualquier workflow de producción persiste evidencia nueva
  Entonces la escritura usa el substrate universal (Evidence/EvidenceRef + relación semántica)
    Y un scan de dependencia estática verifica que ningún writer nuevo referencia los tipos legacy
    Y los hits del lint evidence_kind_v1 solo pueden decrecer (ratchet monotónico: cada release no aumenta el contador auditado en deprecated_patterns.toml:276)
```

**R-003.6** — Cierre de evidence-migration-v2 como deuda.

```gherkin
Scenario: evidence-migration-v2 deja de ser deuda futura
  Dado el estado C0 donde la migración per-call-site está diferida a evidence-migration-v2
    (docs/architecture/lints/deprecated_patterns.toml:26,276,294)
  Cuando el closeout de esta fase termina
  Entonces o bien la migración se ejecutó (0 hits, lint eliminado o archive-only)
    O existe prueba formal ejecutable de que el símbolo es inalcanzable en producción
      (scan + test de dependencia citados en el receipt del ciclo)
    Y el UNBLOCK declarado en deprecated_patterns.toml:294 queda resuelto o su entry actualizado con receipt
```

## 5. Restricciones de migración

- **Strangler, no flag-day:** promocionar el lint a deny puede requerir splits del allowlist por call-site; cada wave de migración baja el contador y actualiza `deprecated_patterns.toml` en el mismo commit, sin romper `assert_planning_evidence_kind_mapping_total`.
- **Read-compat solo bajo SPEC-CONF-001 §Compatibility exception** (6 condiciones); decoders viven únicamente en boundaries de lectura.
- **Preservación de bytes:** donde el replay exacto importa, la migración preserva bytes/source refs (SPEC-CONF-003 Requirements).
- Los ratchets promoted a blocking solo cuentan como PASS cuando su mutation test existe y corre en CI (`CONFORMANCE-FITNESS-RATCHETS.md:60`).
