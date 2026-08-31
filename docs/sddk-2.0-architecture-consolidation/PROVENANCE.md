# Package Provenance

This package is a consolidation artifact produced from:

- the SDDK repository review pinned to `Rubentxu/software-development-decision-kernel@eb5117e6cd4366ceb205a5b2dde4195aa396d32f` (v1.9.1);
- prior SDDK stabilization decisions around local-first/XDG state, deterministic Rust runtime, capability governance, ledger and packs;
- the architecture discussion adapting ideas from `yoheinakajima/activegraph` and `activegraph-packs`, especially event sourcing, graph projection, reactive behaviors, relation behaviors, fork/replay/diff/promote, frames, policies/approvals, context-read tracing, minimal core vocabulary, layered packs and bridge packs;
- earlier project discussions about C4/UML, Arrows.app-style graph navigation, Moldable Development, high-performance graph rendering, tldraw-style canvases, Cognicode and guided UAT.

ActiveGraph ideas are **adapted, not copied as SDDK authority semantics**. The central SDDK-specific decision is that reactive behaviors can propose but the deterministic workflow/policy/capability kernel remains authoritative for governed effects.
