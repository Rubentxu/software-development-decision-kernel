# AX-S4 — Provider adapter portability

**Status:** COMPLETED 2026-09-11 (v1.168.17)
**Question (SPIKES.md):** run one reviewer profile/task/fixture through two adapters (real or fake-compatible) using identical semantic contracts. Identify provider-specific data that truly belongs outside `AgentProfile`.
**Harness:** `crates/sddk-cli/src/spike_axs4.rs` (4 pinned tests).

## Method

Defined the `InstructionsRenderer` port (the seam SPEC-014 implies with "rendered by a provider adapter into native sections") and two deliberately different fake adapters:

- **`sectioned-fake`**: three-role layout (system / developer / user), Anthropic-OpenAI style, routing fragments by strength tier.
- **`flat-fake`**: single-block body layout.

Fixture: a reviewer task compiled once through the `InstructionCompiler` (invariant `writes-cas`, policy `prefer-bundle`, mandatory task requirement `review-scope`, project mandatory `review-scope-restrict`).

Semantic identity = same ordered (semantic_key, strength, text) triples recovered from either render via `semantic_fingerprint()`.

## Findings

1. **The port works with zero production changes.** Both adapters render byte-different prompts (3 sections vs 1) with **identical semantic fingerprints** and identical fragment order (the compiler's deterministic strength-desc/key-asc order is preserved by construction — adapters consume `sections` as-is and only choose layout).

2. **`AgentProfile` is already clean of provider-specific data.** Its fields are purely semantic ceilings (stability / side-effect / authority / targets + name/description). No model name, temperature, token budget, endpoint, or retry policy leaked in. The test `agent_profile_carries_no_provider_transport_data` pins this shape: adding transport fields to `AgentProfile` will require consciously breaking this test.

3. **Where provider-specific data belongs** (adopted direction): the renderer port's constructor. Transport metadata (model, budget, temperature) is adapter-construction state, not instruction semantics. When real providers land, each adapter takes its own config; the semantic contract (`EffectiveInstructions` → fingerprint) never changes.

4. **Renderer port is promotion-ready.** The spike's trait is directly promotable to production when a second real provider arrives — before that, the single flat text in `EffectiveInstructions` fragments is the de facto contract and no abstraction is warranted (extension discipline: don't build the seam before the second implementation exists).

## Revisit triggers

- Wiring a second real provider (promote `InstructionsRenderer` then).
- Any PR proposing transport fields on `AgentProfile` (the pinning test will fail — by design).
