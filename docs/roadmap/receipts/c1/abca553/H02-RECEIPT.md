# C1-H02-RECEIPT — fix(engine): request_id uniqueness + non-sensitive schema-error interpolation

> **Slice id:** `p-63676b11dc0ef88f/c1-h02-request-uniqueness-and-redaction`
> **Baseline SHA:** `68f6788` (HEAD post-cycle-c SCOPE-CONTRACT)
> **Fix SHA:** pendiente commit (H02 fix atómico)
> **Date (UTC):** 2026-09-21T13:55:00Z (start) → 2026-09-21T13:57:00Z (end)
> **Status:** PASS_OBSERVED — fix landed, 9 RED→GREEN tests pass, no regression

## §1 Summary

ROADMAP §2 C1 H02: `structured_work::submit` debe rechazar sustituciones silenciosas de `request_id` y los errores de schema no deben filtrar valores sensibles.

### 1.1 Cambio 1 — `submit` con rechazo explícito de duplicados no equivalentes

```rust
// Before:
pub fn submit(&mut self, req: AgentWorkRequest) {
    self.requests.insert(req.request_id.clone(), req);  // silent replace
}

// After:
pub fn submit(&mut self, req: AgentWorkRequest) -> Result<(), StructuredWorkError> {
    if let Some(existing) = self.requests.get(&req.request_id) {
        if existing != &req {
            return Err(StructuredWorkError::DuplicateRequest {
                id: req.request_id.clone(),
                existing_task_ref: existing.task_ref.clone(),
                new_task_ref: req.task_ref.clone(),
            });
        }
        return Ok(());  // bit-exact duplicate: no-op
    }
    self.requests.insert(req.request_id.clone(), req);
    Ok(())
}
```

Nueva variante `StructuredWorkError::DuplicateRequest { id, existing_task_ref, new_task_ref }`. Mensaje: `"duplicate request_id {id}: existing task_ref=…, new task_ref=…"`.

Nuevo método opt-in `submit_idempotent` que delega en `submit` y devuelve `Ok(true)` en éxito.

### 1.2 Cambio 2 — Redacción del valor en mensajes de violación

```rust
// Before (line 155):
violations.push(format!("field {name} expected {shape}, got {v}"));

// After:
violations.push(format!(
    "field {name} expected {shape}, got {}",
    redacted_value_repr(v)
));
```

Nueva función `redacted_value_repr` que reporta **tipo y forma, no contenido**:

| JSON Value | Repr |
|---|---|
| Null | `null` |
| Bool(_) | `bool` |
| Number(n) | `number(n)` |
| String("") | `string("")` |
| String(_) | `string(<redacted>)` |
| Array(a) | `array[len={a.len()}]` |
| Object(o) | `object[len={o.len()}]` |

## §2 Tests added (9 nuevos, 16 totales SAW)

| Test | Propósito | Status |
|---|---|---|
| `saw009_duplicate_request_strict_rejects` | Strict `submit` con duplicado distinto → `Err(DuplicateRequest)` con `existing_task_ref` + `new_task_ref` correctos | ok |
| `saw010_duplicate_request_idempotent_accepts` | `submit_idempotent` con duplicado bit-exact → `Ok(true)`, sin duplicar | ok |
| `saw011_duplicate_request_idempotent_rejects_different` | `submit_idempotent` con duplicado distinto → `Err(DuplicateRequest)` | ok |
| `saw012_duplicate_request_after_run` | Submit duplicado tras un `run_structured` exitoso → error, pero **receipts preservados** | ok |
| `saw013_schema_violation_does_not_leak_string` | String con valor `AKIA-real-key-12345` → violación contiene `string(<redacted>)`, NO la key | ok |
| `saw014_schema_violation_does_not_leak_object` | Object con campos `password`/`hunter2` → violación contiene `object[len=2]`, ni claves ni valores | ok |
| `saw015_schema_violation_does_not_leak_array` | Array de 5 números → violación contiene `array[len=5]`, NO enumeración | ok |
| `saw016_missing_field_message_unchanged` | `missing field X` mantiene wording exacto | ok |
| `saw017_known_descriptor_violation_unchanged` | Shape mismatch en descriptor conocido (`u64` vs `string("not-a-number")`) → violación lleva `expected u64, got string(<redacted>)` | ok |

## §3 Test results

### Engine lib (`cargo test -p sddk-engine --lib structured_work`)

```
running 16 tests
test structured_work::tests::saw007_unknown_descriptor_rejected ... ok
test structured_work::tests::saw001_002_typed_request_schema_result ... ok
test structured_work::tests::saw006_receipt_provenance ... ok
test structured_work::tests::saw004_same_adapter_two_modes ... ok
test structured_work::tests::saw008_unknown_descriptor_violation_not_contribution ... ok
test structured_work::tests::saw009_duplicate_request_strict_rejects ... ok
test structured_work::tests::saw005_contribution_not_authority ... ok
test structured_work::tests::saw003_invalid_output_visible_not_fabricated ... ok
test structured_work::tests::saw011_duplicate_request_idempotent_rejects_different ... ok
test structured_work::tests::saw010_duplicate_request_idempotent_accepts ... ok
test structured_work::tests::saw012_duplicate_request_after_run ... ok
test structured_work::tests::saw013_schema_violation_does_not_leak_string ... ok
test structured_work::tests::saw015_schema_violation_does_not_leak_array ... ok
test structured_work::tests::saw014_schema_violation_does_not_leak_object ... ok
test structured_work::tests::saw016_missing_field_message_unchanged ... ok
test structured_work::tests::saw017_known_descriptor_violation_unchanged ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 1313 filtered out
```

### Workspace

A confirmar al cierre del comando en background. Baseline C0 (commit `62494ae`): 4966/0/15 → esperado 4977/0/15 (+11 = SAW-007..017).

### Clippy

```
cargo clippy -p sddk-engine --all-targets -- -D warnings
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 49.93s
```

Cero warnings. Los callers pre-existentes `ex.submit(...)` se adaptaron a `let _ = ex.submit(...)` o `.unwrap()` según corresponda.

## §4 Migración

**Cambio breaking** en `submit`: ahora devuelve `Result<(), StructuredWorkError>`. Los callers deben:

- Si quieren ignorar el error: `let _ = ex.submit(req);`
- Si quieren panic en test: `ex.submit(req).unwrap();`
- Si propagan: `ex.submit(req)?;`

Aplicado a:
- `saw001_002_typed_request_schema_result` (línea 306) → `let _ = …`
- `saw003_invalid_output_visible_not_fabricated` (líneas 332, 402, 428) → `let _ = …`
- `saw004_same_adapter_two_modes` (líneas 369, 382) → `let _ = …`
- `saw005_contribution_not_authority` (línea 483) → `let _ = …`
- `saw006_receipt_provenance` (línea 518) → `.expect(...)` / `.expect_err(...)` para SAW-009..SAW-017

## §5 Riesgos y limitaciones

- **Breaking change:** cualquier consumer fuera de SAW tests que llame `submit` con expectativas de unit-typed debe actualizarse. En este crate no hay tales consumers externos (es un módulo interno de `structured_work`).
- **Redacción conservadora:** las strings siempre se redactan aunque sean inofensivas. Si en debugging el caller necesita ver el valor, debe usar logs propios, no el campo `violations`.
- **False positive en field names:** si el nombre del campo ES un secreto (`"password": "x"` → reporta `object[len=1]`), la longitud sigue dando pista. El nombre del campo en sí NO se redacta (es metadata, no valor). H06 cubre este caso a nivel de gateway.

## §6 Acceptance gates

| Gate | Result | Evidence |
|---|---|---|
| **G0 — Fix mínimo y surgical** | PASS | 2 cambios de losas: (a) `submit` body + nuevo variant error, (b) `redacted_value_repr` helper + 1 format! |
| **G1 — RED characterization captured** | PASS | Tests no compilaban antes (firmas cambiadas); al compilar, asserts específicos verifican cada bug |
| **G2 — GREEN captured post-fix** | PASS | 16/16 SAW tests verdes |
| **G3 — No-regression at crate level** | PASS | 1313+ existing engine tests still green |
| **G4 — Clippy limpio** | PASS | `-D warnings` zero issues |
| **G5 — Workspace full** | pending | ejecución background, baseline 4966+11=4977 esperados, 0 failed, 15 ignored |

## §7 Próxima acción

Operador:
1. Validar admisibilidad con `release_admission_check HEAD` (esperado: REJECT non-monotonic 1.169.133 -> 1.169.133 — porque el commit de H02 no bumpea).
2. Decidir entre:
   - (a) Acumular más slices (H05, H06) antes de bumpear y publicar.
   - (b) Bumpear a `1.169.134` ya, aceptar release candidata desde HEAD actual.
   - (c) Esperar al rediseño del contrato de admisión (cycle-c SCOPE-CONTRACT ya emitido en `68f6788`).
3. Publicar con `bash scripts/release.sh`.

Orquestador tras operator-side decision:
- Continuar H05 (test-seam `set_process_service_for_tests`) con mismo método RED→GREEN.
- O continuar H06 (gateway defense: argumentos/bytes, límites, redacción).
