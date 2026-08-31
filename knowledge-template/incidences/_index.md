---
type: moc
title: "Incidences Index"
---

# Incidences

> Problems found during implementation. Each incidence links to the ADRs it challenges and the requirements it affects.

## Open (need resolution)

```dataview
TABLE WITHOUT ID
  file.link AS "Incidence",
  severity AS "Severity",
  discovered AS "Discovered",
  affects_adrs AS "Challenges ADR",
  affects_requirements AS "Affects Req"
FROM "incidences"
WHERE type = "incidence" AND status = "open"
SORT severity DESC
```

## Accepted risks (won't fix)

```dataview
TABLE WITHOUT ID file.link, severity, affects_adrs, resolution
FROM "incidences"
WHERE type = "incidence" AND status = "accepted_risk"
```

## Resolved (historical)

```dataview
TABLE WITHOUT ID file.link, severity, resolved_in_cycle, resolution
FROM "incidences"
WHERE type = "incidence" AND status = "resolved"
SORT file.name DESC
LIMIT 10
```
