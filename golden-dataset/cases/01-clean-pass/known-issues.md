# Known Issues — False Positives to Watch For

These are findings that clusters might **incorrectly** emit for this clean code:

## debt-smells-cluster
- **Should NOT flag** the function as "too short" or "trivial" — brevity is a virtue.
- **Should NOT flag** the `Intl.NumberFormat` as a "hidden dependency" — it's the standard library.

## debt-overeng-cluster
- **Should NOT flag** the JSDoc comment as "unnecessary documentation" — it's minimal and useful.
- **Should NOT flag** the use of `Intl` as "hand-rolled replacement" — it IS the stdlib.

## debt-coupling-cluster
- **Should NOT flag** the `currency` parameter as "environment coupling" — it's injected, not read from env.
