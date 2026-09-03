# Sources and influences

This skill is an original synthesis. It does not reproduce the source texts.

## Netstack3 / Rust systems reasoning

- Joshua Liebow-Feeser, Netstack.FM Interview:
  https://joshlf.com/posts/netstack-fm-ep-10/

Topics incorporated:
- platform-agnostic Core vs platform Bindings;
- synchronous straight-line core with async at bindings;
- reasoning about control flow and resource liveness;
- zero-copy and native Rust layouts;
- variable-length/TLV parsing as a separate layer;
- Safe Transmute ideas;
- formal verification with Kani.

## Agent Skill design

- Agent Skills specification:
  https://agentskills.io/specification
- Agent Skills best practices:
  https://agentskills.io/skill-creation/best-practices
- Matt Pocock, writing-for-agents:
  https://github.com/mattpocock/skills/tree/main/skills/productivity/writing-for-agents
- Matt Pocock, skills:
  https://github.com/mattpocock/skills

Patterns incorporated:
- predictable process over fixed output;
- strong activation description/context pointer;
- progressive disclosure;
- steps with explicit completion criteria;
- concise main SKILL.md;
- one-level references;
- leading concepts;
- positive target behavior;
- deterministic checks where useful.

## OpenCode

- OpenCode Agent Skills documentation:
  https://opencode.ai/docs/skills/
