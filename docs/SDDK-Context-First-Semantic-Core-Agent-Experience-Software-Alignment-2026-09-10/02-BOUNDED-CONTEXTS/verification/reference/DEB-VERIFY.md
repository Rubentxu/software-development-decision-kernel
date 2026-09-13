# Deb-Verify — whole-project reconciliation

DebVerify owns the whole-project view.

It may question knowledge that Verify has not touched for many cycles, compare architecture globally, revalidate tradeoff assumptions and surface implementation/architecture/knowledge debt.

Deep analysis is progressive and risk-weighted, not "send every file to an LLM".

Output is a new `ProjectKnowledgeBaseline` plus delta from the previous baseline.
