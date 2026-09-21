# HANDOFF — A3 CLOSED, frozen baseline v1.169.39

> **Milestone A3 is CLOSED.** No A4 work has begun. The next checkpoint starts from
> tag `v1.169.39`.

## Certified state

| | |
|---|---|
| **Milestone verdict** | **A3: PASS** — `20 PASS` / `2 PASS_WITH_COMPAT` / **`0 unresolved MUST`** |
| **Certified semantic revision** | `98b7fc78d39aa1deab4c37a1c0273d20a8bdb388` (`98b7fc7`) |
| **Tag** | `v1.169.39` → `9e674da14156471f6afb6f080774c6adc900f7ac` |
| **Release** | https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.39 |
| **Release / bundle version** | binary `1.169.39`, bundle `1.169.39`, framework `current → 1.169.39` |
| **Artifact digests** | `binary_sha256 = 3a65f5ec…`, `bundle_sha256 = 77bf8443…` (install receipt schema 2) |
| **Final `HEAD == origin/main`** | `9e674da14156471f6afb6f080774c6adc900f7ac` ✓ |
| **Cycle** | `p-63676b11dc0ef88f/a3-closeout-milestone-reconciliation` — **CLOSED** (sequence 14) |
| **Milestone receipt** | `docs/A3-MILESTONE-RECEIPT.md` |
| **Archive manifest** | `.sddk/cycles/p-63676b11dc0ef88f-a3-closeout-milestone-reconciliation/archive-manifest.json` (mirrored to the XDG cycle-artifacts store) |
| **Tests** | **205 blocks / 4291 passed / 0 failed** |

## Semantic preservation (the release condition)

The release preserves the certified milestone **exactly**:

```
git diff --stat 98b7fc7..v1.169.39 -- 'crates/**'   →   empty
```

What differs between the certified revision and the tag, and nothing else:

| Difference | Commit |
|---|---|
| the milestone receipt document | `f310c01` |
| an empty ceremonial version marker (required by `githooks/pre-push`, INC-M7-9) | `5b7cb72` |
| the version bump (`Cargo.toml` + `Cargo.lock`) | `9e674da` |

The ceremonial marker and the version bump are kept clearly separate from the
certified semantic revision, as in the previous cycles.

## The two compatibility scopes (accepted)

### K7 — cross-tree overlay projects nodes, not payload edges

`knowledge::project_into` projects every assertion into the canonical
`InMemorySemanticGraph` type deterministically and rebuildably, each node carrying
`basis_hash`, `knowledge_kind`, `payload_kind` and `declared_at`.

Not delivered: edges decoded from `KnowledgePayload::Relation`. Those bytes are
opaque with an arbitrary `content_type`; decoding them needs a documented canonical
encoding that does not exist. Inventing one would fabricate provenance — the same
reason `why architecture` refuses to synthesise its own missing
`evidence → observes → software` leg.

→ `FU-A3-CO-1` **P2**.

### AC2 — dead core vocabulary and a nominal collision

- `CoreRelationKind::{ContractedBy, SpecifiedBy}` are declared and pinned but
  **emitted by nobody**. The unit↔contract linkage AC2 actually uses is
  `ArchitectureClaimedBy` plus the contract metadata node.
- Core `specified_by` (node → contract) and AC2's `ac2_rel_specified_by`
  (contract → spec document) share a name with opposite directions. Both are
  individually correct — AC2 *must* use extension kinds (REQ-AC2-004) — but the
  shared name is a trap for a reader and for an agent navigating provenance.

Neither is an authority divergence or a behavioural violation of A3.

→ `FU-A3-CO-2` **P2**; `FU-A3-CO-3` **P3**.

## Early-delivered future milestones (recorded, unchanged)

```
AC4 = A3-S5   AC5 = A3-S7   AC6 = A3-S6   AC7 = A3-S8   AC8 = A3-S9
```

**Early delivery does not close A4/A5.** `architecture receipt` is not all of
Verify; `architecture findings` is not all of DebVerify; `paradigm_lens` is not
complete Software Alignment; AC8 is not `BASE_PRODUCTION_READY`.

## Open follow-ups

| id | P | What |
|---|---|---|
| `FU-A3-S15-1` | **P1 if required by generic Verify/DebVerify, else P2** | `evidence → observes → software_relation` — **A4-0 provenance substrate candidate.** Benefits WHY, generic Verify, generic DebVerify, CogniCode and Chronos; must not be implemented as a WHY-specific patch. **OPEN.** |
| `FU-A3-CO-1` | P2 | canonical relation encoding for `KnowledgePayload::Relation`, then cross-tree edges |
| `FU-A3-CO-2` | P2 | dead core vocabulary: emit or remove `ContractedBy` / `SpecifiedBy` |
| `FU-A3-CO-3` | P3 | disambiguate the two `SpecifiedBy` names (preliminary direction: `CONTRACTED_BY` / `SPECIFIED_BY_CONTRACT` vs `DECLARED_BY_SPEC` / `DEFINED_IN_SPEC`, but the decision should follow an inspection of the whole vocabulary rather than intuition) |
| `FU-A3-S15-2` | P3 | **corrected**: the AC2 overlay *does* have a pinned deterministic rebuild (`architecture_graph::rebuild`, REQ-AC2-006); A3-S15's probe tested a weaker path and the original wording overstated the gap |
| `FU-A3-S15-3` | P3 | `VerifiedBy` targets a spec node rather than evidence (no evidence node kind exists) |
| `FU-A3-S15-4` | P3 | pin generated explanation text against its condition, not its existence |
| `ASC-MA-1` | P3 | root help still calls `architecture` "emit the architecture-conformance receipt" |

## What A3 delivered in this closeout (vs what was already there)

**Already present, verified and unchanged:** `KnowledgeAssertion`,
`KnowledgeBasis`, deterministic `BasisHash`, KMT identity/freshness/invalidation,
AC1/AC2/AC3, and all of AC4–AC8.

**Delivered by the closeout:** the knowledge cross-tree projection (K7 nodes),
Software Unit cards (P1–P3), the agent advisory boundary (`advisory_context` +
separated capsule/instruction identity + the non-conversion pins), and the AC2
rebuild/freshness proof.

The advisory invariant is proven against the **real** `InstructionCompiler`:

```text
same EffectiveInstructions + different advisory_context
  ⇒ same effective_instruction_set_hash, different context_capsule_hash
```

## Next: A4 (NOT started)

Agreed preliminary shape — a checkpoint decision, not a commitment:

```text
A4-0  Provenance substrate      Evidence ──OBSERVES──> SoftwareRelation
                                stable identity · basis/freshness ·
                                provenance · rebuildability
A4-1  Generic Verify
A4-2  Generic DebVerify
A4-3  Software Alignment
A4-4  Integration / receipts
```

Rationale recorded: the
`Finding → Contract → Decision/Spec → Evidence -X→ SoftwareRelation` gap already
affects WHY, so building generic Verify and Alignment first would very likely mint
another provisional representation of "this evidence observes this relation" only
to have to converge it later.

And the substrate piece must answer more than "create an `Observes` edge": *what
did this evidence observe, on what basis, with what freshness, which software
relation does it identify, who produced the observation, is it deterministic
OBSERVED or INFERRED, can it be rebuilt, and can it contradict another evidence
without replacing it* — precisely so CogniCode (static), Chronos (runtime) and a
deterministic local analyzer can all produce observations without any of them
becoming authority.

`FU-A3-CO-2` / `FU-A3-CO-3` should be resolved near the start of A4 — not
necessarily inside A4-0, but before generic Verify starts consuming the graph
vocabulary at scale, so the unused relation kinds do not become fossils that
future features read as available.
