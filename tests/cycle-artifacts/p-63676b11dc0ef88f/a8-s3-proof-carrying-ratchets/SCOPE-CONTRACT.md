# SCOPE-CONTRACT — a8-s3-proof-carrying-ratchets (AC14)

## Goal

UAT de proof-carrying changes e immune-system ratchets: un defecto
resuelto se convierte en protección durable.

| Req | Test | Estado |
|---|---|---|
| PC-1 proof portable: suite digest determinista viaja con el cambio | t_pc1 | ✅ |
| PC-2 ratchet monótono: endurecer OK, relajar → NonMonotonicStrictness | t_pc2 | ✅ |
| PC-3 firma requerida: EmptySignature/UnknownSigner rechazados | t_pc3 | ✅ |
| PC-4 waiver caducado → ExpiredOverride, no reabre el gate | t_pc4 | ✅ |

## Cambios en src (mínimos, aditivos)

- `signed_gates.rs`: constructores `GatePolicy::new` y
  `PolicyRatchet::new` — los tipos son `#[non_exhaustive]` y no
  podían construirse desde fuera del crate (gap de superficie
  pública para consumidores AC14). Sin cambio semántico.

## Evidence

- `cargo test -p sddk-engine --test a8_s3_proof_carrying_ratchets` → 4 PASS.
- `cargo test -p sddk-engine --lib signed_gates` → 8 PASS (sin regresión).
