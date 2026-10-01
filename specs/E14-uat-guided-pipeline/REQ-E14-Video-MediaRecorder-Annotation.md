---
type: requirement
title: "REQ-E14-Video-MediaRecorder-Annotation"
slug: "REQ-E14-Video-MediaRecorder-Annotation"
domain: "[[E14-uat-guided-pipeline]]"
status: active
created: 2026-08-12
created_in_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_version: v1.9.1
decision_authority: "TBD by sddk-design"
tested_by: "tests/test_evidence_capture_contract.js + sddk uat open"
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: 2026-11-10
---

# REQ-E14-Video-MediaRecorder-Annotation

## Requirement

The dashboard kit (`assets/uat-dashboard/kit/components.js`) MUST
implement `evidenceCaptureUI()` handlers for `kind: video` and
`kind: annotation` using the browser `MediaRecorder` API and a
`<canvas>` overlay respectively.

**Video**:
- Records the viewport via `MediaRecorder`.
- Encodes as `video/webm; codecs=vp9`.
- Enforces a configurable duration limit (default 30 seconds),
  surfaced in the UI; auto-stops at the limit.
- Hashes the resulting `Blob` with SHA-256
  (`crypto.subtle.digest("SHA-256", …)`).
- Persists the `Blob` to the evidence store, mirroring the
  screenshot flow.
- Emits `{ kind: video, ref: <sha256>, duration_ms, size_bytes }`.

**Annotation**:
- Requires a base screenshot already persisted (sha256 known).
- Opens a modal with an editable `<canvas>` overlaid on the
  screenshot.
- Supports arrow / rectangle / text / clear tools.
- Exports the overlay as PNG.
- Hashes the PNG with SHA-256.
- Emits `{ kind: annotation, ref: <sha256>, based_on:
  <screenshot_sha256> }`.

Both kinds MUST surface `duration_ms`, `size_bytes` and the sha256
in the guided UI. The kit MUST refuse to open the annotation modal
if no prior screenshot exists, with a console-visible error
`annotation requires a base screenshot`.

## Scenarios

### Scenario: Video captures webm with hash
- **GIVEN** an evidence slot of `kind: video` in the guided UI
- **WHEN** the user records and stops (≤30s)
- **THEN** the blob is `video/webm`, sha256 is computed, persisted,
  and the slot stores `duration_ms`, `size_bytes`, and `ref`

### Scenario: Video exceeds duration limit
- **GIVEN** the 30s limit reached
- **WHEN** the recorder is running
- **THEN** it auto-stops at 30s and the slot stores
  `duration_ms ≈ 30000`

### Scenario: Annotation requires base screenshot
- **GIVEN** no prior `screenshot` evidence exists
- **WHEN** the user opens the annotation modal
- **THEN** the modal refuses to open with
  `error: annotation requires a base screenshot`

### Scenario: Annotation persists overlay hash + base ref
- **GIVEN** a base screenshot with sha256 `B`
- **WHEN** the user draws an arrow and clicks "Adjuntar"
- **THEN** a PNG overlay is produced with sha256 `O`; the slot
  stores `{kind: annotation, ref: O, based_on: B}`

### Scenario: Both kinds surface metadata in UI
- **GIVEN** a slot of either kind
- **WHEN** the slot is rendered
- **THEN** the UI shows `duration_ms`, `size_bytes` and the sha256

## Traceability

- **Decision authority:** TBD by sddk-design
- **Created in cycle:** [[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]
- **Supersedes:** `specs/E14-uat-guided-pipeline/E14.1-EVIDENCE-INBOX.md`
  (kept for history; spec-text typo of duplicated `Annotation` is
  documented as contradiction C-1; the canonical enum in
  `crates/sddk-domain/src/uat.rs:889` lists `Annotation` once)
- **Last modified:** 2026-08-12
- **Tested by:** `tests/test_evidence_capture_contract.js` (TDD, JS
  contract test required)
- **Incidences:** none

## Changelog (bi-temporal)

- 2026-08-12T07:34 | created | cycle=[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]] | valid_from=2026-08-12 | valid_to=∞
- 2026-08-12T07:34 | supersedes | E14.1-EVIDENCE-INBOX.md | contradiction C-1 (spec-text duplicate Annotation) resolved: canonical enum is in domain source | valid_from=2026-08-12 | valid_to=∞