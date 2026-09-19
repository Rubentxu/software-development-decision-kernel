# AIW-SPEC-04 — Puertos externos, host y empaquetado

**Estado: Proposed.**

- **AIW-REQ-P01 MUST** respetar `arch-spec-021`: tipos de proveedor no entran en dominio SDDK, resultados son fuente de evidencia con refs y basis, no Authority/Verify/Alignment. Negociar protocol/capabilities **en ejecución**. Grandes grafos/trazas permanecen en proveedor por defecto.
- **P02 MUST** tras CC-S0 usar adaptador **real** CogniCode para cumplir A6/AC10; fake solo valida seam. Verificar estático contra una claim concreta y dos revisiones distintas.
- **P03 MUST** A7 con Chronos delimitar ejecutable/env/scenario/instrumentación/completitud y distinguir correlación de causalidad; contradicciones con estático se conservan, sin averaging ni elegir un proveedor como verdad universal.
- **P04 MUST** mantener BASE con proveedores ausentes. Opcional/preferido ausente produce `NOT_EVALUATED` o gap; requerido ausente bloquea **esa** operación, sin falsificar aprobación.
- **P05 MUST** compartir gateway/Authority/redacción de secrets entre CLI/host; J2 MUST justificar qué falta en `arch-spec-030` antes de publicar un trait+view JCode-specific. No exponer raw args, stdout/stderr sin enmascarar ni datos de denegación.
- **P06 MUST NOT** necesitar daemon permanente. La activación on-demand del proveedor/host se admite si cuenta con timeout/cancel/recover; eventual transporte externo no introduce nuevo owner de ledger.
- **P07 SHOULD** evaluar la separación de binarios solo en R11 con segundo consumidor probado, costes de packaging/compat y seguridad. `sddk` mantiene CLI estable o deprecación versionada. No convertir stdout de un CLI en API canónica de otro.
- **P08 OPTIONAL** Jev/TypeSafe se ensaya en corpus de routing con comparativa baseline; MUST NOT entrar en Base, en el gate/verificación, ni habilitar permisos. Si se invoca, la decisión aceptada se registra y el replay usa receipt histórico, no vuelve a llamar modelo.
