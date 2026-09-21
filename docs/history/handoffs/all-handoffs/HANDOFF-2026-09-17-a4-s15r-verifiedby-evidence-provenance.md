# HANDOFF — A4-S15R `VerifiedBy` Evidence Provenance (v1.169.65)

## Certified state

| | |
|---|---|
| **Cycle** | `p-63676b11dc0ef88f/a4-s15r-verifiedby-evidence-provenance` (A-min, **CLOSED**) |
| **Change budget** | `VERIFIEDBY PROVENANCE MODEL CORRECTION` (single) |
| **Released baseline** | `v1.169.64` → `2135e6026cdf69b6041e230714d06374f43f60d1` |
| **Development head at cycle open** | `d0329397435f8d31c7eaff5e7dc2feaea918f0a1` |
| **Workspace version** | `1.169.65` |
| **Actual release tag** | `v1.169.65` |
| **Release SHA** | `38b0a13e940a75bee5177a830122c796a347d592` |
| **HEAD == origin/main** | yes (`38b0a13`) |
| **Tag → SHA** | `v1.169.65` → `38b0a13` |
| **PublicReleaseGate** | **PASS** (draft=false, prerelease=false, 9 assets, local install coherent) |
| **Binary / bundle / current** | all `1.169.65` |
| **Binary sha256** | `ad7e661f36b4712fe8a38433e50958d2c6b660b213a45dc821a4d2fbfdd4a5b8` |
| **ADR** | `ADR-0127-VERIFIEDBY-TARGETS-EVIDENCEREF` |
| **Closes** | `FU-A3-S15-3` (P3) |
| **Unblocks** | A4-5b (does NOT auto-open) |

> §25: release SHA, development HEAD, workspace version, actual tag and
> released baseline are recorded **separately**, deliberately, per
> `INC-A4-RELEASE-VERSION-DRIFT`.

## What shipped

```text
contract --SpecifiedBy--> spec        (declared intent; always emitted)
contract --VerifiedBy-->  evidence     (verification evidence; 0/1..N edges)
```

Before this cycle, `VerifiedBy` pointed at the **spec** node with the
`EvidenceRef`s attached as edge metadata — because no evidence node kind
existed. A3-S15 documented that shape and filed `FU-A3-S15-3`
(“VerifiedBy targets a spec node rather than evidence”) to correct it
later. ADR-0121 §4 confirmed the deferral explicitly. A4-S15R is that
later cycle.

Implementation:

- `ArchitectureOverlayNodeKind::EvidenceRef` (+1 variant; `ALL` 8 → 9).
- `OverlayNodeRef::Evidence(EvidenceRef)` (+ helper
  `evidence_overlay_node_ref`).
- `ArchitectureGraphOverlay::add_contract_metadata` rewritten:
  `SpecifiedBy` always; one `VerifiedBy` edge per typed-unique
  `EvidenceRef` (dedup + canonical order via `EvidenceBundle`); no edge
  for zero evidence; no metadata duplication on the edge.
- Evidence node identity = `EvidenceRef::ordering_key()`
  (`sha256(domain | kind | locator | cas)`); locator =
  `evidence:{ordering_key}`. Typed, deterministic, no string inference.
- CLI `node_tag` renders `Evidence(_)` as `evidence:<kind>:<locator>`.

## §0 consumer preflight (evidence)

Blast radius **zero**. No normative consumer depended on
`VerifiedBy == contract → spec`:

| Site | Class | Notes |
|---|---|---|
| `add_contract_metadata` (overlay.rs) | PRODUCER | The single producer; repointed. |
| `rebuild.rs:43`, `architecture_cmd.rs:263` | PRODUCER | Always passed `&[]` (empty) → no legacy edge emitted. |
| `ac6_ac4_witness_bridge.rs:104`, `architecture_graph/tests.rs:498`, `architecture_why/tests.rs:92`, `architecture_receipt/tests.rs:378` | TEST | All `&[]`. |
| `architecture_why::traverse` | (independent) | ADR-0121 §4: `VerifiedBy → spec` NOT on the WHY path; WHY reads typed `evidence_refs`. |
| `sddk-domain::staleness::verified_by`, `sddk-domain::graph::verified_by` | DEAD (re: this fix) | Different namespace/graph system. |
| `architecture_declaration::validate` + tests | DEAD / TEST | Doc comment / asserts `verified_by` is system-produced, not declarable. |

## §24 Claim → Evidence

```text
OBSERVED:  VerifiedBy targets an Evidence node.
OBSERVED:  SpecifiedBy still targets the Spec node.
OBSERVED:  0/1/N evidence cardinality is correct.
OBSERVED:  rebuild is deterministic (clear + re-project → identical digest).
OBSERVED:  equal locator but different kind/CAS cannot collide.

STRUCTURAL: no Evidence persistence authority added.
STRUCTURAL: no second graph/store added.
STRUCTURAL: no A4-5b/Advisory/WHY feature introduced.

DERIVED:    A4-5b can now explain verification provenance without
            treating SpecRef as evidence.
```

The last line stays **DERIVED** until A4-5b.

## Corpus (21 pins)

`crates/sddk-engine/tests/a4_s15r_verifiedby_evidence_provenance.rs`

- §17 identity falsification: same ref → one node; different kind →
  different nodes; different CAS → different nodes; insertion-order
  independent digest.
- §18 cardinality: 0 / 1 / N; duplicate dedup; VerifiedBy never targets
  a spec node; evidence node carries typed props.
- §19 A3-S15 regression: SpecifiedBy survives with/without evidence;
  two provenance axes distinct; reachability `contract → evidence`.
- §20 A4 regression: anti-encroachment (no AdvisoryContext /
  ContextCompiler / InstructionCompiler / AuthorityEngine /
  IntelligenceLoop in the overlay sources); deterministic +
  clear/re-project stability; enum includes `EvidenceRef`.
- §15 `ArchitectureClaimedBy ≠ VerifiedBy`.
- §16 stale `VerifiedBy(contract→spec)` cannot survive clear+rebuild.

Plus `bonus_overlay_node_kinds_all_nine_have_unique_tags` updated to 9.

## UAT

```text
cargo fmt --check                                   OK
cargo clippy --workspace --all-targets -D warnings  OK
cargo test --workspace                              OK (all suites green)
cargo build --release -p sddk-cli                   OK
```

## Artifacts

- Cycle spec: `.sddk/cycles/p-63676b11dc0ef88f-a4-s15r-verifiedby-evidence-provenance/spec.md`
- Corpus: `crates/sddk-engine/tests/a4_s15r_verifiedby_evidence_provenance.rs`
- ADR: `docs/architecture/adrs/ADR-0127-VERIFIEDBY-TARGETS-EVIDENCEREF.md`
- Spec updates: `docs/architecture/specs/arch-spec-A3-S3-architecture-semantic-graph-overlay.md` (REQ-AC2-014/017)
- A3-S15 handoff addendum: `docs/handoff/HANDOFF-2026-09-15-a3-s15-why-architecture-and-v1.169.38-release.md`
- Follow-up ledger: `.sddk/followups/a4-followups.md` (`FU-A3-S15-3` → CLOSED)
- Roadmap: `docs/architecture/README.md`

## Roadmap delta

```text
before                              after
A4-5a    CLOSED v1.169.64           A4-5a    CLOSED v1.169.64
A4-S15R  CURRENT (IN PROGRESS)      A4-S15R  CLOSED v1.169.65
A4-5b    blocked_by A4-S15R         A4-5b    NEXT (unblocked)
A4-5C    blocked_by A4-5b           A4-5C    blocked_by A4-5b
A4-CLOSEOUT                         A4-CLOSEOUT
A5                                  A5
```

## Exit criteria

- `VerifiedBy` has one unambiguous meaning: `contract → EvidenceRef`. **PASS**
- `SpecifiedBy` has one unambiguous meaning: `contract → SpecRef`. **PASS**
- Evidence is first-class only as PROJECTION. **PASS**
- No parallel evidence authority. **PASS**
- No string inference. **PASS**
- A3-S15 WHY remains behavior-compatible. **PASS**
- `FU-A3-S15-3` is CLOSED. **PASS**
- A4-5b is structurally unblocked. **PASS**

## STOP

A4-5b does **not** auto-open. Its mission (separate cycle): consume
`IntelligenceLoopResult` + `IntelligenceLoopReceipt`, fold that into the
advisory context, and wire WHY/WHY-NOT by reusing existing WHY surfaces
(there is already a WHY engine over `ActiveGraphProjection`) — without
introducing a third explanation authority.
