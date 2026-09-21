# SPIKE-006 — Dynamic Workflow Graph Expansion

## Question
Can SDDK discover N work items during execution, expand the graph deterministically, survive restart and replay the exact same graph without an LLM controlling the scheduler?

## Prototype
1. Start a template with `discover → map → join → verify`.
2. Fake discovery returns 1, 4 and 50 work units in separate cases.
3. Emit ExpansionProposal.
4. Validate node/depth/concurrency budget.
5. Append expansion/node/revision events.
6. Execute fake workers in parallel.
7. Kill process after partial completion.
8. Restart and resume.
9. Rebuild projection from ledger and compare graph digest.

## Acceptance
- exact graph digest after replay;
- no duplicate NodeRun/side effect after restart;
- invalid/over-budget expansion rejected;
- join fires once;
- no LLM needed for scheduler behavior.
