# SPEC-005 — Preference Memory

## Lifecycle
observation -> candidate -> learned -> pinned.

## Preference
id, value, confidence, occurrence_count, provenance, first_seen, last_seen, state, editable.

## Promotion
No promover por una observación aislada salvo `explicit_user_preference`.
Default learned threshold: >=3 evidencias consistentes y confidence >=0.75.

## Commands
memory list/show/edit/forget/pin/export/reset.

## Storage
XDG, write atomic, schema versioned.

## Engram
Optional secondary adapter only.
