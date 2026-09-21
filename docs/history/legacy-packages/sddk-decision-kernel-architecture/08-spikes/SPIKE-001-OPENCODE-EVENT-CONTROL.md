# SPIKE-001 — OpenCode Event & Control Adapter

## Question
Can SDDK observe meaningful OpenCode lifecycle/errors and inject context/commands without embedding OpenCode semantics into the kernel?

## Hypothesis
Yes: implement a thin adapter around OpenCode session/event APIs, normalize host events, and prove bidirectional control.

## Scope
- connect to/create an OpenCode session;
- subscribe to event stream;
- capture provider/model/session/tool errors and lifecycle events;
- map them to canonical events;
- issue one controlled prompt/turn with explicitly selected model when supported;
- inject a `ContextCapsule` or recovery message through adapter control;
- abort/resume if supported;
- record capabilities advertised by adapter.

## Out of scope
- generic support for every IDE;
- production routing policy;
- full Cockpit.

## Test harness
Use fake event fixtures in addition to a real OpenCode integration. Keep fixtures so API changes are detectable.

## Success criteria
- raw OpenCode event never leaks into domain types;
- canonical event contains provenance;
- adapter capability discovery is explicit;
- SDDK can start/continue a logical attempt using selected route parameters;
- event order/correlation is preserved well enough for causal trace.

## Deliverables
- `OpenCodeAgentHost` adapter;
- contract tests;
- sample event fixture corpus;
- mapping table;
- compatibility/version notes.
