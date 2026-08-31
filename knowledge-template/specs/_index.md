---
type: moc
title: "Specs Index"
---

# Specifications

> System requirements organized by domain. Each requirement is a node with traceability to ADRs, tests, and cycles.

## All Domains

```dataview
TABLE WITHOUT ID
  file.link AS "Requirement",
  domain AS "Domain",
  status AS "Status",
  rfc2119 AS "Level",
  last_modified_version AS "Version",
  decision_authority AS "Decision Authority"
FROM "specs"
WHERE type = "requirement"
SORT domain, file.name
GROUP BY domain
```

## Requirements without test coverage

```dataview
TABLE WITHOUT ID file.link, domain, rfc2119
FROM "specs"
WHERE type = "requirement" AND !tested_by
```

## Deprecated requirements (kept for history)

```dataview
TABLE WITHOUT ID file.link, domain, last_modified_version
FROM "specs"
WHERE type = "requirement" AND status = "deprecated"
```
