---
type: moc
title: "Milestones Index"
---

# Milestones

> One node per SDDK cycle. Tracks the serialization lock — only one milestone can be `in_progress` at a time.

## Active Lock

```dataview
TABLE milestone AS "Milestone", acquired AS "Acquired"
FROM "milestones"
WHERE type = "active_lock"
```

## By Status

### In Progress
```dataview
TABLE WITHOUT ID file.link, domain, target_version, branch, started
FROM "milestones"
WHERE type = "milestone" AND status = "in_progress"
```

### Completed (recent)
```dataview
TABLE WITHOUT ID file.link, tag, pr, completed
FROM "milestones"
WHERE type = "milestone" AND status = "completed"
SORT completed DESC
LIMIT 10
```

### Blocked
```dataview
TABLE WITHOUT ID file.link, domain, started
FROM "milestones"
WHERE type = "milestone" AND status = "blocked"
```

### Planned
```dataview
TABLE WITHOUT ID file.link, priority, target_version
FROM "milestones"
WHERE type = "milestone" AND status = "planned"
SORT priority ASC
```
