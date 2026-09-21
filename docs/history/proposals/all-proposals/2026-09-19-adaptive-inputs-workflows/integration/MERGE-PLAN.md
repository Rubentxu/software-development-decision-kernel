# Integración segura del paquete y sustitución prospectiva de la documentación anterior

## Modo recomendado: merge aditivo sin overwrites

El ZIP contiene **únicamente** archivos bajo `docs/proposals/2026-09-19-adaptive-inputs-workflows/`. Extraer en la **raíz del repo** manteniendo las rutas: `unzip -n sddk-adaptive-inputs-workflows-2026-09-19.zip -d <repo-root>`; `-n` evita sobrescribir archivos existentes. Validar `git status --short` y `git diff --check`. El paquete no contiene código Rust ejecutable ni modifica por sí solo el roadmap, la base de datos, prompts o tests.

## Pasos de incorporación

1. **Comprobar checkout actual:** `git rev-parse HEAD`, `git status --short` y arquitectura normativa. Si HEAD difiere de `da37510`, volver a comparar roadmap y símbolos, en particular cc-s1/cc-s2, arch-spec-021, IPB-009, `DigestSha256` y `operator::build_operator`. No asumir que el ZIP representa futuras entregas.
2. **Copiar docs** y revisar [ROADMAP-OVERLAY.md](ROADMAP-OVERLAY.md). No sobrescribir `docs/architecture/a5/A5-CURRENT-ROADMAP.md` ni `docs/architecture/README.md` sin reconciliación y aprobación. El overlay contiene textos listos para insertar en el proceso de sync autorizado.
3. **Asignar primer WorkItem y contrato:** elegir AIW-S0/S1 (A6) si existe proveedor real; anotar ID existente, depender de A5-C/CC-S0 sin reabrirlos. Si no hay proveedor real, hacer solo protocolo/transport discovery etiquetado BLOCKED y no declarar AC10 PASS.
4. **Promover UNA ADR cuando su scope lo exija:** seguir ADR-0001, consultar numeración vigente, reescribir `AIW-ADR-*` a número canónico libre, añadir tests, stakeholders y fecha; `status: accepted` solo con autoridad. AIW-ADR-03 solo tras UAT que demuestre hueco de shape.
5. **Anclar SPEC/UAT al ciclo real** sin copiar todo el paquete como requisitos de una entrega única. `AIW-REQ-*` son IDs de este ZIP hasta asignación oficial. UAT no ejecutados → `NOT_RUN`.
6. **Sustituir referencias futuras a documentos de origen**, no borrarlos. Añadir enlace de consolidación en los históricos SOLO tras adopción; conservar commits, hallazgos y receipts originales. Ver [LEGACY-DISPOSITION.md](LEGACY-DISPOSITION.md).
7. **Adoptar procesos internos y prompts por slice:** no modificar todos los 70 agentes ni introducir 12 comandos en todas las skills. Cambiar el prompt/skill que consume el primer productor, ejecutar E2E y medir; ampliar después.
8. **Crear receipt tras ejecutar realmente:** recoger repo SHA, fixture/proveedor real, output refs, UAT positivos/negativos, límites. No usar texto del paquete como prueba de green.

## Cambios que NO vienen aplicados

- Ninguna modificación a `docs/architecture/*`, `docs/SDDK-Production-Readiness-Alignment-*`, `AGENTS.md`, `prompts/`, `skills/`, código, Cargo workspace, release, ledger o tests existentes.
- No migración SQL, nuevo enum/trait, tabla Agenda o API TypeSafe.
- No renombrado destructivo de documentos con fecha, ni eliminación de propuestas J2/CC-S0, ni sobrescritura de certificaciones.

## Reglas de conflicto

Si el roadmap actual ha avanzado A6/A7, este paquete deja de ser fuente de status: conservar hechos nuevos de runtime+receipt y actualizar la matriz antes de abrir trabajo. Si una ADR aceptada ya resuelve el mismo requisito, citarla y **eliminar la ADR candidata duplicada**, no renumerarla. Si una nueva family/CLI exige modelo nuevo, adjuntar evidencia de la carencia y pasar [regla de admisión](../02-DECISIONS-AND-SCOPE.md) antes de implementarlo.
