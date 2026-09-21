# Proposal: Governed Capability Flow — Phase 3 MUST #2

**Slug**: `governed-capability-flow` · **Cycle**: `c-20260818-105733` · **Anchors**: ADR-008, SPEC-007 §1–§7

## Intent

ADR-008 mandates `Proposal → Policy → Approval → Capability → Verify → Receipt`. `sddk_gateway` has `CapabilityPolicy` + `CapabilityReceipt` but no `Proposal` layer above and no postcondition `Verify` — a capability can persist `Succeeded` without proving its postcondition. MUST #2/#3 require the chain proven on **one** capability and `agent_version_hash` + `behavior_version_hash` bound into the receipt.

## Scope

### In Scope
- `Proposal` (intent, scope, constraints, idempotency_key, expiry, hashes).
- `ProposalPolicy::authorize(&Proposal) -> Decision` (default-deny).
- `Capability` trait + `EvidenceBundleWriteCapability` first impl.
- `Verify` step after `execute`, before finalization; failure ⇒ `Failed(verification_failed)`.
- Extend `CapabilityReceipt` with `agent_version_hash` + `behavior_version_hash`.
- Test: deny ⇒ no effect; allow ⇒ verified receipt.

### Out of Scope
- Human approval (MUST #4), redaction (MUST #5) — deferred.
- Multiple capabilities — just `evidence.bundle.write`.

## Capabilities

> CONTRACT with sddk-spec


### New Capabilities
- `proposal-domain-model`, `proposal-policy-evaluation`, `capability-trait-interface`, `postcondition-verify-step`, `evidence-bundle-write-capability`, `proposal-flow-integration-test` — see Approach for file paths and roles.

### Modified Capabilities
- `capability-receipt-persistence` (sddk_storage): schema gains `agent_version_hash` + `behavior_version_hash` (nullable; required for governed writes)


## Approach

1. `sddk-domain`: pure-data `Proposal`.
2. `sddk-gateway`: `ProposalPolicy` default-deny (non-empty hashes + unexpired + declared capability).
3. `sddk-gateway`: `Capability` trait; first impl writes `EvidenceBundle`, returns `Outcome { digest }`.
4. `sddk-gateway`: `verify()` between `execute` and `finish_effect`.
5. `sddk-storage`: receipt insert accepts new fields; legacy reads with `None`.

## Affected Areas

| Area | Impact |
|------|--------|
| `crates/sddk-domain/src/proposal.rs` | New |
| `crates/sddk-domain/src/models.rs` | Mod |
| `crates/sddk-gateway/src/{proposal_policy,capability,verify}.rs` | New |
| `crates/sddk-gateway/src/gateway.rs` | Mod |
| `crates/sddk-storage/src/lib.rs` | Mod |
| `crates/sddk-gateway/tests/proposal_flow.rs` | New |

## Risks

| Risk | L | Mitigation |
|------|---|------------|
| Migration breaks rows | L | Fields nullable; legacy reads = `None` |
| Verify masks failures | M | Deterministic postcondition (recompute digest) |
| Empty hashes | M | `ProposalPolicy` rejects empty |
| Latency on writes | L | In-memory policy; no I/O hot path |

## Rollback Plan

Revert the additive commit. All new types additive; receipt fields optional. No DB migration. Old `CapabilityGateway::apply` callers unchanged.

## Dependencies

ADR-008 (accepted), ADR-0016 (accepted), SPEC-007 §2.

## Success Criteria

- [ ] `cargo check --workspace` green; baseline preserved.
- [ ] `cargo test -p sddk-gateway --test proposal_flow` passes 3 cases.
- [ ] Denied proposal emits no `EvidenceBundle`, no receipt.
- [ ] Allowed receipt `Succeeded` with both hashes non-empty.
- [ ] Falsified postcondition ⇒ `Failed(verification_failed)`.
- [ ] Non-governed `CapabilityGateway::apply` callers unchanged.
