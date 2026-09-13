# SPEC-025 — Alignment Workbooks

Workbooks are Excel-like queryable projections.

```text
WorkbookDefinition
  id/version
  lens?
  tables[]
  metrics[]
  charts[]
  controls[]
  drilldown[]
  required_inputs[]
```

Formats: text, JSON, CSV, UI grid and chart series.

Drill-down is uniform:

`system -> context -> package/crate -> module -> file -> symbol -> evidence`.

Grid edits emit typed commands that update Decision Memory/assertions/waivers and rebuild projections. Direct cell persistence is forbidden.
