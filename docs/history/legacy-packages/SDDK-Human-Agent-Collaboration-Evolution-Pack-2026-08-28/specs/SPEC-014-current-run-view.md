# SPEC-014 — CurrentRunView

## Fields
project, cycle, path, phase, progress, objective, last_outcome, next_action,
human_action_required, blockers, risks, subject, artifacts, authority_refs, observed_at.

## Build order
1 CLI/runtime
2 artifacts/CAS
3 git where needed
4 vault for durable knowledge
5 interaction profile for presentation only

## Error
Missing mandatory authority => degraded/blocked view; never guessed default.
