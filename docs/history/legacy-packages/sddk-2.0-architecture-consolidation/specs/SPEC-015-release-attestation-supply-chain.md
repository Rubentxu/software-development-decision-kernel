# SPEC-015 — Release, Local Gate Attestation and Supply Chain

**Status:** Proposed

## 1. Preserve existing strengths

SDDK already has mature release mechanics including checksums, SBOM/attestation concepts, cosign-based signing and bundle integrity. The goal is to unify local gate evidence with that supply-chain model, not duplicate it.

## 2. Local-first gate receipt

Target command:

```text
sddk dev check --attest
```

It SHOULD produce a signed/verifiable receipt containing:

- repository remote identity;
- commit SHA/tree hash;
- SDDK version;
- toolchain versions;
- pack/bundle manifest hashes;
- gates executed;
- gate results and timings;
- evidence/artifact hashes;
- actor/machine identity policy;
- timestamp/nonce;
- signature/certificate metadata.

## 3. Remote verification bridge

Cloud CI may remain optional for heavy validation, but protected release/PR paths SHOULD be able to verify the signed local receipt independently. A small remote verifier can check:

- signature/provenance;
- commit binding;
- policy-required gates;
- receipt freshness;
- toolchain/manifest constraints;
- optional spot checks.

This preserves local execution authority without turning trust into a self-asserted local boolean.

## 4. Standard provenance

Where practical, SDDK receipts SHOULD map to in-toto/Sigstore-compatible provenance rather than inventing an unrelated cryptographic envelope.

## 5. Release channels

Introduce channels:

- `stable` — explicitly promoted, strongest evidence;
- `candidate` — release candidate/user-test channel;
- `edge` — recent validated main builds;
- `dev` — local/unpublished iteration.

Side-by-side framework bundles and project version pinning make this low-risk.
