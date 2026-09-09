# SPEC-008 — Authority, policy and governed side effects

## Flow

```text
ActionProposal
→ Policy evaluation
→ AdmissionDecision
→ optional HumanApproval
→ Capability execution
→ postcondition verification
→ Evidence + Receipt
```

## Requirements

- AU-001 default deny for undeclared governed capabilities;
- AU-002 exactly one AdmissionDecision per execution attempt;
- AU-003 denied/awaiting-approval proposals perform no side effect;
- AU-004 irreversible actions require declared authority policy;
- AU-005 receipt references policy snapshot, actor, capability input digest, result and evidence;
- AU-006 pack rules can influence policy but cannot bypass the engine;
- AU-007 `sddk why action:<id>` can explain admission.
