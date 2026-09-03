---
name: rust-systems-reasoning
description: Rust systems architecture, performance, parsing, memory layout, async boundaries, invariants, zero-copy, and correctness reviews. Use when designing or reviewing Rust systems code where control flow, resource liveness, type-driven invariants, platform isolation, binary parsing, unsafe/zerocopy, or proof-oriented testing materially affect correctness or performance.
license: MIT
compatibility: Agent Skills standard. Suitable for OpenCode and other SKILL.md-compatible agent harnesses.
metadata:
  version: "1.0.0"
  domain: "rust-systems"
---

# Rust Systems Reasoning

Use **tight core** as the leading design idea: keep the domain core easy to trace, explicit about invariants, and isolated from incidental platform complexity.

## Workflow

1. **Establish constraints**
   - Identify correctness invariants, latency/throughput budgets, allocation/copy limits, platform constraints, and failure semantics.
   - Separate measured requirements from assumptions.
   - Done when every material design choice can be evaluated against an explicit constraint.

2. **Draw the boundary**
   - Put domain/protocol/state-machine logic in the core.
   - Put OS APIs, drivers, IPC, filesystem, sockets, clocks, executors, queues, and external services behind ports/adapters.
   - Prefer straight-line synchronous core execution when the domain permits it.
   - Done when every external side effect has an explicit boundary.

3. **Encode invariants**
   - Validate untrusted data at entry.
   - Convert validated data into types that preserve the established guarantees.
   - Use newtypes, enums, typestates, non-zero/range-constrained types, ownership, and lifetimes before comments or repeated runtime checks.
   - Done when invalid states are unrepresentable where practical, and remaining runtime invariants are named.

4. **Choose the representation**
   - For fixed native layouts, consider zero-copy only after proving layout, alignment, validity, aliasing, and lifetime requirements.
   - For variable layouts such as TLV, length-prefixed records, or nested formats, use a parser layer rather than forcing a struct-shaped representation.
   - Keep `unsafe` narrow and expose a safe API whose preconditions are mechanically enforceable.
   - Load [memory-layout.md](references/memory-layout.md) for zero-copy, binary parsing, FFI, casts, or unsafe code.
   - Done when representation assumptions are explicit and testable.

5. **Place concurrency deliberately**
   - Keep async, blocking, scheduling, retries, buffering, and backpressure at adapters unless core semantics genuinely require them.
   - If async enters the core, document the concrete capability it enables and the control-flow/liveness cost it introduces.
   - Load [async-boundaries.md](references/async-boundaries.md) for async or concurrent designs.
   - Done when ownership, blocking points, queues, cancellation, and resource lifetimes are traceable.

6. **Verify by risk**
   - Use examples/unit tests for local behavior, property tests for invariants, fuzzing for parsers/state machines, and formal/model checking for compact high-consequence properties.
   - Treat compile-time guarantees as part of the verification strategy.
   - Load [verification.md](references/verification.md) when the task involves safety, fuzzing, unsafe code, protocol parsing, or formal verification.
   - Done when each important invariant has a verification mechanism or an explicit reason why it does not.

7. **Review the whole path**
   - Trace at least one representative operation end-to-end: input → validation → core state transition → side effect.
   - Account for copies, allocations, locks, blocking, awaits, ownership transfers, and unsafe boundaries along that path.
   - Load [review-checklist.md](references/review-checklist.md) for architecture/code reviews.
   - Done when no material step in the representative path depends on an unnamed assumption.

## Decision order

Prefer, in order:

1. correctness and memory safety;
2. explicit invariants and understandable control flow;
3. measured performance;
4. architectural isolation;
5. ergonomics.

Do not trade a higher item for a lower one without stating the trade-off.

## Output

For design work, return:
- constraints and invariants;
- proposed core/boundary split;
- type/state model;
- concurrency model;
- representation/parsing strategy;
- verification strategy;
- measurable risks and trade-offs.

For review work, rank findings by **correctness → safety → performance → architecture → ergonomics**. For each finding give evidence, consequence, and the smallest robust change.
