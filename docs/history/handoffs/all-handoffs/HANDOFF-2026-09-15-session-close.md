# SESSION CLOSE — 2026-09-15 · AC track + A3 closeout + A4-0

> Resume point for the next session. Everything below is committed, pushed and
> receipted. **Nothing is half-done.**

## Final state

| | |
|---|---|
| `HEAD == origin/main` | `def2df863424c65833f6651893e2a7d8524de239` (`def2df8`) |
| Working tree | clean |
| Binary / bundle / framework | `1.169.41` |
| Tests | **205 blocks / 4317 passed / 0 failed** |
| Cycles | **7 opened, 7 CLOSED** |
| Carry-over debt | none |

## Releases this session (8 tags)

| Tag | SHA | Cycle | What |
|---|---|---|---|
| `v1.169.34` | `e55cd95` | A3-S12 | `architecture receipt --changed [--base]` + `ChangeBasis` |
| `v1.169.35` | `97e08c7` | A3-S13 | `--contract`, `--out`, receipt id `v2`, honest read surfaces |
| `v1.169.36` | `a6bafe3` | A3-S14 | `architecture findings [--kind]` (AC5 audit inspectable) |
| `v1.169.37` | `7caa113` | A3-S15 | `why architecture` — **superseded** (see below) |
| `v1.169.38` | `e58047c` | A3-S15 | patch: `ContractNotEvaluable` named the wrong cause |
| `v1.169.39` | `9e674da` | A3 closeout | **A3 milestone PASS**, frozen baseline |
| `v1.169.40` | `f9b0d19` | A4-0 | Evidence/Observation/Provenance substrate |
| `v1.169.41` | `4320d4e` | A4-0b | declaration-supplied observations |

## The three things that were actually hard

1. **A3-S12**: `--changed` reported a **silent empty basis** for files that changed.
   Git quotes non-ASCII paths and detects renames, so a changed `café/y.rs` and the
   origin of a `git mv` were both invisible. Fixed with `-z` and `--no-renames`.
   The acceptance suite could not catch it: the fixture used `src/a.rs`.
2. **A3-S13**: the receipt id **ignored the scope**, so global, `--changed` and
   `--contract` runs over one declaration shared an id while reporting different
   verdicts. Pre-existing from A3-S12, and blocking once `--out` made the id the
   artifact's address. Derivation bumped to `id.v2`.
3. **A3-S15 → release recovery**: `ContractNotEvaluable` stated a condition that was
   **false** for one of its two causes, found only in the **post-release smoke** of
   the installed binary. Every gate had passed, because no test asserted the
   *content* of a generated reason — only that a reason existed. Recovered with
   `release.recover`, fixed text-only, shipped as the patch `v1.169.38`.

That third one is the durable lesson, recorded as **`FU-A3-S15-4`**: *pin generated
explanation text against the condition it claims, not against its existence.* It
**recurred** in A4-0 (the `EVIDENCE_TO_SOFTWARE_REASON` text became false the moment
A4-0 provided the edge) and was caught by that same remedy.

## Milestones

**A3: PASS** — `20 PASS / 2 PASS_WITH_COMPAT / 0 unresolved MUST`
(`docs/A3-MILESTONE-RECEIPT.md`). AC4–AC8 are recorded as **early-delivered future
milestones** that do **not** close A4/A5.

**A4-0 + A4-0b: delivered.** The substrate makes a software relation an ADT with a
deterministic identity instead of a rendered string, gives observations a
clock-stable identity excluding timestamps/messages/severities/text/producer, keeps
contradictions first-class (`Affirms|Denies`, `Conflicted`, never latest-wins, never
a confidence number), and projects into the one `SemanticGraphProjection`. A4-0b
makes it **reachable**: the declaration can carry `observations[]`, and `why`
closes the `evidence → software relation` leg end to end.

Normative consolidation: the historical specs used `SPEC-019..034`, which **collide**
with the normative repository. A4 consolidates at `042+`. `arch-spec-042` is
implemented; `043`–`047` are contract-ready. ADRs `0122`–`0124` accepted.

## Resume point — next session

Order matters and is evidence-based (A4-0 handoff §Recommendation):

```text
1. FU-A3-CO-2   (P2)  emit or remove CoreRelationKind::{ContractedBy, SpecifiedBy}
                      -- an AC1 semantics change, so it needs its own decision.
                      It is the GATE before A4-1: generic Verify will consume
                      CoreRelationKind at scale, and two declared-but-unemitted
                      variants are a fossil the first consumer inherits.
2. A4-1         generic Verify, per arch-spec-043, which is already contract-ready.
```

The A4-0 recommendation is now **one item shorter**: the evidence channel is
reachable from a real run, so the remaining blocker is the vocabulary decision, not
the substrate.

Open follow-ups: `FU-A3-CO-2` (P2), `FU-A3-CO-1` (P2), `FU-A3-CO-3` (P3),
`FU-A3-S15-3` (P3), `FU-A3-S15-4` (P3), `ASC-MA-1` (P3).

## Process notes worth keeping

- **`.sddk/cycles` is gitignored** — cycle artifacts are local; they mirror to
  `~/.local/share/sddk/projects/<p>/cycle-artifacts/…`.
- **Transitions release the lease.** Re-acquire before every transition.
- **The pre-push hook** requires a `chore(release): bump version` commit, so
  post-release handoff pushes need the ceremonial empty marker commit.
- **`--help` writes to stderr.** Writing a golden fixture from stdout yields an
  empty file, and **an empty fixture makes the case skip** — a false pass. Hit and
  caught while regenerating `cli_golden`.
- **Flaky toolchain on this mount**: three transient `failed to open object file` /
  `cc` failures. Two cleared on retry; the third was a genuinely corrupted target
  dir, fixed with `cargo clean -p sddk-domain -p sddk-storage`. Not code-related.
- **`sddk dev doctor --prefix <framework-root>`** reads a stale pre-v2 receipt at the
  data-dir root and reports `binary.bundle_coherence: missing` on a healthy
  split-prefix install. Use the real install prefix (`~/.local/bin`). Pre-existing.

## Handoffs (most recent first)

1. `HANDOFF-2026-09-15-a4-0b-observation-input-v1.169.41.md`
2. `HANDOFF-2026-09-15-a4-0-verification-provenance-foundation.md`
3. `HANDOFF-2026-09-15-a3-closed-baseline-v1.169.39.md`
4. `HANDOFF-2026-09-15-a3-s15-why-architecture-and-v1.169.38-release.md`
5. `HANDOFF-2026-09-15-a3-s14-architecture-findings-and-v1.169.36-release.md`
6. `HANDOFF-2026-09-15-a3-s13-architecture-verify-and-v1.169.35-release.md`
7. `HANDOFF-2026-09-15-a3-s12-architecture-changed-and-v1.169.34-release.md`
