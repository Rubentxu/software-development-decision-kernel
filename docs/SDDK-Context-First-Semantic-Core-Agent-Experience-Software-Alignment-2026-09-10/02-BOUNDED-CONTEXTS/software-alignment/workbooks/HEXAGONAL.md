# Workbook — Hexagonal Lens

Primary table: `unit | expected role | observed role | allowed deps | forbidden deps | effects | ports | adapters | assessment`.

DSM compares module dependencies. Heuristics produce TENSION/MISALIGNED; explicit forbidden dependency rules can produce ContractViolation.

Controls: port bypass, adapter leakage, inward dependency violation, direct infrastructure dependency, cycles.
