# SPIKE-005 — Context Delta for Failover Resume

## Question
Can a second model continue useful work without receiving the full original context or redoing completed investigation?

## Experiment
Attempt #1 processes several must-read objects, writes intermediate structured findings, then fails with provider quota exhaustion.

Context Compiler creates Attempt #2 delta:
- reusable findings;
- completed work;
- pending work;
- changed/stale artifacts;
- negative knowledge;
- previous failure class.

## Compare
A: full cold restart.
B: recovery ContextCapsule delta.

Measure:
- tokens;
- completion time;
- duplicated tool reads;
- output verification;
- missed context.

## Success criteria
B achieves equal/better verification with materially less duplicated context/work, or documents why a specific capability requires a cold restart.
