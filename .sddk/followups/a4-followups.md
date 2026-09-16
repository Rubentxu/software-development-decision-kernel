# A4-cycle follow-ups — living registry

> **Purpose**: this is the live working registry of follow-up items for
> the A4 intelligence-loop work (A4-0 through A4-5 + closeout). It is
> **not** a certified historical receipt — those live in
> `docs/A3-MILESTONE-RECEIPT.md` and the per-cycle artifacts under
> `.sddk/cycles/`.
>
> **Rule**: items in this registry may be promoted to `closed` or
> re-prioritised freely as cycles land. The A3 receipt remains an
> unauthenticated snapshot of that moment and is never rewritten.
>
> Last touched: **2026-09-16** at the opening of A4-3.

## Disposition (snapshot at A4-3 entry)

| ID                       | Origin                        | Priority | Status at A4-3 entry | Why it is what it is |
|--------------------------|-------------------------------|----------|----------------------|----------------------|
| `FU-A3-S15-1`           | A3-S15 receipt                | P2       | **CLOSED** by A4-0   | A4-0 wired `Evidence → observes → SoftwareRelation`. The provenance gap that left this open is gone. |
| `FU-A4S0-1`             | A4-0 receipt                  | P2       | **CLOSED** by A4-0b  | A4-0b made the relation reachable from a real execution (not just type-defined). |
| `FU-A3-CO-2`            | A3 closeout receipt           | P3       | **CLOSED** by v1.169.42 | `ContractedBy` and core `SpecifiedBy` relation kinds removed in the v1.169.42 cleanup because they had zero producers. |
| `FU-A3-CO-1`            | A3 closeout receipt           | P2       | open                 | Relation payload encoding convention still ad-hoc. Likely needs A4-4 (lenses need a uniform shape to compare against intent). |
| `FU-A3-CO-3`            | A3 closeout receipt           | P3       | open                 | After CO-2 the remaining shape is mostly a rename. Revisit before A4-5 if ambiguity remains. |
| `FU-A3-S15-3`           | A3-S15 receipt                | P3       | open                 | `VerifiedBy → spec` provenance direction not yet formalised. Needed before the general WHY engine in A4-5. |
| `FU-A3-S15-4`           | A3-S15 receipt                | P3       | open                 | Convert into a fitness rule (CLI lint / doctor check). Probably A5. |
| `ASC-MA-1`              | Agent experience observation  | P3       | open                 | `sddk --help` and a few sub-`--help` screens have non-uniform structure. Worth a UX pass before A5. |

## Infra flakes observed (not part of A4 scope but tracked)

| ID                                          | Origin                       | Status                     | Disposition |
|---------------------------------------------|------------------------------|----------------------------|-------------|
| `uat_stale_tests::stale_detects_geometry_change` | First observed during `release.sh` run for v1.169.48 | open (one occurrence)      | Passed on isolated re-run. Likely Playwright/server timing race. Must-fix before A5 if it reproduces. |
| `FU-A4-4A-REL-1` | A4-4a acceptance (v1.169.52) | **closed 2026-09-16 (REL-1, v1.169.53)** | `scripts/release.sh` step 9 reported `ok "release $TAG published"` based on `gh release create` exit code but did NOT assert `isDraft=false` nor that asset URLs are under `/v1.169.52/` (vs the draft-only `/untagged-.../` slug). Caught during A4-4a acceptance: initial release was `isDraft: true`, every asset returned HTTP 404 from the public CDN, distribution was broken. Mitigated in-flight via `gh release edit --draft=false` + 90s CDN refresh. **Root-cause fix delivered by REL-1 / v1.169.53:** new step 9b `PublicReleaseGate` re-checks `gh release view` on the live API (`isDraft=false`, `isPrerelease=false`), anchors tag SHA via `git ls-remote origin $TAG` (not `origin/main`), enforces the 9-asset contract, and probes each asset URL (6 × 10 s budget per asset, ~9 min worst-case). Fail-closed. Contract tests in `tests/test_release_public_gate.sh` (10 scenarios + 1 static check; PASS=11/FAIL=0). Handoff `docs/handoff/HANDOFF-2026-09-16-rel-1-public-release-gate-v1.169.53.md`. |
| `FU-A4-4A-STRING-GROUNDING` | A4-4b pre-flight (2026-09-16) | **closed 2026-09-16 (A4-4aR, v1.169.54)** | The A4-4a `applicable_concerns()` reducer collapsed three distinct semantic layers — Applicability, Grounding, Evaluability — into a single Applicable/NotApplicable answer. It used `DecisionRef::render().contains(concern.canonical_tag())` and `ContractId::as_str().contains(...)` to decide applicability (string-grounding), and used `(Pipeline, TemporalCoupling) → NotApplicable` to erase a universal concern. A4-4b would have built a lens kernel on top of that conflation — equivalent to the `false-clean` shape A4-2 (DebVerify) had to correct. **Root-cause fix delivered by A4-4aR / v1.169.54:** reducer signature shrunk to 2 args (`&ProjectIntent`, `&UnitIntent`); `NotApplicableReason` reduced to 4 legitimate scope reasons (`NotInProjectIntent`, `NotInUnitIntent`, `ExplicitlyExcludedByProject`, `ExplicitlyExcludedByUnit`); `ApplicableReason` reduced to a single `ProjectAndUnitIntent`; `DecisionRefs` / `ContractRefs` wrapper structs deleted (zero external callers verified); `ProjectIntent` / `UnitIntent` gain `excluded_concerns: BTreeSet<UniversalConcern>` with `#[serde(default)]`; paradigm-level erasure rule removed; 22 unit tests + 5 integration tests under A4-4aR semantics with explicit falsification pins. `cargo test --workspace --offline` 1101 passed, 0 failed; clippy `-D warnings` clean; `bash tests/test_release_public_release_gate.sh` still PASS=11. Handoff `docs/handoff/HANDOFF-2026-09-16-a4-4ar-applicability-semantics-correction-v1.169.54.md`. Scope contract at `.sddk/cycles/p-63676b11dc0ef88f-a4-4ar-applicability-correction/spec.md`. |
| `FU-REL-1-BACKFILL` | REL-1 close-out (2026-09-16) | **closed 2026-09-16 (backfilled onto origin as part of v1.169.54 chore(release))** | The REL-1 handoff file on origin (`docs/handoff/HANDOFF-2026-09-16-rel-1-public-release-gate-v1.169.53.md`) had two deferred-SHA placeholder rows in its `Pinned reference points` table because the pre-push hook (`githooks/pre-push`) blocks doc-only follow-ups. The backfill landed atomically with the v1.169.54 `chore(release)` commit: both rows now cite `ccecdc723355b0ecc7e037f2266f465165dc59dc` (the REL-1 release SHA) instead of placeholder text. A new "Backfill" block was also added to the handoff recording when and where the backfill occurred. |

## Not yet recorded (to be added as A4-3 / A4-4 land)

- A4-3 specific findings (architectural alignment semantics).
- A4-4 lens registry and first three lens contracts (OO/Functional/ADT).
- A4-5 loop wiring observations.
- Any new debt from A4-CLOSEOUT.
- A4-4a: `Intent + UniversalConcern Foundation` (v1.169.52) — closed; 22/22 tests (18 unit + 4 integration); 4418/4418 workspace; arch-spec-046 part-1 model surface shipped, part-2 deferred to A4-CLOSEOUT per arch-spec-046 §6. The only follow-up is the release-script gate above.
