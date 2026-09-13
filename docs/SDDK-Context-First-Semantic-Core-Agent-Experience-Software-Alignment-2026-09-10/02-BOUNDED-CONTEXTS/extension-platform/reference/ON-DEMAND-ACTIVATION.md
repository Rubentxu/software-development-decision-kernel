# On-demand activation

Preferred Linux UX follows the Podman-style pattern:

```text
socket listening, no provider process
 -> first RPC connect
 -> systemd --user starts provider
 -> warm requests reuse process/cache
 -> idle timeout after no active jobs
 -> process exits
```

Provider availability is separate from process activity. A fully enhanced SDDK can have zero provider daemons resident while idle.
