# HANDOFF — A3-S15 `why architecture` provenance traversal + v1.169.38

## Certified state

| | |
|---|---|
| **Cycle** | `p-63676b11dc0ef88f/a3-15-why-architecture` (A-lite, **CLOSED**, sequence 20) |
| **Certified SHA** | `e58047c4bfc04fe206189b36b4f2d41fe359fd14` (`e58047c`) |
| **HEAD == origin/main** | yes |
| **Tag** | `v1.169.38` → `e58047c` |
| **Release** | https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.38 |
| **Binary / bundle / framework** | all `1.169.38` |
| **Install receipt** | schema v2, `binary_sha256 = 28b982eb…`, `bundle_sha256 = 43c78478…` |
| **Supersedes** | `v1.169.37` (stays published; see "The release that failed" below) |
| **ADR** | `ADR-0121-ARCHITECTURE-WHY-TRAVERSAL` (accepted, mirrored to the vault) |

## What shipped

```text
sddk architecture findings            → shadow_authority <finding-id>
sddk why architecture <finding-id>     → a reproducible explanation
sddk why architecture <contract-id>    → the other namespace
```

`why architecture` is READ/EXPLAIN: it evaluates nothing, runs no second audit and
mutates nothing.

## FindingId — the exact basis

```
FindingId = sha256(
    "sddk.architecture_finding.id.v1|"
  | revision                  # declaration's `revision`
  | knowledge_basis
  | contract_set_digest       # AC4's digest, exposed as a public fn and shared
  | kind                      # canonical tag, e.g. shadow_authority
  | subjects…                 # sorted, as the audit orders them
  | contract_ids…             # sorted
)
```

Excluded, each for a measured reason:

| Excluded | Why |
|---|---|
| `message` | can change for a redaction alone |
| `severity` | pure function of `kind`; no discrimination, implies independent variation |
| `now` | the workflow is two invocations; a clock-dependent id could never be fed back |
| `semantic_graph_digest` | **measured clock-dependent** — the overlay embeds each linkage claim's `evaluated_at` |

`contract_set_digest`, `verification_plan_digest`, `revision` and `knowledge_basis`
were measured clock-stable; `semantic_graph_digest` was not, which is why the basis
is built from the stable three.

## New AC2 relations

One: **`SpecifiedBy`** (`ac2_rel_specified_by`), contract anchor → spec node,
emitted **unconditionally** by `add_contract_metadata`.

- Vocabulary: `ArchitectureOverlayRelationKind::ALL` **14 → 15** (pin updated).
- Reason: the `spec:` node always existed but had no edge — the only relation to it
  was the evidence-conditional `VerifiedBy`, which points at the spec node for want
  of an evidence node kind. So `contract → spec` (the declared intent behind a
  contract) was unreachable by traversal.
- `VerifiedBy → spec` is **left untouched**: not on the WHY path, and repointing it
  is an AC2 semantics change with its own cycle.

## Shared semantic pipeline (confirmed)

`receipt`, `findings` and `why` all build **one** `ArchitectureContext`:

```
load_declaration → validate → declare_overlay (project) → run_debverify_audit
```

Confirmed behaviourally (per-kind counts and per-finding kind/severity/contract
arity agree at `--now-ms` 0, 5000 and 99999999) **and structurally**: exactly one
`run_debverify_audit` call site in the workspace, inside `build_context`, and
`why_cmd` neither parses the declaration nor invokes an audit. A future surface that
re-invoked the audit would make the three disagree while each still passed its own
suite.

The AC4 **delta stays caller-specific** on purpose, which preserves A3-S14's
recorded observation that `findings` reports where `receipt` errors out.

## Tests

| | |
|---|---|
| Blocks | **205** (was 204) |
| Tests | **4281 passed / 0 failed** (was 4240) |
| Delta | **+41** |
| New engine | 5 `FindingId`, 1 `SpecifiedBy`, 13 `architecture_why` |
| New e2e | 24 `architecture_why_cli_e2e` |
| Golden surfaces moved | 3, all inspected: agent surface 57 → 59, top-level help +1, `cli_golden` `sddk-help.txt` +1 |

## Unresolved edges surviving

**One**, and it is constant:

```
evidence→observes→software_relation
  the substrate attaches evidence to relations as (provider, reference) metadata;
  there is no evidence node kind and no edge from an evidence reference to a
  software relation, so this leg is UNKNOWN rather than inferred
```

It is present in every answer and is never synthesised from subject or message.

## Follow-ups discovered

| id | P | What |
|---|---|---|
| `FU-A3-S15-1` | **P2** | **`evidence → observes → software_relation` does not exist.** Closing it changes provenance substrate and would benefit `why`, Verify, DebVerify, CogniCode and Chronos. **Explicitly not closed in A3-S15.** |
| `FU-A3-S15-2` | P3 | `SemanticGraphProjection::rebuild_from_canonical` has **no callers anywhere in the workspace** — no AC2 adapter, so the overlay cannot be rebuilt through the intended path. The rebuild probe exercises the rebuild the system actually performs. |
| `FU-A3-S15-3` | P3 | `VerifiedBy` targets a spec node rather than evidence (no evidence node kind exists). |
| `FU-A3-S15-4` | P3 | **Pin generated explanation text against its condition, not its existence.** See below. |
| `OBS-A3-S14-1` | P3 | Contradiction matches a `source_kind` string against an authority subject name. Unchanged; does not block a correct WHY. |
| `OBS-A3-S14-2` | P3 | `findings` reports where `receipt` errors. Unchanged and deliberately preserved by the seam. |

## The release that failed, and why it matters

v1.169.37 was published, then its **installed-binary smoke** found a **false
explanation**:

```
c-proj  kind=projection_only  subject=comp:dup      ← comp:dup IS a declared unit
why_not: ContractNotEvaluable { detail: "… links a contract only when its
          subject is a declared unit" }
```

The subject *was* declared. The real cause is that `projection_only` is not a
unit-scoped kind, so the overlay never links it. An operator triaging that contract
would hunt a missing unit that is not missing.

**Why every gate passed.** The acceptance suite (4279 green) never asserted the
*content* of a generated reason. Probe 5 asserted a reason **existed**. Every other
probe tested a presence or an absence: an edge exists, a leg is unresolved, an id is
stable, a count matches. None tested the truthfulness of generated prose, because
the probe list described properties of the model rather than the content of its
explanations.

Handled with the protocol's own mechanism: `release.recover` (Release → Build) with
`release-failure-evidence.md`, then build gate → verify + debt-verify → the patch
**v1.169.38**. v1.169.37 is not rewritten. The fix is **text-only**; the linking
behaviour was already correct. New pins assert the *wrong* text is **absent**, at
engine and e2e level.

The generalisable lesson is `FU-A3-S15-4`: a test that checks "a reason is present"
cannot tell an explanation from a plausible-looking falsehood — which is precisely
the distinction this cycle exists to make.

## Verification method

Twelve falsification probes were run against the shipped CLI, not the acceptance
suite: clock stability, semantic sensitivity on six axes, order invariance, subject
collapse, cardinality 1→1/1→2/1→3, no invented provenance, resolution by lookup not
syntax, `SpecifiedBy` rebuild stability, assessment-vs-fact, audit-failure branch,
three-surface agreement, and read-only proof. All twelve held; the thirteenth
concern (truthfulness of the reason) was not in the list, and that is where the
defect was.

Two honest limitations recorded rather than papered over:

- a true `CONTRACT_OR_FINDING` collision is **not constructible** (`id = hash(basis(id))`
  needs a fixed point of sha256), so that arm is pinned structurally while
  shape-independence is pinned behaviourally;
- the audit is **total today** (`DebVerifyError` has one variant documented as
  unreachable), so its failure branch is pinned structurally plus the reachable form
  of the same rule.

## Next

1. `FU-A3-S15-1` — `evidence → observes → software_relation` (substrate slice).
2. `plan architecture --move/--dependency` (counterfactual, advisory).
3. A `why-not` surface — the `WhyNotReason` ADT is prepared and unused.
4. `architecture paradigms` / `behavior-map` — deferred since A3-S10.

---

## Addendum — A4-S15R (2026-09-17)

**`FU-A3-S15-3` resolution.** ADR-0121 §4 explicitly deferred the
repointing of `VerifiedBy` from `SpecRef` to `EvidenceRef` to its own
cycle. A3-S15 left the legacy shape in place; A4-S15R (cycle
`p-63676b11dc0ef88f/a4-s15r-verifiedby-evidence-provenance`, v1.169.65)
shipped the deferred correction.

- `VerifiedBy` now targets `OverlayNodeRef::Evidence(EvidenceRef)`.
- `SpecifiedBy` is unchanged (contract → spec, always emitted).
- Evidence is a first-class `ArchitectureOverlayNodeKind::EvidenceRef`
  projection node (rebuildable; no new authority/store/CAS/second graph).
- 19-pin corpus in
  `crates/sddk-engine/tests/a4_s15r_verifiedby_evidence_provenance.rs`
  (falsification, cardinality, A3-S15 regression, A4 regression,
  anti-encroachment).
- A3-S15 WHY behavior is **read-only compatible**: no `why` command,
  no `WhyEngine`, no explanation vocabulary touched.

The deferred item is closed; A4-5b unblocks now that `FU-A3-S15-3` is
resolved. A4-5b does NOT auto-open — its own cycle begins when called.

Cross-references:

- `docs/architecture/adrs/ADR-0127-VERIFIEDBY-TARGETS-EVIDENCEREF.md`.
- `docs/architecture/specs/arch-spec-A3-S3-architecture-semantic-graph-overlay.md`
  (REQ-AC2-014/017 updated).
- `docs/handoff/HANDOFF-2026-09-17-a4-s15r-verifiedby-evidence-provenance.md`
  (cycle closeout).
