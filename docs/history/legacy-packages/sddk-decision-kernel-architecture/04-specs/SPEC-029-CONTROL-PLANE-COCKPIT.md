# SPEC-029 — Static Control Plane / Cockpit

**Status:** Proposed

## Goal
Independent visibility over agentic work without running a server.

## Storage

```text
~/.local/share/sddk/control-plane/
├── control-plane.sqlite
├── cockpit.html
└── exports/
```

The canonical event store may be elsewhere; `control-plane.sqlite` is a query-optimized projection/cache and can be rebuilt.

## Commands

```bash
sddk cockpit build
sddk cockpit open
sddk cockpit watch
sddk journal --workflow wf-123
sddk why node nr-42
```

`watch` monitors persistence changes and atomically regenerates HTML; it does not need to serve HTTP.

## Static HTML requirements
- single self-contained file or self-contained directory mode;
- no CDN;
- no mandatory fetch/XHR;
- embedded snapshot data;
- opens from `file://`;
- CSP appropriate to local static usage;
- deterministic build for same snapshot when practical.

## Views

### Overview
- active/recent workflows;
- total/observed usage;
- provider health;
- failovers;
- errors requiring attention.

### Journal
Human-readable temporal projection with filtering.

### Workflow timeline
Node/attempt durations, waiting time, tool time and verification.

### Agent/Model/Provider
Usage, failures, route changes, historical outcomes.

### Graph
Moldable lenses: execution, causal, context, cost, evidence, UAT, supply chain.

### Failure trace
Cause → affected execution → behavior → recovery → result.

### Context
Capsules, artifacts selected/read, stale/invalidated inputs.

### UAT
Plan/run/evidence/human decision/signoff.

## Metric honesty
If remaining provider quota is unknown, display `unknown`, not an estimated 91% unless the estimate is explicitly labeled and has a defined method.

## Privacy
Cockpit must support redaction policies before embedding content. Default to metadata/references instead of full prompt or sensitive artifact bodies.
