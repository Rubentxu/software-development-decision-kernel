# Workbook schema

```text
WorkbookDefinition
  id/version/lens_refs
  tables[]
  metrics[]
  charts[]
  controls[]
  drilldown_path[]
  projection_inputs[]
```

## Grid columns

Cada celda debe ser una de:

- measured value;
- categorical assessment;
- projected decision field;
- EvidenceRef/DecisionRef link;
- freshness/confidence component.

No admitir texto LLM sin provenance.

## Edición

```text
editable cell intent
 -> SemanticEditCommand
 -> owner context use case
 -> canonical fact/object
 -> projection rebuild
```

Ejemplos:

- cambiar tradeoff -> Decision BC;
- declarar bounded context -> Decision/Knowledge declaration use case;
- aprobar waiver -> Governance;
- recalcular score/chart -> sólo projection.
