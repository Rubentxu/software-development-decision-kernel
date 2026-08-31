# Entropy Policy

Entropy is a mandatory evidence envelope on SDDK code-change paths, not always heavyweight.

| Context/Risk | Entropy Depth |
|--------------|---------------|
| C0/C1 low risk | Heuristic envelope |
| C1 high ambiguity | Focused connascence + OCP estimate |
| C2 | Affected-area metrics only |
| C3 | Baseline comparison only |
| Critical risk | Full analysis + escalation |

**Entropy reporting across SDDK phases (when `entropy-sdd` skill is available):**
- sddk-explore: Connascence landscape (Protocol A)
- sddk-propose: Entropy budget (Protocol B)
- sddk-design: Information Bottleneck interface check (Protocol C)
- sddk-verify: Design Quality Score + SOLID-Entropy compliance (Protocol D)
- sddk-archive: Entropy trend (Protocol E)

**Inject `entropy-sdd` compact rules** in sub-agent prompts when context_quality ≤ C2 or recommended_effort ≥ deepen.
