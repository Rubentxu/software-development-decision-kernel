# SPEC-029 — LLM Alignment Evaluator

LLMs are used for semantic judgement after deterministic evidence.

Input includes previous knowledge, selected scope, evidence digest, ArchitecturalIntent, active tradeoffs, lens set and deepening budget.

Progressive disclosure:

`L0 project -> L1 context -> L2 module -> L3 card -> L4 symbols/graph -> L5 AST/source -> L6 runtime`.

Output:

- typed assertion dispositions;
- AlignmentAssessments;
- suggested evidence/deepening;
- opportunities with alternatives/tradeoffs;
- unknowns/contradictions.

The prompt MUST state that Alignment is advisory. The evaluator MUST NOT emit imperatives as policy facts.
