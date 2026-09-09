# Risk register

| Risk | Impact | Mitigation | Trigger |
|---|---|---|---|
| rewrite spiral | high | strangler adapters + UAT parity | >2 milestones without user-visible value |
| accidental data loss | critical | rebuild/export tests before store removal | any destructive migration |
| new abstraction layer without deletion | high | every new core abstraction names what it replaces | concept count rises after M3 |
| over-generic revision substrate | medium | keep semantic diff/merge in domains | generic API starts knowing Plan/Memory kinds |
| graph becomes authority | high | projection-only ports and rebuild tests | write API introduced on SemanticGraph |
| Vault becomes hidden memory | medium | explicit KnowledgeSource contract | decisions sourced only from Markdown |
| pack SDK too powerful | high | no raw store mutation; conformance tests | pack requires concrete storage |
| CLI churn | medium | compatibility aliases + generated guidance | common scripts break |
| context bloat | high | HOT/WARM/COLD budgets + retrieval receipt | capsule routinely near max tokens |
| LLM cache mistaken for truth | high | replay output marked candidate/evidence | cached result bypasses verification |
| duplicate authority returns | critical | ownership registry + architecture lint | second canonical store declared |


## Agent Experience risks

| Risk | Failure mode | Mitigation / trigger |
|---|---|---|
| CommandRegistry becomes second CLI implementation | parser and registry drift | generate/validate wiring from common typed substrate; golden parity |
| Instruction compiler over-engineering | complex DSL replaces simple useful instructions | start with small typed keys/strength + free-text bodies; spike before extraction |
| Too much agent context | cheat sheets consume tokens and confuse selection | contextual surface budget + relevance UAT |
| Too little command surface | agent cannot recover | explicit recovery contract/refresh + telemetry |
| Skill ecosystem becomes hidden plugin runtime | skills execute arbitrary privileged code | skills are instruction/contracts; executable effects remain Tasks/Capabilities |
| Provider abstraction leaks | core semantics branch by model vendor | provider portability UAT + leaf dependency rule |
| False reproducibility claim | identical hashes imply identical LLM output | explicit replay classes; provenance ≠ deterministic output |
| Asset migration misses embedded prompt strings | stale semantics survive in code | static scanner + inventory + M9 removal gate |
