# A5 — BASE_PRODUCTION_READY Hardening Programme

> **Status:** planning complete (`A5-PLAN`, cycle
> `p-63676b11dc0ef88f/a5-plan-base-production-ready`).
> **Certified semantic baseline:** `A4_CERTIFIED` (v1.169.68).
> **Nothing in this directory is implemented.** It is a plan.

A4 certified that SDDK's **semantics are correct**. A5 certifies that those
semantics can **operate reliably in production**.

The living roadmap remains `docs/architecture/README.md`; this directory
is its A5 detail.

## Deliverables

| Document | What it answers |
|---|---|
| [`A5-PRODUCTION-READINESS-CONTRACT.md`](./A5-PRODUCTION-READINESS-CONTRACT.md) | What exactly is `BASE_PRODUCTION_READY`? (falsifiable gates G1–G16, severity rules, A4 semantic freeze) |
| [`A5-RISK-REGISTER.md`](./A5-RISK-REGISTER.md) | Which risks can invalidate it? |
| [`A5-DEBT-DISPOSITION.md`](./A5-DEBT-DISPOSITION.md) | Which debts block it, which don't? |
| [`A5-WORKSTREAM-DAG.md`](./A5-WORKSTREAM-DAG.md) | What is the cycle DAG and change budgets? |
| [`A5-UAT-MATRIX.md`](./A5-UAT-MATRIX.md) | What must be operationally exercised? |
| [`A5-FALSIFICATION-MATRIX.md`](./A5-FALSIFICATION-MATRIX.md) | Which failure modes are tested? (durability, concurrency, authority, security, release) |
| [`A5-RELEASE-CERTIFICATION-PROTOCOL.md`](./A5-RELEASE-CERTIFICATION-PROTOCOL.md) | How is a certified release tied to an immutable revision? |
| [`A5-DEFERRED-POST-BASE.md`](./A5-DEFERRED-POST-BASE.md) | What is explicitly POST_BASE? |
| [`A5-PUSH-CONTRACT-INVESTIGATION.md`](./A5-PUSH-CONTRACT-INVESTIGATION.md) | The ceremonial-release-marker investigation |
| [`../specs/arch-spec-049-sddk-configuration-model-v1.md`](../specs/arch-spec-049-sddk-configuration-model-v1.md) | SDDK Configuration Model v1 — adoption, profiles (bender/cautious/manual), autonomy, human feed; single authority for config keys, precedence and non-overridable laws |

## The definition, in one line

```text
BASE_PRODUCTION_READY
  iff every mandatory gate (G1..G16) has durable evidence
  and zero undisposed blocker.
```

No score. No percentage. No maturity rating.

## Cycles

```text
A5-1  release / distribution / version governance
A5-2  durability / rebuild / recovery
A5-3  concurrency / CAS / authority side-effect races
A5-4  compatibility / deprecation / lints / operator UX
A5-5  reliability / clean-machine / security / operational UAT
A5-C  BASE_PRODUCTION_READY certification
```

Feature freeze applies throughout A5.
