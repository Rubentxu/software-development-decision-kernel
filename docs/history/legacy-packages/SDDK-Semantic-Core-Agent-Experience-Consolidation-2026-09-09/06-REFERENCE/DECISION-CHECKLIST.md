# Architecture decision checklist

Before adding or changing a feature, answer:

1. What semantic concept owns this responsibility today?
2. Is the new state Fact, Object, Projection or Ephemeral?
3. What is its single source of truth?
4. Can it be represented by an existing Evidence/Decision/Revision/Graph/Run primitive?
5. Does it introduce another state machine, graph, evidence type, revision system or store?
6. Is it core invariant or Pack/domain extension?
7. Does any side effect route through AuthorityEngine?
8. What old abstraction/path becomes unnecessary?
9. What UAT proves parity and source-of-truth behavior?
10. Can the feature be removed/rebuilt without losing canonical truth?

For agent-facing changes also answer:

11. Is behavior encoded in prompt prose that belongs in Task/Policy/Context/Command contracts?
12. Does the AgentProfile remain provider-independent?
13. Is a new Skill procedural only, with Capability requirements declared separately?
14. Is CLI syntax/examples sourced from CommandRegistry?
15. Are examples executable/tested?
16. Can InstructionCompiler explain inclusion/conflict/precedence?
17. Does AgentExecutionReceipt record all affected contract hashes/refs?
18. Are active prompts/skills free from deprecated internal model/store names?
