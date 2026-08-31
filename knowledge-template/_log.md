---
type: log
title: "Project Activity Log"
---

# Activity Log (append-only)

> Chronological history of all knowledge graph operations. Each entry records: date, agent/action, what changed, and a wikilink to the affected node.

## Format

```
- YYYY-MM-DDTHH:MM | {action} | {what} | [[node-link]]
```

## Actions

| Action | Meaning |
|--------|---------|
| `created` | New node created |
| `updated` | Node properties or body modified |
| `status_changed` | Status transition (e.g., proposed→accepted) |
| `challenged` | An incidence challenged an ADR or requirement |
| `superseded` | ADR replaced by a newer one |
| `locked` | Serialization lock acquired (cycle started) |
| `released` | Serialization lock released (cycle completed) |

---

## Entries

<!-- New entries appended below by SDDK agents. Do not edit existing entries. -->
