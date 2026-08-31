---
type: moc
title: "Cycles Index"
---

# Cycle Manifests

> One node per completed or in-progress SDDK cycle. Each cycle manifest is the traceability hub — it links to all artifacts, ADRs, requirements, and incidences touched.

## All Cycles (chronological)

```dataview
TABLE WITHOUT ID
  file.link AS "Cycle",
  status AS "Status",
  milestone AS "Milestone",
  path AS "Path",
  verify_verdict AS "Verify",
  debt_verdict AS "Debt",
  tag AS "Tag",
  started AS "Started"
FROM "cycles"
WHERE type = "cycle"
SORT started DESC
```

## Blocked cycles (need resolution)

```dataview
TABLE WITHOUT ID file.link, milestone, started, branch
FROM "cycles"
WHERE type = "cycle" AND status = "blocked"
```
