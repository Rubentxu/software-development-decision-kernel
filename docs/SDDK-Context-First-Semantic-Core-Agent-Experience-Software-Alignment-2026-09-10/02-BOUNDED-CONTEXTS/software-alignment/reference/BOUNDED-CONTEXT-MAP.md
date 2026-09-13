# Bounded Context Map

```text
Knowledge --------------------> Software Alignment
  ^                                  |
  |                                  | advisory assessments
Evidence Providers ------------------+
                                     |
Decision Memory -> ArchitecturalIntent
                                     |
                                     v
Verification (verify/deb-verify) <---+
             |
             v
Governance / Authority (optional consumption)
```

### Upstream
Knowledge, Decision, Evidence/provider adapters.

### Downstream
Verification, ContextCompiler, WHY/read models; Governance only through explicit policy integration.

### Anti-corruption
External provider types are normalized to SDDK Evidence/metrics before Alignment sees them.
