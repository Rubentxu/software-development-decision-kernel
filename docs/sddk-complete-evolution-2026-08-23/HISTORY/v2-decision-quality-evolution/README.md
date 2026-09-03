# SDDK Final Evolution — Engineering Assurance & Governed Continuous Improvement

**Baseline:** `Rubentxu/sddk-framework` `main`, SDDK 1.37.1, reviewed 2026-08-23.  
**Status:** Final proposal for a future SDDK evolution cycle.  
**Product identity preserved:** **Software Development Decision Kernel**.

## Executive decision

This proposal deliberately rejects the temptation to turn SDDK into a generic autonomous-research platform, a Rust framework, or a self-modifying agent.

The evolution is accepted only where it strengthens the product's existing jobs:

1. make better software-engineering decisions;
2. preserve why those decisions were made;
3. verify decisions with evidence;
4. coordinate humans, agents and deterministic tools safely;
5. learn from real executions;
6. improve decision workflows empirically without giving an LLM unilateral authority.

The evolution therefore has **two aligned vertical capabilities**.

### A. Engineering Assurance

A composable domain pack that turns architecture, systems, performance and verification reasoning into evidence-backed assessments usable by SDD, UAT, Incident, Security and future packs.

### B. Governed Continuous Improvement

An extension of the existing **Evaluation Feedback + Workflow Laboratory** direction. SDDK learns from its Event Ledger, proposes improvements to skills/prompts/routing/context/workflows, evaluates candidates through fork/replay/diff and holdouts, then promotes only through policy, bounded rollout and receipts.

It is **not** a new generic `autoresearch` product.

## Product-fit rule

Every future feature must answer:

```text
Which SDDK job-to-be-done does this improve?
Which existing primitive does it extend?
Where does it belong: kernel, pack, capability, skill, profile, adapter or projection?
What authority does it own?
What evidence proves it adds value?
How is it removed/reverted if it does not?
```

If these questions cannot be answered, the feature does not enter the roadmap.

## Recommended adoption sequence

```text
EA-0  contracts/skills
EA-1  deterministic assurance core
EA-2  Rust profile dogfooding
EA-3  SDD bridge
GCI-0 experience projection + baselines
GCI-1 candidate experiments in Workflow Laboratory
GCI-2 governed promotion
GCI-3 assisted candidate generation
GCI-4 optional search strategies only after evidence
EA/GCI graph + cockpit lenses
multi-pack + multi-language proof
```

Population search, MCTS, GEPA-like optimization and lineage-based resource allocation are **late optional strategies**, not architectural foundations.

## North-star statement

> SDDK should become better at making, verifying and improving software-development decisions — not better at accumulating unrelated AI features.
