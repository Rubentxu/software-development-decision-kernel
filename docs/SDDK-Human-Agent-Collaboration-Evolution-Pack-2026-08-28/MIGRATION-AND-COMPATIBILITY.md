# Migración y compatibilidad

## Estrategia: strangler de presentación

No sustituir de golpe el `human_summary` actual.

1. Introducir `InteractionEvent` detrás de feature/config.
2. Proyectar el human summary existente desde StageReport cuando sea posible.
3. Comparar legacy vs new renderer en shadow mode.
4. Medir semantic parity.
5. Promover el nuevo renderer.
6. Mantener lectura de artifacts legacy.

## Config migration

Si no existe profile:
```yaml
audience: novice
autonomy: balanced
personality: wisecracking_robot
```

Si existe `report_audience`, mapearlo sin modificar el archivo original hasta una operación explícita de migration.

## Schema versioning

Todos los envelopes incluyen `schema_version`.

Reglas:
- reader N acepta N y N-1;
- migración explícita cuando cambie significado;
- campos nuevos opcionales antes de hacerlos required;
- unknown fields preservables donde el runtime actual lo permita.

## CLI

- `status/plan/run/ship/recover` no cambian su semántica.
- low-level commands no se eliminan.
- Companion commands son proyecciones/explicaciones, no rutas alternativas del lifecycle.
- prompts dejan de repetir recetas CLI y referencian matrix rows.

## Rollback

Desactivar Companion renderer => volver a human summary legacy.
No se requiere rollback del ledger porque Companion no es autoridad de lifecycle.
