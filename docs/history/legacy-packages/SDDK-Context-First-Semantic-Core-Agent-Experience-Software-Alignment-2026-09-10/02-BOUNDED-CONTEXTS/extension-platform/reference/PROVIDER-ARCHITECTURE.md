# Intelligence Provider Architecture

```text
SDDK Verify/DebVerify
      |
      +-- CodeIntelligencePort ---- RPC adapter ---- CogniCode/other
      |
      +-- RuntimeIntelligencePort - RPC adapter ---- Chronos/other
```

Providers are optional and replaceable. SDDK does not embed their crates in domain/application. Agent MCP access remains a separate exploratory path.
