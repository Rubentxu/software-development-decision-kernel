# Moldable Views

## Principle
One underlying graph/history; multiple lenses depending on the question.

## Execution lens

```text
Workflow → NodeRun → Attempt → Host → Model → Provider
```

Answers: what ran where, how long, with what result?

## Causal failure lens

```text
Failure → affected Attempt → behavior → circuit → reroute → recovery
```

Answers: why did it fail and how was it recovered?

## Context lens

```text
AgentExecution → ContextCapsule → Artifact/Decision/Requirement
                     ↓
                  actual reads
```

Answers: what information was available/read/stale?

## Cost lens

```text
Workflow → capability → model/provider → usage/cost
```

Answers: where are resources spent?

## Evidence lens

```text
Decision/Acceptance → Evidence → Artifact/Test/Trace → provenance
```

## UAT lens

```text
Requirement → Scenario → Run → Oracle → Human Decision → Signoff
```

## Supply-chain lens

```text
Source commit → Build → Artifact → SBOM → Attestation → Deployment/Release
```

## Architecture lens
A pack may contribute C4/UML/code-knowledge views without changing the control-plane schema. The graph query and view descriptor form the extension boundary.
