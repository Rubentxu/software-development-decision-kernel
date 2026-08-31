---
type: moc
title: "ADRs Index"
---

# Architecture Decision Records

> Every significant architectural decision. Immutable decision text; append-only Implementation Log.

## Accepted (binding)
```dataview
TABLE WITHOUT ID file.link, affects_domains, created, decided
FROM "adrs"
WHERE type = "adr" AND status = "accepted"
SORT decided DESC
```

## Challenged (needs attention)
```dataview
TABLE WITHOUT ID file.link, challenged_by, affects_requirements
FROM "adrs"
WHERE type = "adr" AND status = "challenged"
```

## Proposed (pending cycle closure)
```dataview
TABLE WITHOUT ID file.link, affects_domains, created_in_cycle
FROM "adrs"
WHERE type = "adr" AND status = "proposed"
```

## Superseded (historical)
```dataview
TABLE WITHOUT ID file.link, superseded_by, decided
FROM "adrs"
WHERE type = "adr" AND status = "superseded"
SORT decided DESC
```

## Deprecated
```dataview
LIST
FROM "adrs"
WHERE type = "adr" AND status = "deprecated"
```
