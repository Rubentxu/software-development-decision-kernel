# Agent Experience architecture

## Goal

Agents should be able to use SDDK correctly on the first attempt while remaining decoupled from internal stores and provider-specific prompt formats.

## Five layers

```text
1. AgentProfile       who/role constraints
2. SkillSet           how to perform a task
3. ContextCapsule     what project facts/knowledge matter
4. EffectiveInstructions  how SDDK expects this task to be performed
5. AgentCommandSurface    exact SDDK commands relevant to this task
```

These feed an `ExecutionRequest`. Authority remains external and evaluates effects at execution time.

## Desired failure mode

When an agent cannot perform an operation, the system should return a typed contract/admission/precondition error. It should not force the model to guess CLI syntax, inspect internal tables or improvise a workaround.

## Design principle

> **Move knowledge from prose into contracts; render prose only at the edge.**
