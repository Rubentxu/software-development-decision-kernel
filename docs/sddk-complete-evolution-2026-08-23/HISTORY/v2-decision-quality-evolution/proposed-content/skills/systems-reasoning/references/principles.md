# Systems Reasoning Reference

- Validate at trust boundaries and preserve guarantees.
- Keep important control flow traceable where practical.
- Isolate incidental platform/runtime concerns behind explicit boundaries.
- Add execution-model complexity only for demonstrated need.
- Define queue capacity/backpressure.
- Require workload+metric for performance claims.
- Keep representation and domain semantics distinct.
- Use the cheapest adequate verification mechanism.

Representative trace:

```text
input → validation → state transition → side effect → verification
```

Mark waits, ownership/liveness, locks, queues, trust changes and relevant hot-path costs.

Verification ladder:

```text
compiler/type guarantees → unit/integration → property tests → fuzzing → model/formal when tractable
```

Selected heuristics are influenced by Netstack3/zerocopy systems reasoning:
https://joshlf.com/posts/netstack-fm-ep-10/
