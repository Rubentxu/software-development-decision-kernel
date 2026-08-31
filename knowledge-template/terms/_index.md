---
type: moc
title: "Terms Index"
---

# Glossary

> Domain terms used across ADRs, specs, and design docs. Prevents ambiguity.

```dataview
TABLE WITHOUT ID
  file.link AS "Term",
  domain AS "Domain",
  status AS "Status",
  used_in_adrs AS "Used in ADRs",
  used_in_requirements AS "Used in Reqs"
FROM "terms"
WHERE type = "term"
SORT domain, file.name
```
