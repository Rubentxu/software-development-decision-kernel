# ADR-010 — Target/Task graph and porcelain CLI

**Status:** Proposed

## Decision

Keep domain `Goal` as user/project intent. Use **Target** for Maven/Gradle-like CLI workflow aggregation to avoid overloading Goal.

A Target resolves to a deterministic Task DAG. Tasks declare dependencies, inputs, outputs, side-effect class, authority requirement, evidence contract, cacheability and retry semantics.

Default user CLI is porcelain; low-level existing commands remain plumbing/compatibility during migration.

## Cache rule

Deterministic local tasks may be up-to-date/cached by declared inputs. LLM outputs are never treated as deterministic build-cache truth; replayed outputs are candidate/evidence requiring the same governance rules.
