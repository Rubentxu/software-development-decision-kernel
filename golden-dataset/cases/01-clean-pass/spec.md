# Spec: Add `formatPrice` utility

## Requirement

Add a pure function `formatPrice(cents: number, currency: string): string` that formats a price in cents to a display string.

## Scenarios

- `formatPrice(1099, "USD")` → `"$10.99"`
- `formatPrice(0, "EUR")` → `"€0.00"`
- `formatPrice(-500, "USD")` → `"-$5.00"`

## Notes

This is a pure function with no side effects, no state, no dependencies beyond `Intl.NumberFormat`. It should be trivially testable.
