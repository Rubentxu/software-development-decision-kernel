# SPEC-005 — Evidence and Provenance Model

**Status:** Proposed

## 1. Goal

Make evidence a reusable platform primitive rather than a UAT-only concern.

## 2. Core model

```text
Claim/Observation
      |
      +-- supported_by --> Evidence
      +-- contradicted_by -> Evidence
      |
      v
Oracle / Review
      |
      v
Verdict / Decision
```

Each evidence item MUST record:

- stable ID;
- kind;
- source/artifact reference;
- content hash;
- captured-at time;
- actor;
- subject(s);
- acquisition method;
- optional validity window/version binding;
- optional redaction metadata.

## 3. Evidence kinds

The existing UAT vocabulary should seed, not constrain, the universal model. Expected kinds include:

- file;
- screenshot;
- command output;
- assertion;
- metric;
- trace;
- console;
- network;
- HTTP;
- DOM;
- ARIA/accessibility;
- geometry;
- video;
- trajectory;
- note;
- test result;
- benchmark;
- source snapshot.

## 4. Claim-Evidence-Oracle-Verdict

A reusable assurance record SHOULD model:

- `claim`: what is asserted;
- `evidence[]`: what supports/contradicts it;
- `oracle`: deterministic rule, agent reviewer or human reviewer;
- `confidence`: optional calibrated estimate, never standalone authority;
- `actor`;
- `verdict`;
- `receipt`.

## 5. Version binding and staleness

Evidence SHOULD bind to the version/hash of relevant subjects. If those subjects change, staleness rules may mark the evidence `possibly_stale` or `invalidated`.

## 6. Redaction

Secrets MUST NOT be copied into evidence. Evidence adapters must support redaction before hashing/persistence when the raw source cannot safely be stored. The receipt should record redaction policy/version.

## 7. Reuse

The same evidence model SHOULD support:

- UAT;
- architecture review;
- security findings;
- release gates;
- benchmark claims;
- documentation freshness;
- agent evaluation;
- policy exceptions.
