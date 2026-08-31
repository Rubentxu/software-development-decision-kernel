---
type: moc
title: "Project Knowledge Index"
okf_version: "0.2"
---

# Project: {PROJECT_NAME}

> **Knowledge graph for the SDDK workflow.** Every decision, requirement, cycle, and incidence is a node. Navigate via wikilinks `[[like-this]]` or query via Dataview below.
> Open this vault in [Obsidian](https://obsidian.md) for graph view and backlinks.

## 🔒 Active Cycle (Serialization Lock)

```dataview
TABLE milestone AS "Milestone", acquired AS "Started", branch AS "Branch"
FROM "milestones"
WHERE type = "active_lock"
```

> If this table has a row, a cycle is in progress. No new cycle can start until it completes.

## 🏗️ Milestones

```dataview
TABLE WITHOUT ID
  file.link AS "Milestone",
  status AS "Status",
  target_version AS "Target",
  branch AS "Branch",
  pr AS "PR",
  tag AS "Tag"
FROM "milestones"
WHERE type = "milestone"
SORT file.name DESC
```

## 📋 Recent ADRs

```dataview
TABLE WITHOUT ID
  file.link AS "ADR",
  status AS "Status",
  affects_domains AS "Domains",
  created AS "Created"
FROM "adrs"
WHERE type = "adr"
SORT created DESC
LIMIT 10
```

## ⚠️ Challenged ADRs (need attention)

```dataview
TABLE WITHOUT ID
  file.link AS "ADR",
  challenged_by AS "Challenged by",
  affects_requirements AS "Affects"
FROM "adrs"
WHERE status = "challenged"
```

## 📝 Requirements by Domain

```dataview
TABLE WITHOUT ID
  file.link AS "Requirement",
  domain AS "Domain",
  status AS "Status",
  last_modified_version AS "Last Version",
  decision_authority AS "Decision"
FROM "specs"
WHERE type = "requirement"
SORT domain, file.name
```

## 🐛 Open Incidences

```dataview
TABLE WITHOUT ID
  file.link AS "Incidence",
  severity AS "Severity",
  discovered AS "Found",
  affects_adrs AS "ADRs"
FROM "incidences"
WHERE status = "open"
SORT severity DESC
```

## 🔄 Recent Cycles

```dataview
TABLE WITHOUT ID
  file.link AS "Cycle",
  status AS "Status",
  milestone AS "Milestone",
  verify_verdict AS "Verify",
  debt_verdict AS "Debt",
  tag AS "Tag"
FROM "cycles"
WHERE type = "cycle"
SORT started DESC
LIMIT 10
```

## 📖 Glossary (Terms)

```dataview
TABLE WITHOUT ID
  file.link AS "Term",
  domain AS "Domain",
  status AS "Status"
FROM "terms"
WHERE type = "term"
SORT file.name
```

---

## How to navigate this graph

| Question | Where to look |
|----------|---------------|
| "What's the current state?" | This page — Active Cycle + tables above |
| "What decisions affect auth?" | Open `[[specs/auth/_index]]` or grep `affects_domains` in `adrs/` |
| "What happened in cycle X?" | Open the `[[CYC-date-slug]]` node — it links to everything |
| "Why was this decided?" | Open the `[[ADR-NNN]]` — read Context + Decision Drivers |
| "Is this requirement tested?" | Open the `[[REQ-Slug]]` — check `tested_by` property |
| "Are there stale docs?" | Check `stale_after` property on any node vs today's date |
