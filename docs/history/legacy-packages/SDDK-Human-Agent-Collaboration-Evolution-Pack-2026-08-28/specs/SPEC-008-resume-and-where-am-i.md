# SPEC-008 — Resume & Where Am I

## Resume
Reconstruir desde CurrentRunView + last meaningful events + artifacts.

## Output
objective, where_left, important_changes, state, pending, human_action.

## Limits
<=150 palabras novice default.

## Cold start
Si no existe trusted cycle id y el runtime no puede descubrirlo con garantías: reportar blocked/unknown. Nunca inferir del vault lock o una rama git.
