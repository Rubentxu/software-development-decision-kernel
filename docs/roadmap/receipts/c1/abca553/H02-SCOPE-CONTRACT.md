# C1 H02 — SCOPE-CONTRACT — request_id uniqueness + non-sensitive schema-error interpolation

> **Slice id:** `p-63676b11dc0ef88f/c1-h02-request-uniqueness-and-redaction`
> **Baseline SHA:** `68f6788` (HEAD post-cycle-c SCOPE-CONTRACT)
> **Status:** DESIGN — pendiente RED→GREEN
> **Surface:** `crates/sddk-engine/src/structured_work.rs` (submit + run_structured)

## §1 Surface investigation (verbatim del código actual)

### 1.1 `submit` — bug de unicidad silenciosa

```rust
// Línea 124-126
pub fn submit(&mut self, req: AgentWorkRequest) {
    self.requests.insert(req.request_id.clone(), req);
}
```

**Problema:** `BTreeMap::insert` con key existente **reemplaza silenciosamente** la entrada previa. Esto significa:

```text
let mut ex = StructuredWorkExecutor::new();
ex.submit(AgentWorkRequest {
    request_id: "req-1".into(),
    task_ref: "task:original".into(),
    return_schema: ReturnSchema { fields: BTreeMap::from([("a".into(), "string".into())]) },
    ..rest_default
});
// Pasan 5 minutos. Otro caller registra otra petición con el mismo id
// pero contenido distinto.
ex.submit(AgentWorkRequest {
    request_id: "req-1".into(),
    task_ref: "task:WRONG".into(),
    return_schema: ReturnSchema { fields: BTreeMap::from([("completely-different".into(), "u64".into())]) },
    ..rest_default
});
// La petición original se ha perdido sin error. run_structured("req-1", ...)
// ejecutará la SEGUNDA contra cualquier output que llegue, sin warning.
```

Esto viola H02 ROADMAP: "`request_id` duplicado no sustituye silenciosamente una petición no equivalente".

### 1.2 Errores de schema — interpolación sensible

```rust
// Línea 152 y 155
violations.push(format!("missing field {name}"))
violations.push(format!("field {name} expected {shape}, got {v}"))
```

`{v}` es `serde_json::Value` serializado con Display. Si `v` contiene:
- Tokens, passwords, keys (en tests con datos sintéticos).
- PII en campos que el host decidió llamar `summary` o `confidence` pero contienen secretos por error de mapping.

→ El `Vec<String>` de violaciones entra al `StructuredRunOutcome::SchemaViolation(violations)`, que a su vez viaja a logs, receipts, UAT reports, y posibles herramientas downstream.

H02 ROADMAP: "errores de schema **nunca** interpolan valores sensibles".

## §2 Diseño propuesto

### 2.1 Cambio de contrato en `submit`

**Decisión:** distinguir dos modos:
- **Modo estricto (default):** si `request_id` ya existe y el contenido difiere → **error visible** (`StructuredWorkError::DuplicateRequest { existing_task_ref, new_task_ref }`).
- **Modo idempotente (opt-in):** si `request_id` ya existe y el contenido es **idéntico** (`PartialEq` sobre `AgentWorkRequest`) → no-op silencioso.

Justificación del modo idempotente: en SAW-004 el mismo executor se usa en companion y orchestrated; algunos harnesses re-registran la misma petición sin querer. Idempotencia cuando es bit-exact protege eso.

**API concreta:**

```rust
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StructuredWorkError {
    #[error("unknown request {0}")]
    UnknownRequest(String),
    #[error(
        "duplicate request_id {id}: existing task_ref={existing_task_ref}, \
         new task_ref={new_task_ref} (use a fresh request_id or submit_idempotent)"
    )]
    DuplicateRequest {
        id: String,
        existing_task_ref: String,
        new_task_ref: String,
    },
}

impl StructuredWorkExecutor {
    pub fn submit(&mut self, req: AgentWorkRequest) -> Result<(), StructuredWorkError> {
        self.submit_with_mode(req, SubmitMode::Strict)
    }

    /// Idempotent submit: if a request with the same id exists and is
    /// structurally identical, do nothing. Otherwise return DuplicateRequest.
    pub fn submit_idempotent(&mut self, req: AgentWorkRequest) -> Result<bool, StructuredWorkError> {
        // returns true if accepted (new or identical), false if duplicate
        // (existing differs)
        if let Some(existing) = self.requests.get(&req.request_id) {
            if existing == &req {
                return Ok(true);  // no-op, structurally identical
            }
            return Err(StructuredWorkError::DuplicateRequest {
                id: req.request_id.clone(),
                existing_task_ref: existing.task_ref.clone(),
                new_task_ref: req.task_ref.clone(),
            });
        }
        self.requests.insert(req.request_id.clone(), req);
        Ok(true)
    }
}

enum SubmitMode { Strict, Idempotent }
```

**Migration:** cambiar la firma `pub fn submit(&mut self, req: AgentWorkRequest)` → `pub fn submit(&mut self, req: AgentWorkRequest) -> Result<(), StructuredWorkError>`. Llamadas existentes en tests deben adaptarse: añadir `?` o `.unwrap()`.

### 2.2 Redacción de errores de schema

**Decisión:** los mensajes de violación deben reportar el nombre del campo y el shape esperado, NO el valor observado. El valor se redacta a un placeholder estable (`<redacted>` o `value@<shape>`) que:
- Permite correlacionar qué tipo de fallo ocurrió.
- No expone contenido.

```rust
fn redacted_value_repr(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(_) => "bool".into(),
        serde_json::Value::Number(n) => format!("number({})", n),
        serde_json::Value::String(s) if s.is_empty() => "string(\"\")".into(),
        serde_json::Value::String(_) => "string(<redacted>)".into(),
        serde_json::Value::Array(a) => format!("array[len={}]", a.len()),
        serde_json::Value::Object(o) => format!("object[len={}]", o.len()),
    }
}

violations.push(format!(
    "field {name} expected {shape}, got {}",
    redacted_value_repr(v)
));
```

**Notas:**
- Para `Null`, `Bool`, `Number` mostramos el tipo o el número sin revelar contenido.
- Para `String` redactamos SIEMPRE — el contenido podría ser PII/secrets.
- Para `Array`/`Object` mostramos la longitud sin enumerar.

### 2.3 Test cases

| Test | Escenario | Esperado |
|---|---|---|
| `saw009_duplicate_request_strict_rejects` | `submit(req1); submit(req2_different)` con mismo id | `Err(DuplicateRequest { ... })` con `existing_task_ref=task:original`, `new_task_ref=task:WRONG` |
| `saw010_duplicate_request_idempotent_accepts` | `submit_idempotent(req1); submit_idempotent(req1)` | segundo `Ok(true)`, no error, no se duplica |
| `saw011_duplicate_request_idempotent_rejects_different` | `submit_idempotent(req1); submit_idempotent(req2_different)` | segundo `Err(DuplicateRequest)` |
| `saw012_duplicate_request_after_run` | `submit(req1); run_structured(req1); submit(req2_different)` | error post-run preserva receipts previos |
| `saw013_schema_violation_does_not_leak_string` | output `{"secret": "AKIA-real-key-12345"}` cuando schema pide `u64` | violación contiene `string(<redacted>)`, NO `AKIA-real-key-12345` |
| `saw014_schema_violation_does_not_leak_object` | output `{"nested": {"password": "x"}}` cuando schema pide `string` | violación contiene `object[len=1]`, NO la clave `password` ni el valor |
| `saw015_schema_violation_does_not_leak_array` | output `[1,2,3,4,5]` cuando schema pide `string` | violación contiene `array[len=5]`, NO enumeración |
| `saw016_missing_field_message_unchanged` | campo requerido ausente | violación contiene `missing field X` (sin cambio) |
| `saw017_known_descriptor_violation_unchanged` | shape mismatch en descriptor conocido | violación contiene `field X expected Y, got <tipo>`, sin valor |

### 2.4 Invariantes

```text
INV-9  La firma de `submit` cambia. Esto es BREAKING para todos los
       callers. Aceptable porque SAW tests son la única superficie
       afectada en este crate.
INV-10 El modo idempotente es estrictamente bit-exact (PartialEq). No
       hay "schema-equivalent" semántico — el caller debe ser
       determinista al re-registrar.
INV-11 La redacción NO aplica a `name` y `shape` (que son strings
       del schema, controlados por el developer) — solo al `value`
       que viene del host.
INV-12 Los receipts de SAW-005/SAW-006 NO se ven afectados — los
       receipts no almacenan `violations`, solo `outcome_kind`.
```

## §3 Riesgos y trade-offs

| Trade-off | A favor | En contra |
|---|---|---|
| Breaking change en `submit` | Surface clara, fail-loud | Cualquier consumer externo (que no sea SAW tests) rompe |
| Modo idempotente opt-in vs siempre idempotente | Explicito sobre intención | Más API surface |
| Redacción universal de strings | Conservadora | Puede ocultar bugs legítimos donde el valor sí ayuda a depurar |
| Redacción solo del valor, no del nombre del campo | El nombre del campo es metadata, no contenido | Falso positivo si el nombre del campo ES el secreto (e.g. `password` field name) |

## §4 Plan de implementación

1. Crear tests rojos SAW-009..SAW-017 (8 tests).
2. Aplicar cambio en `submit` + añadir `submit_idempotent`.
3. Añadir variante `DuplicateRequest` en `StructuredWorkError`.
4. Aplicar redacción en `redacted_value_repr` y modificar línea 155.
5. Adaptar llamadas existentes en `saw001_002`, `saw003`, etc. a `ex.submit(req).unwrap()`.
6. Re-correr SAW tests — todos verdes (7 existentes + 9 nuevos = 16/16).
7. Re-correr workspace `cargo test --workspace` — sin regresiones.
8. Emitir C1-H02-RECEIPT.

## §5 Estado actual

- Tests RED: 0 escritos.
- Cambios: 0 aplicados.
- Próxima acción: implementar tests + cambio mínimo.
