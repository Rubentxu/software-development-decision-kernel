# SPEC-035 — Provider RPC Lifecycle

Preferred Linux mode:

```text
SDDK -> UDS -> systemd user socket activation -> provider daemon
                                      |
                                idle shutdown
```

The daemon is disposable; provider persistent indices/traces live outside process memory.

Other modes:

- standalone UDS on-demand;
- stdio child-process fallback;
- named pipe future Windows;
- secured remote transport future.

SDDK never assumes a provider process is resident. Capability availability and process activity are separate concepts.
