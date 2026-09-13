# SPEC-030 — Intelligence Provider SPI

SDDK-owned ports:

```rust
trait CodeIntelligencePort {
  analyze_delta(...)
  analyze_scope(...)
  analyze_impact(...)
  capabilities(...)
}

trait RuntimeIntelligencePort {
  observe_scenario(...)
  compare_scenario(...)
  behavior_summary(...)
  capabilities(...)
}
```

Ports speak SDDK semantics, not CogniCode/Chronos tool names. Concrete RPC adapters live outside `sddk-domain`.

Providers return normalized Evidence/basis; large native datasets can remain in provider stores with stable refs/digests.
