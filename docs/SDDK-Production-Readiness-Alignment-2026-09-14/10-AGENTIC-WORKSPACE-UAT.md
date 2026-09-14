# Agentic Workspace / JCode UAT

These tests validate the host-integration track independently from Base/provider production-readiness profiles.

Every PASS requires a reproducible command/fixture/receipt and the exact SDDK + integration + host basis.

## A. External SDK boundary

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-001` | Build integration spike outside JCode repository using exact git revision | public `jcode-sdk` is sufficient; no internal JCode crate import | J0 |
| `AW-UAT-002` | Connect to existing shared JCode runtime | `JcodeClient::connect` works without launching a private competing runtime | J0 |
| `AW-UAT-003` | Observe `global_events` and one completed turn | event stream is usable and session identity preserved | J0 |
| `AW-UAT-004` | Inject context without assistant response | host accepts no-reply/background context path | J0 |
| `AW-UAT-005` | Run schema-constrained task | structured result validates against requested schema | J0 |
| `AW-UAT-006` | Package integration crate while upstream SDK is unpublished | packaging limitation is explicit; no fake crates.io-ready claim | J0 |

## B. Generic SDDK API/SDK boundary

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-010` | Compile generic API crate without JCode dependency | zero host-specific type/name leak | J1 |
| `AW-UAT-011` | Fake host negotiates subset of capabilities | unsupported capabilities remain explicit; no version guessing | J1 |
| `AW-UAT-012` | Client/server transport replacement fixture | semantic behavior stable across fake stdio/local transport | J1 |
| `AW-UAT-013` | Alignment advisory enters ContextCapsule | context hash changes; instruction-set hash does not | J1/J4 |
| `AW-UAT-014` | Host action requires safe interrupt but capability missing | result is explicit unsupported/deferred; no emulation by killing session | J1 |

## C. Session binding and locality

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-020` | Attach JCode session to Project only | no synthetic Run is created | J3 |
| `AW-UAT-021` | Same host session moves from WorkItem to Task binding | semantic binding changes with receipt; transcript remains host-owned | J3 |
| `AW-UAT-022` | SDDK restarts and reattaches | binding reconstructs from persisted semantic refs without transcript copy | J3 |
| `AW-UAT-023` | Remote/SSH session reports workspace | logical identity and remote locality stay distinct; no local-path assumption | J3/J8 |
| `AW-UAT-024` | Two sessions share same project but different tasks | ContextBasis/Task refs remain isolated | J3 |

## D. Context delivery

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-030` | New binding bootstrap | one compact `SessionBootstrapContext` is delivered with basis/provenance | J4 |
| `AW-UAT-031` | Irrelevant file change | KMT relevance filters it; zero ContextDelta injected | J4 |
| `AW-UAT-032` | Decision relevant to active task changes | minimal ContextDelta injected with reason/provenance | J4 |
| `AW-UAT-033` | Same delta delivered twice | delivery is idempotent or second delivery deterministically rejected as duplicate | J4 |
| `AW-UAT-034` | Alignment tension changes only advisory context | no instruction mutation and no automatic interrupt | J4 |

## E. Reactive event/materiality loop

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-040` | 100 read/search/list host events | no Canonical Event Log flood; events remain ephemeral/telemetry | J5 |
| `AW-UAT-041` | Rapid repeated edits of one source file before turn end | edits coalesce to one semantic WorkspaceChangeSet | J5 |
| `AW-UAT-042` | Turn completion after source+test changes | at most one normal Verify cycle for coalesced semantic boundary | J5 |
| `AW-UAT-043` | Localized edit | Verify remains delta-scoped; no unconditional full scan | J5 |
| `AW-UAT-044` | Optional CogniCode absent | JCode reactive loop remains functional in Base mode; static deep evidence is NOT_EVALUATED | J5 |
| `AW-UAT-045` | Optional Chronos absent | JCode reactive loop remains functional; runtime evidence is NOT_EVALUATED | J5 |
| `AW-UAT-046` | Provider returns contradictory evidence | contradiction reaches Knowledge/Alignment/ContextDelta without being averaged away | J5 |
| `AW-UAT-047` | Existing `ledger watch` / `diff-watch` unavailable | host integration still works; CLI polling tools are not transport dependencies | J5 |

## F. Attention/interruption semantics

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-050` | Normal Alignment tension | CONTEXTUAL/ATTENTION only; no soft interrupt | J5/J8 |
| `AW-UAT-051` | Active task cancelled | semantic `InterruptSafely` emitted; adapter maps only if supported | J8 |
| `AW-UAT-052` | Active DecisionRef superseded critically | attributable ATTENTION/INTERRUPT according to configured policy | J8 |
| `AW-UAT-053` | External branch/worktree replacement invalidates basis | current execution basis marked stale before further structured contribution is accepted | J5/J8 |

## G. Structured execution

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-060` | SDDK sends AgentWorkRequest with return schema | JCode returns schema-valid `ContributionV2`; no markdown scraping | J6 |
| `AW-UAT-061` | Structured result fails schema | rejected/incomplete result remains visible; no fabricated Contribution | J6 |
| `AW-UAT-062` | Agent contribution conflicts with Decision Memory | Synthesis preserves dissent/contradiction; host result does not directly mutate decision authority | J6 |
| `AW-UAT-063` | Companion and orchestrated mode use same adapter | no duplicate semantic integration implementation | J6 |

## H. Permission mediation

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-070` | No SDDK policy applies | native JCode approval UX remains authoritative | J8 |
| `AW-UAT-071` | SDDK advisory concern exists | one approval flow is decorated; no double approval | J8 |
| `AW-UAT-072` | Explicit SDDK governed action denied | denial is attributable and host action does not proceed | J8 |
| `AW-UAT-073` | ENFORCE mode disabled | advisory result cannot silently become hard host permission denial | J8 |

## I. Versioning/publication

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-080` | JCode product version changes but Harness API/capabilities compatible | runtime negotiation decides compatibility, not version string alone | J2+ |
| `AW-UAT-081` | Harness API major incompatible | adapter reports INCOMPATIBLE and does not guess | J2+ |
| `AW-UAT-082` | Git dependency moves upstream | build remains pinned to exact recorded `rev` until basis intentionally updated | J0+ |
| `AW-UAT-083` | Upstream SDK later published | publication strategy can switch without changing generic SDDK domain contracts | J2+ |

## J. Second-host portability

| ID | Scenario | Expected invariant | Milestone |
|---|---|---|---|
| `AW-UAT-090` | Fake second host implements only ContextInjection + StructuredExecution | generic API remains usable with subset capabilities | J9 |
| `AW-UAT-091` | Thin OpenCode/Claude adapter spike | no JCode names/types required in generic contracts | J9 |
| `AW-UAT-092` | Second host has unique native feature | namespaced extension preserves feature without polluting generic core | J9 |

## K. GA gates

### JCode Core GA

Requires PASS for:

```text
AW-UAT-001..006
AW-UAT-010..014
AW-UAT-020..024 except remote-specific 023 may be NOT_SUPPORTED truthfully
AW-UAT-030..034
AW-UAT-040..047
AW-UAT-060..063
AW-UAT-080..082
```

J8 permission/interruption/model/SSH features are not required for Core GA.

### Agentic API stable / 1.0 consideration

Requires JCode Core GA plus `AW-UAT-090..092` and no unresolved generic-contract leak caused by JCode-specific assumptions.

## Evidence record

```yaml
uat_id: AW-UAT-...
sddk_commit: "..."
sddk_agentic_api_version: "..."
sddk_jcode_commit: "..."
jcode_product_version: "..."
jcode_revision: "..."
harness_api: "..."
negotiated_capabilities: []
fixture: "..."
command_or_test: "..."
result: PASS
result_ref: "..."
notes: "..."
```
