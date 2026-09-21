# SPEC-013 — Moldable Explorer and Projection UX

**Status:** Proposed

## 1. Goal

Build an inspector where the same underlying ledger/graph can be viewed through task-specific representations rather than one universal graph screen.

## 2. Primary dimensions

The explorer has two first-class navigation axes:

- **Graph:** what is connected to what;
- **Timeline/Trace:** what happened, in what order and because of what.

## 3. Moldable views

A view is a declarative projection with:

- applicable node/selection types;
- query/filter;
- layout/renderer;
- drill-down targets;
- actions permitted;
- side panels/metrics;
- provenance requirements.

Initial views:

- Overview;
- Cycles;
- Architecture/C4;
- Verification;
- Evidence;
- UAT;
- Agent execution;
- Capabilities/Approvals;
- Release assurance;
- Fork diff;
- Raw trace.

## 4. C4 navigation

C4 Context -> Container -> Component SHOULD be represented as successive graph projections, not unrelated generated diagrams. Double-click/drill-down changes projection scope while preserving identity/provenance.

## 5. Diagram/canvas integration

The architecture SHOULD keep rendering behind adapters. Candidate renderers (to spike, not hard-code immediately) include:

- high-performance WebGL graph renderer for large code/knowledge graphs;
- tldraw-like canvas for editable annotations and freeform architecture canvases;
- SVG/HTML for deterministic reports;
- Mermaid/PlantUML export for portable text diagrams.

## 6. Large graph strategy

The UI MUST NOT render the full code graph by default. Query-backed progressive disclosure, clustering, level-of-detail and server/WASM-side filtering are required.

## 7. View descriptors

View definitions SHOULD be pack-extensible. Example in `examples/views/release-assurance.yaml`.
