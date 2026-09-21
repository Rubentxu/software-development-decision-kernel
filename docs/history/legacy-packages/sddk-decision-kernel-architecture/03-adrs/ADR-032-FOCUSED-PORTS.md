# ADR-032-FOCUSED-PORTS — Replace aggregate Ledger dependencies with focused ports

**Status:** Accepted


## Decision
Application services depend only on the minimal interface required for their use case. `Ledger` can remain as a temporary compatibility aggregate but must not be introduced into new application code.

## Consequences
Improves ISP/SOLID, testability and boundary clarity. More constructor parameters are acceptable when they make authority explicit; service bundles can group cohesive ports without recreating a god interface.
