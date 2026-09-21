# Intelligence Provider Ports

## Port owned by SDDK

```rust
trait CodeIntelligencePort {
    analyze_delta(...);
    analyze_scope(...);
    analyze_impact(...);
    capabilities(...);
}

trait RuntimeIntelligencePort {
    observe_scenario(...);
    compare_scenario(...);
    behavior_summary(...);
    capabilities(...);
}
```

Los tipos son ADTs de SDDK. No reexportar protobuf externos.

## Adapter lifecycle

```text
ProviderRegistry
 -> discover endpoint
 -> negotiate protocol/capabilities
 -> map to BASE/STATIC_ENHANCED/RUNTIME_ENHANCED/FULLY_ENHANCED
 -> invoke only when plan requires/preferred
```

## Provider requirement

`OPTIONAL | PREFERRED | REQUIRED` por Task/Policy.

## On-demand service

Un provider puede estar `DORMANT` y seguir siendo AVAILABLE. UDS/socket activation/stdio child son detalles de adapter.
