# CogniCode integration profile

CogniCode contributes static/structural evidence: symbols, call/dependency graphs, impact, complexity, architecture checks, hot paths, dead code and AST structural observations.

SDDK should prefer aggregated RPC use cases such as AnalyzeDelta/AnalyzeScope/AnalyzeImpact. The CogniCode adapter maps them to SDDK Evidence/MetricSample/ImpactSet; it does not leak Ladybug/petgraph/Tree-sitter internals.
