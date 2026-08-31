# SPEC-010 — Behavioral IR and Agent Identity

**Status:** Proposed

## 1. Problem

Agents, permissions, prompts, skills, manifests and docs can drift because the same behavioral contract is repeated in different formats.

## 2. Behavioral IR

Define a canonical machine-readable representation for an executable behavior/agent contract.

Required fields SHOULD include:

- `id`, `version`, `kind`;
- owner pack;
- subscribed event types/patterns;
- allowed phases/frames;
- read scope/view;
- declared outputs;
- capability requests;
- approval requirements;
- deterministic/agentic class;
- model/provider constraints;
- prompt reference + SHA-256;
- skill references + hashes;
- policy reference + hash;
- budgets/timeouts;
- evidence requirements;
- failure/retry semantics;
- fixture/conformance suite.

## 3. Generated artifacts

The IR SHOULD generate or validate:

- permission registry entries;
- pack inventory;
- runtime registration metadata;
- agent prompt preamble metadata;
- docs/reference pages;
- capability audit inventory;
- compatibility checks.

Human-authored prose remains useful, but deterministic fields should not be hand-copied across multiple files.

## 4. Actor identity

Agent receipts/events SHOULD bind execution identity to content:

```text
agent_id
agent_definition_sha256
prompt_sha256
skill_set_sha256
policy_sha256
model_identity
capability_set_sha256
```

Changing a prompt or policy therefore creates a distinguishable actor version for audit purposes.

## 5. Compatibility

Current Markdown agent files may remain source content initially. The migration can introduce IR sidecars and later decide whether frontmatter becomes the canonical authoring surface.
