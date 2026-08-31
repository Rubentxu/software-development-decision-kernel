---
status: accepted
date: 2026-08-18
deciders: [orchestrator]
linked_cycles: []
---

# ADR-0016 — Universal Evidence Model

## Status

Accepted — implemented in `sddk-domain/src/evidence.rs`.

## Contexto

Phase 3 (Evidence & Governance) MUST item: "Extract universal Evidence model from UAT concepts."

The SDDK system already has a rich evidence model in `sddk-domain/src/uat.rs` (`UatEvidenceBundle`, `UatEvidenceArtifact`, `UatEvidenceEnvironment`, `UatEvidenceExecution`). This model was designed for UAT but its structure — content-addressable artifacts, environment context, and execution metadata — is genuinely universal: any governed capability can produce evidence.

The alternative was to create a parallel, generic evidence model that UAT-specific types would implement/extend. This adds complexity and abstraction layers for marginal benefit — the UAT types ARE the universal model; they just weren't labeled as such.

## Decision

Promote `UatEvidenceBundle`, `UatEvidenceArtifact`, `UatEvidenceEnvironment`, `UatEvidenceExecution`, and `UatEvidenceKind` as the **canonical universal Evidence model** for SDDK, not a UAT-specific construct.

`UatEvidenceKind` values cover the full capture taxonomy used by governed capabilities (file, screenshot, console, network, DOM, ARIA, geometry, trace, video, etc.). Adding a new evidence kind is an extension, not a modification.

### Type taxonomy

```
EvidenceBundle
├── artifacts: Vec<EvidenceArtifact>   # content-addressable captured artifacts
├── environment: EvidenceEnvironment  # what environment the execution ran in
└── execution: EvidenceExecution      # who/what model executed the capability

EvidenceArtifact
├── kind: EvidenceKind               # what was captured
├── ref: String                      # sha256:<hex> content hash
├── path: Option<String>             # storage path
├── mime: Option<String>             # MIME type
├── size_bytes: Option<u64>
└── note: Option<String>

EvidenceKind (closed enum, extensible)
├── File | Screenshot | CommandOutput
├── Assertion | Metric
├── Trace | Console | Network
├── Http | Dom | Aria | Geometry
├── Video | Trajectory
└── Note  (default)

EvidenceEnvironment
└── git_sha, app_version, browser, viewport, os

EvidenceExecution
└── executor, model, model_hash, prompt_hash
```

### Implementation

1. Create `sddk-domain/src/evidence.rs` as the canonical module
2. Move `UatEvidence*` types to `evidence.rs` and rename to `Evidence*` (dropping `Uat` prefix)
3. Create `pub use evidence::*` in `sddk-domain/src/lib.rs`
4. Add `pub type UatEvidenceBundle = EvidenceBundle` etc. in `uat.rs` for backward compat
5. Update `sddk-gateway/src/evidence.rs` to use the domain types directly (not re-define)
6. Update all imports across `sddk-gateway`, `sddk-cli` to use the new names

### Backward compatibility

Old names as type aliases in `uat.rs`:
```rust
pub use evidence::{EvidenceBundle, EvidenceArtifact, EvidenceEnvironment,
                   EvidenceExecution, EvidenceKind};
pub type UatEvidenceBundle = EvidenceBundle;
pub type UatEvidenceArtifact = EvidenceArtifact;
// etc.
```

## Consecuencias

Positivas:
- Evidence model is clearly positioned as a first-class domain concept, not UAT-specific
- Single source of truth — no duplication between domain and gateway
- Extensible via `EvidenceKind` enum variants (extension, not modification)
- `evidence_refs: Vec<String>` in `EventEnvelopeV1` remains as lightweight string references to full bundles stored elsewhere

Negativas:
- Rename of UAT types requires broad import updates across gateway and CLI
- Some field names in `EvidenceKind` may feel UAT-centric (e.g., `Dom`, `Aria`) but these are reasonable generic terms for capture types

## Alternativas considered

- **Separate generic + UAT-specific model**: trait + impl — adds indirection without benefit; the types are already general enough.
- **Evidence as a trait object**: `dyn Evidence` in `EventEnvelopeV1.evidence_refs` — breaks serialization and adds heap allocation.
- **Leave as-is**: UAT types remain technically universal but poorly labeled — this ADR is primarily a naming/positioning decision.
