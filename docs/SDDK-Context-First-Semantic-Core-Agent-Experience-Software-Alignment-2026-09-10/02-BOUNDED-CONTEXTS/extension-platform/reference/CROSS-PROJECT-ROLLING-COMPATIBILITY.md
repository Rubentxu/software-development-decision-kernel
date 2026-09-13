# Cross-project rolling compatibility

SDDK must tolerate provider evolution independently.

Recommended handshake:

```text
GetServiceInfo
GetCapabilities
negotiate protocol/capability versions
open workspace/scenario
execute semantic use case
validate result basis
```

A new CogniCode/Chronos feature is invisible to SDDK until:

1. provider exposes a stable semantic capability;
2. SDDK sees value;
3. a provider adapter version maps it;
4. SDDK Alignment/Verification chooses to consume it.

This avoids synchronized releases across three projects.
