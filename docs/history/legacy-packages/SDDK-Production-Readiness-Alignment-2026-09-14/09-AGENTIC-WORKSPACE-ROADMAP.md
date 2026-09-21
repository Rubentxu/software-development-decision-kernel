# Agentic Workspace / JCode Roadmap — P0 to P9

This roadmap is a **parallel product track**, not a replacement for the Base A0→A5 convergence sequence.

## Priority policy

Priority is expressed in four bands:

- **P0 / blocker:** must finish before any new architecture track consumes the affected contract.
- **P1 / high:** next product value after the blocker; may run in parallel where dependencies permit.
- **P2 / medium:** important enhancement, but does not block the immediately preceding GA profile.
- **P3 / optional/validation:** proves portability or adds secondary integration paths.

## Global ordering

```text
BASE critical path

A0 authority/storage/registry drift closure          [P0]
  -> A1 C7 baseline conformance receipt             [P0]
  -> A2 context cut + semantic ownership            [P0]
  -> A3 Knowledge + Agent advisory substrate        [P1]
  -> A4 Alignment + Verify + DebVerify              [P1]
  -> A5 BASE_PRODUCTION_READY                       [P0 release gate]

Parallel preparation after A2

J0 external JCode SDK proof                         [P1]
J1 SDDK Agentic Workspace public API/SDK design     [P1]

Post-Base product tracks

J2 -> J6 JCode Core GA                              [P1]
A6 CogniCode Static Enhanced                        [P1 parallel]
A7 Chronos Runtime Enhanced                         [P1 parallel]
J8 advanced JCode host capabilities                 [P2]
J9 second-host portability validation               [P2 before API 1.0]
A8 Full Enhanced convergence                        [P2]
J7 MCP pull surface                                 [P3 optional]
R11 crate split evaluation                          [P3 evidence-driven]
```

The default product priority after `BASE_PRODUCTION_READY` is **JCode Core GA (J2→J6)** because it exposes the already-built SDDK semantics directly in an interactive development workflow. CogniCode and Chronos may proceed in parallel and enhance that workflow, but the JCode integration MUST remain useful in Base mode.

---

## J0 — External `jcode-sdk` boundary proof — P1

**Can start:** after A2 module/ownership cut is stable enough that no spike code needs SDDK internals. It does not need A5.

Build a tiny Rust project **outside the JCode workspace** and pin:

```text
JCode 0.84.0
rev 752df77d3c13fa7a648eda257dbfcb8d3ea5974d
jcode-sdk 0.1.0
Harness API major 1
```

Prove:

1. `JcodeClient::connect` attaches to the shared runtime;
2. list + attach/create session works;
3. `global_events` / `turn_done` is observable;
4. host context can be injected without forcing an assistant reply;
5. `run_structured` returns schema-valid structured data;
6. runtime capability discovery works;
7. SSH/locality APIs can be detected without importing internals;
8. the spike compiles from an external workspace using exact `git rev`;
9. `cargo package`/crates.io constraints are recorded honestly while `jcode-sdk publish=false` remains true.

**Exit:** `JCODE-SDK-SPIKE-RECEIPT.md` records commands, exact revision, supported calls and limitations.

**Do not do:** fork/rename `jcode-sdk`, import internal JCode crates, or create SDDK domain types in the spike.

## J1 — SDDK Agentic Workspace public API/SDK — P1

**Depends on:** A3 contracts stable enough to expose Context/Task/Evidence references; J0 capability evidence.

Define SDDK-owned semantic contracts for:

- `AgenticSessionDescriptor`;
- `SessionBinding`;
- `HostCapabilitySet`;
- `WorkspaceIdentity` / `WorkspaceLocation`;
- `SessionBootstrapContext` / `ContextDelta`;
- normalized `HostEvent` / `WorkspaceChangeSet`;
- `AgentWorkRequest` / `ContributionV2`;
- semantic `HostAction` such as `InterruptSafely`;
- permission mediation result;
- integration receipts and compatibility basis.

Implement an on-demand local integration server/client with fake transport fixtures. The transport may initially be stdio/local socket; semantics remain transport-neutral.

**Exit:** public contract tests + no-host-type-leak fitness tests; fake client/server exercises every stable operation.

## J2 — `sddk-jcode` anti-corruption adapter — P1

**Depends on:** J0 + J1.

Create a separate Rust integration parcel consuming only:

```text
jcode-sdk public API
sddk-agentic-sdk public API
```

Map:

- JCode session -> `AgenticSessionDescriptor`;
- JCode events -> normalized host events;
- SDDK ContextDelta -> host no-reply injection;
- SDDK `InterruptSafely` -> JCode `soft_interrupt` when supported;
- SDDK structured work -> `run_structured`;
- permission state -> host-native permission operation/decorated approval.

**Exit:** adapter can compile and test with neither repository imported as internals/source modules.

## J3 — Activation + SessionBinding — P1

Support companion mode startup/attach:

```text
JCode available
 -> connect shared runtime
 -> resolve workspace/project
 -> create SessionBinding
 -> compile SessionBootstrapContext
 -> inject once
```

Requirements:

- one JCode session is not assumed to be one Run;
- bindings support PROJECT / WORK_ITEM / RUN / TASK / EPHEMERAL;
- transcript remains host-owned;
- SDDK stores only semantic refs/receipts/evidence;
- local/SSH/container/remote locality remains explicit.

**Exit:** restart/reattach fixture reconstructs binding without transcript duplication or guessed Run identity.

## J4 — ContextBridge + KMT relevance — P1

Deliver compact bootstrap and subsequent deltas.

Rules:

- full capsule once at session basis establishment;
- later changes use `ContextDelta`;
- delta contains basis/provenance/relevance reason;
- irrelevant KMT changes cause no host context churn;
- Alignment remains advisory context and never changes effective instruction hash;
- duplicate/reordered delivery is idempotent or explicitly rejected by sequence/basis semantics.

**Exit:** context-spam fixture proves repeated irrelevant edits do not inject messages; advisory hash fixture remains green.

## J5 — Reactive Verify loop — P1

Consume host event stream, not polling.

Pipeline:

```text
host events
 -> EventNormalizer
 -> MaterialityClassifier
 -> coalesced WorkspaceChangeSet
 -> KMT
 -> VerifyPreflight / Verify
 -> optional provider deepening
 -> Knowledge/Alignment
 -> useful ContextDelta
```

Key constraints:

- routine read/grep/list operations are EPHEMERAL;
- material edits/build/test/turn completion can produce evidence/change sets;
- use `turn_done` as normal coalescing boundary;
- no unconditional full repository scan;
- no duplicate Canonical Event Log authority;
- `ledger watch`/`diff-watch` remain operator/debug surfaces, not transport.

**Exit:** event-storm UAT demonstrates bounded verification and at most one useful delta per semantic edit burst.

## J6 — Structured work + JCode Core GA — P1

Add orchestrated mode:

```text
SDDK AgentWorkRequest
 -> attach/create host session
 -> run_structured(return_schema)
 -> ContributionV2
 -> Synthesis/Evidence
 -> AgentExecutionReceipt
```

Core GA requires companion + orchestrated modes, truthful capability negotiation, session binding, context deltas and reactive Verify.

**Artifact:** `JCODE-CORE-GA-RECEIPT.md`.

**GA does not require:** MCP, Chronos, CogniCode, safe interruption, model selection, rewind or SSH beyond truthful NOT_SUPPORTED reporting.

## J7 — MCP pull surface — P3 / optional

Only implement if actual workflows still need agent-initiated pull after J4-J6.

Maximum initial semantic surface: 3–5 operations. Do not mirror the whole CLI/API over MCP.

Candidate pull needs:

- explain current SDDK context/WHY;
- retrieve bounded Decision/Knowledge detail;
- request Verify status/evidence;
- inspect permitted next actions.

**Exit criterion:** each operation has a demonstrated user/agent pull use-case that native push + SDK cannot solve cleanly.

## J8 — Advanced host capabilities — P2

Add selectively according to negotiated host capability:

- safe soft interruption;
- permission bridge OBSERVE/ADVISE/ENFORCE;
- model/reasoning selection;
- compaction/rewind coordination;
- SSH/remote workspace support;
- richer collaboration/swarm requests without creating a second SDDK swarm orchestrator.

**Exit:** unsupported capabilities degrade explicitly; no version-based guessing.

## J9 — Second-host validation — P2 before Agentic API 1.0

Build a fake host plus one thin real adapter spike (OpenCode or Claude Code based on the best stable public integration available at that time).

Purpose is not feature parity. It tests whether the SDDK Agentic Workspace API encodes SDDK semantics rather than JCode implementation details.

Pass conditions:

- no `jcode_*` types/names in generic contracts;
- second host can implement SessionBinding + ContextDelta + StructuredExecution or truthfully report unsupported capabilities;
- host-specific extensions remain namespaced;
- no generic contract needs a breaking redesign solely because JCode-native assumptions leaked inward.

**Artifact:** `MULTI-HOST-PORTABILITY-RECEIPT.md`.

---

## Release profiles

Agentic integration uses independent declarations:

```text
AGENTIC_API_EXPERIMENTAL    J1 contract exists; SemVer <1 allowed to move
JCODE_CORE_GA               J0-J6 green
JCODE_ADVANCED              J8 selected advanced UAT green
MULTI_HOST_VALIDATED        J9 green
AGENTIC_API_STABLE          portability evidence allows 1.0 consideration
```

None of these changes the truth of `BASE_PRODUCTION_READY`, `STATIC_ENHANCED`, `RUNTIME_ENHANCED` or `FULLY_ENHANCED`.

## Scheduling with provider tracks

After A5:

```text
              +-> J2-J6 JCode Core GA --------+
              |                                |
BASE READY ---+-> A6 CogniCode Static --------+--> richer reactive workflow
              |                                |
              +-> A7 Chronos Runtime ---------+
```

J5 consumes enhanced provider evidence only when available/required. Provider teams do not block Base JCode GA.

## Priority changes that require explicit decision

Escalate a lower-priority item only when one of these is evidenced:

- a blocker prevents J0-J6 execution;
- production usage requires a specific advanced host capability;
- provider evidence is REQUIRED by an adopted Task/Policy;
- upstream JCode compatibility break threatens the pinned integration basis;
- security/authority flaw requires immediate reordering.

Novelty or convenience alone is not a reason to interrupt A0-A5.
