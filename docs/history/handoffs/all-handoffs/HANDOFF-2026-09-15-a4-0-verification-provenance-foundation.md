# HANDOFF — A4-0 Verification Provenance Foundation closed (v1.169.40)

> **A4-0 is CLOSED.** A4-1 has **not** started. This handoff ends with the
> evidence-based recommendation on Generic Verify readiness that was requested.

## Certified state

| | |
|---|---|
| **Slice** | A4-0 — normative reconciliation + Evidence/Observation/Provenance substrate |
| **Cycle** | `p-63676b11dc0ef88f/a4-0-verification-provenance-foundation` — **CLOSED** (sequence 14) |
| **Tag** | `v1.169.40` |
| **Release / bundle** | binary `1.169.40`, bundle `1.169.40`, framework `current → 1.169.40` |
| **HEAD == origin/main** | `f9b0d19` |
| **Tests** | **205 blocks / 4314 passed / 0 failed** (+23) |
| **A3 baseline** | green, unchanged (frozen at `98b7fc7`, tag `v1.169.39`) |
| **Specs** | `arch-spec-042` implemented; `043`–`047` contract-ready |
| **ADRs** | `ADR-0122` Evidence observes software · `ADR-0123` Verify and DebVerify are distinct kernels · `ADR-0124` Alignment is advisory (mirrored to the vault) |

## Phase 0 — normative consolidation

The historical context-first specs for Alignment/Verify/DebVerify use
`SPEC-019..034`. Those numbers are **already owned** by the normative repository
(`arch-spec-019` production-readiness convergence … `arch-spec-035` paradigm lens
system). Verified free from **042** (highest assigned was 041). Consolidation is at
042+, as proposed.

The historical model is **reconciled, not copied**: the seven alignment statuses,
`ContractViolation | AlignmentTension | ImprovementOpportunity`,
`ArchitecturalIntentSnapshot` as an input, and "a `ContractViolation` requires an
explicit contract" all survive. The advisory boundary moved from aspiration to
**executable fact** on the strength of A3's `AdvisoryContext` fixture.

## The substrate (what A4-0 delivers)

```text
SoftwareRelation { from, kind: CoreRelationKind, to }              ← ADT, not a rendering
RelationId    = sha256("sddk.software_relation.id.v1|"    | from | kind | to)
ObservationId = sha256("sddk.software_observation.id.v1|" | subject | evidence
                                                          | origin | stance | basis)
```

Excluded from identity: timestamps, messages, severities, rendered text, the
producer label. Included: `revision` and `knowledge_basis` (clock-stable; A3
measured that `semantic_graph_digest` is not).

`ObservationOrigin` (`DeterministicLocal | StaticProvider | RuntimeProvider |
HumanDeclared | Inferred`) is a **distinct axis** from provenance strength, so a
distinct enum; the mapping onto `OBSERVED / DECLARED / INFERRED / DECIDED /
VERIFIED` is documented rather than encoded twice (DECIDED is decision memory,
VERIFIED is AC1's `ClaimOutcome` — neither is an observation).

Contradictions are first-class: `ObservationStance::Affirms | Denies` on an
insert-only set, resolution
`Supported | Contradicted | Conflicted { supporting, contradicting } |
Insufficient(gap)`. **No latest-wins, no confidence number.** `Conflicted` is a real
outcome — exactly the static/runtime disagreement CogniCode and Chronos will produce.

Projection emits extension nodes plus the **`a4_rel_observes` edge**
(observation → relation) into the one `SemanticGraphProjection`. No second graph, no
second DB. No third `EvidenceRef` (two already existed; the universal one is reused).

WHY now closes `evidence → observes → software relation` when an observation covers
the subject, and keeps its unresolved edge **only** when nothing did.

## Two things the closeout corrected in itself

1. **A text pin caught a now-false explanation.** Wiring the leg made
   `EVIDENCE_TO_SOFTWARE_REASON` claim "there is no evidence node kind and no edge
   …" — false the moment A4-0 provided it. Corrected; the previous engine **and**
   e2e pins now assert the *absence* of that false claim. This is `FU-A3-S15-4`
   recurring and being caught by its own remedy.
2. **The vocabulary hygiene produced a disposition, not a change.**
   `ContractedBy` and core `SpecifiedBy` → `REMOVE` candidates (no producer);
   `ac2_rel_specified_by` → `KEEP` + `RENAME`. **Not applied**: removal moves
   `CoreRelationKind::ALL` 16 → 14 and touches the historical anchor and post-AC1
   baseline pins — an AC1 semantics change, not a behaviour-preserving cleanup.
   It gates A4-1.

## Open follow-ups

| id | P | What |
|---|---|---|
| `FU-A4S0-1` | **P1** | **CLI observation input** (declaration-driven) so a user-facing `why` run can close the leg. Gates A4-1 and the AC10/AC11 producers. |
| `FU-A3-CO-2` | **P2** | `ContractedBy` / core `SpecifiedBy`: emit or remove. **Gates A4-1** before generic Verify consumes the vocabulary. |
| `FU-A3-CO-1` | P2 | canonical relation encoding for `KnowledgePayload::Relation`, then cross-tree edges |
| `FU-A3-CO-3` | P3 | disambiguate the two `SpecifiedBy` names (after a whole-vocabulary inspection) |
| `FU-A3-S15-3` | P3 | `VerifiedBy` targets a spec node rather than evidence |
| `FU-A3-S15-4` | P3 | pin generated text against its condition — **recurred in A4-0 and was caught** |
| `ASC-MA-1` | P3 | root help still describes `architecture` as only the receipt verb |

## Recommendation: can Generic Verify be built on this substrate?

**Yes for the evidence channel; no for the reachable workflow. One structural gap
remains, and it is not architectural — it is an input surface.**

**What is sufficient now (evidence-based):**

| A4-1 needs | Available | Evidence |
|---|---|---|
| a typed subject | `ObservationSubject::{SoftwareRelation, Unit, Contract, Knowledge}` | 21 `observation::` pins |
| an evidence channel | universal `EvidenceRef` on every observation | `acceptance_reuses_the_universal_evidence_ref` |
| deterministic identity to key results on | `RelationId`, `ObservationId`, both clock-stable | `acceptance_relation_id_is_stable_and_content_addressed`, `acceptance_observation_id_is_clock_stable` |
| contradiction-preserving resolution, so `Contradicted`/`Conflicted` are expressible | `EvidenceResolution` | `acceptance_contradictory_evidence_coexists`, `acceptance_no_latest_wins` |
| an explicit gap state, so "no evidence" never reads as a pass | `EvidenceResolution::Insufficient` | `acceptance_missing_relation_stays_unknown` |
| purity, so `evaluate(claim, evidence_set)` is a function | resolution is a pure fn over an immutable set | `acceptance_read_paths_do_not_write` |
| neutrality toward consumers | no Alignment/Authority/provider dependency | `acceptance_no_provider_or_alignment_dependency` |

That is the whole surface a Verify kernel needs to consume claims against
evidence, and it is pinned.

**What is not sufficient:**

1. **`FU-A4S0-1` — no producer can reach the substrate from outside.** The CLI has
   no observation input; `why_cmd` passes `observations: None`. The closure is
   proven at engine level (two pins) but **no user-facing run can exercise it**.
   Building A4-1 on this would give a kernel whose evidence channel is unreachable:
   every real run would be `Insufficient`, and `Insufficient` would look
   indistinguishable from "the substrate does not work".
2. **`FU-A3-CO-2` — the relation vocabulary is unsettled.** Generic Verify will
   consume `CoreRelationKind` at scale. Shipping with two declared-but-unemitted
   variants means the first consumer to interpret them inherits a fossil. This is
   cheap to settle now and expensive after A4-1 builds on it.

**Recommended first slice for A4-1's predecessor**, in this order:

```text
1. FU-A4S0-1  declaration-driven observation input, pinned end-to-end through why
2. FU-A3-CO-2 emit or remove the two core relation kinds (behaviour-preserving,
              fully pinned — the operator's own gate)
3. then A4-1 generic Verify, against a substrate a real run can populate
```

**Confidence:** high on the substrate's shape (every needed capability has a pin),
high on the gap being an input surface rather than a design flaw, moderate on
whether the declaration is the right home for observations — that is the one design
question I deliberately did not decide inside this slice, because A4-1, AC10 and
AC11 are the consumers that should constrain it.
