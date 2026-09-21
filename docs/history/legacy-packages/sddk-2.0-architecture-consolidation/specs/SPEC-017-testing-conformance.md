# SPEC-017 — Testing and Conformance Strategy

**Status:** Proposed

## 1. Test layers

SDDK SHOULD distinguish:

1. pure domain tests;
2. application/use-case tests against fake ports;
3. adapter conformance tests;
4. CLI contract tests;
5. workflow/replay golden tests;
6. pack compatibility tests;
7. UAT guided-runner integration tests;
8. supply-chain/release smoke tests.

## 2. Determinism

Tests for ledger, replay, projections and receipts MUST avoid live network dependencies. Nondeterministic tools/LLMs use recorded fixtures or deterministic scripted providers.

## 3. Testkit

`sddk-testkit` SHOULD provide builders/macros/fixtures for:

- temporary project identity;
- in-memory/fake ports;
- event stream fixtures;
- pack loading;
- capability denial/approval cases;
- evidence capture;
- fork/replay;
- receipt assertions.

The goal is to stop capability-specific integration tests from reimplementing infrastructure setup.

## 4. Property tests

High-value property tests include:

- projection rebuild equivalence;
- event ordering and hash stability;
- fork shared-prefix identity;
- promotion conflict fail-closed behavior;
- pack enable/disable idempotency;
- capability denial never causing external side effect;
- acceptance immutability.

## 5. Compatibility fixtures

Every schema version that remains supported SHOULD have at least one fixture and round-trip/upcast test.
