# AIW-ADR-03 — Persistencia observacional sucesora solo ante carencia demostrada

**Estado: Proposed / CONDICIONAL. NO implementar automáticamente.** **Ámbito:** Evidence/Observation + storage.

## Contexto

El documento `6251af7` identifica que `EvidenceAttachmentRecord::from_universal_relation` acepta `observed_for`, `verifies`, `references`, `justifies` y rechaza en escritura `supports`, `gates`, `produced_by`, `contradicts`. `observation` preserva soportes/refutaciones en memoria; un adjunto actual no representa de forma comprobada dos observaciones contradictorias con identidad y basis propias. Es un hueco para persistir contradicciones, **no** un bloqueo para toda evidencia básica de A6.

## Decisión candidata

1. **Primero** demostrar con fixture persistente que la relación exigida no puede expresarse mediante `CoreRelationKind`/`ObservationId`, el writer universal existente ni un evento canónico adecuado. No extender `EvidenceKind` legacy para forzar la escritura.
2. **Si falla**: proponer una forma sucesora mínima fila-por-observación/referencia relacional, con `ObservationId`, base, producer ref, scope, relación y hash de contenido canónico. El cuerpo grande permanece CAS/proveedor; no segunda Knowledge DB. Una migración aditiva con dual-read/strangler y sin destruir attachments previos.
3. Factos/objetos y relaciones son writer-owned; projector reconstruible y nunca escritor de autoridad. Errores de ref/basis/relación son fail-closed. Soporte y contradicción son enlaces, no veredictos de governance.
4. No usar el `DigestSha256` provisional de CC-S0 como SHA-256. Resolver la identidad canónica antes de persistir/rehusar artefactos; `schema_version` y migraciones requieren pruebas de vuelta atrás de lectura y concurrencia CAS.

## Alternativas rechazadas

`ObservationSet` completo como blob JSON sin identidad por observación; doble-write indiscriminado; relación inventada dentro de una columna legacy; duplicar KnowledgeBasis y sus hashes.

## Gate de implementación

Dos observaciones contradictorias de revisiones/ámbitos trazables sobreviven a reinicio y ambas se recuperan; referencia inexistente, FNV disfrazado, hash inválido, repetición y colisión UNIQUE fallan controladamente; Base anterior sigue leyendo sus evidencias. Si el UAT puede pasar con mecanismos actuales, cancelar esta ADR sin nueva tabla.
