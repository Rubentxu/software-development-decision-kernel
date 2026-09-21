# AIW-ADR-05 — CLI y hosts comparten casos de uso sin fragmentar autoridad

**Estado: Proposed · Ámbito:** CLI composition root, Gateway, JCode, R11.

## Contexto

SDDK tiene CLI amplio; J2 propone un adapter específico de JCode; arch-spec-030 exige frontera genérica host-neutral, sin daemon obligatorio. Aumentar binarios por dominios antes de extraer casos de uso multiplica composición, permisos y storage. El gateway ya tiene runner seguro y redacción SEC-1, pero la ruta de captura de agent shell debe enlazarse con él.

## Decisión propuesta

- Mantener `sddk` como única distribución obligatoria; comando `run-and-record` es candidato de *presentación* del runner/capability gateway existente, no segundo ejecutor.
- JCode y CLI pasan por los mismos casos de uso y Authority. Reutilizar la frontera de arch-spec-030 antes de introducir trait+view JCode-specific. El adapter controla acceso a stdout/stderr enmascarados, permisos y errores sin args secretos.
- Un CLI futuro solo si R11 demuestra distribución independiente/privilegios/dependencias/consumidor externo; su `main` se compone sobre mismos casos de uso sin invocar `sddk` por parsing de stdout.
- Herramientas ajenas (CogniCode/Chronos/test/bench) son adaptadores/ejecutables especializados, no obligatoriamente crates/binarios SDDK.

## Alternativas rechazadas

Extraer `sddk-dev` y `sddk-inspect` ya; service mesh de CLIs; segundo ledger/proyección canónica por proceso; publicar API 1.0 antes de segundo host; dejar que host vea unredacted `CapabilityReceipt` o args denegados.

## UAT

CLI y host producen resultado semántico equivalente, denegación 0 side effects, secretos canary no aparecen en salida/error/CAS/host, multi-proceso respeta concurrencia; Base puede ejecutarse sin daemon ni proveedores externos.
