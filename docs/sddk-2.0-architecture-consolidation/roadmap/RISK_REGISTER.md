# Risk Register

| ID | Risk | Likelihood | Impact | Mitigation | Trigger |
|---|---|---:|---:|---|---|
| R-001 | Event sourcing increases complexity before value is visible | M | H | Start with one vertical and rebuild parity tests | >2 dual-write slices remain after P2 |
| R-002 | Graph becomes a second authority | M | H | Rebuild-only contract, no authoritative GraphStore writes | graph data cannot be recreated from events |
| R-003 | Pack model adds dependency/version complexity | M | M | UAT first, conformance fixtures, small manifest v2 | load-order bugs across >3 packs |
| R-004 | Micro-crate explosion slows Rust development | M | M | Boundaries may be modules; measure compile/test ergonomics | compile time rises materially after extraction |
| R-005 | Behavior reactivity causes event storms | M | H | budgets, idempotency, loop detection, bounded drains | repeated causal cycles or queue growth |
| R-006 | Agentic relation behaviors create hidden authority | L/M | H | proposals-only rule and architecture lint | behavior imports capability executor directly |
| R-007 | Context-read tracing leaks sensitive content | M | H | IDs/hashes, bounded tracing, redaction, opt-in | raw secret/content appears in trace fixture |
| R-008 | Fork cache hides meaningful nondeterminism | M | M | content hashes + strict replay divergence | cached response used with drifted prompt/tool args |
| R-009 | Explorer drives architecture instead of domain needs | M | M | read APIs first; renderer adapters; no UI authority | domain type added solely for rendering convenience |
| R-010 | Golden dataset overfits one model/language | M | M | multi-language, held-out cases, stability metrics | F1 high but cross-language recall low |
| R-011 | Local gate receipt becomes self-attestation with no trust | M | H | independent remote verification and commit binding | protected branch accepts unverifiable receipt |
| R-012 | Documentation governance becomes bureaucracy | M | L | generate deterministic inventories; keep prose roles simple | docs updates exceed feature implementation friction |
| R-013 | Rapid release cadence destabilizes users | M | M | channels + promotion evidence | repeated patch releases with migration regressions |
| R-014 | Core ontology grows uncontrollably | M | H | ADR-007 and three-pack rule | domain noun proposed for core without cross-domain evidence |
