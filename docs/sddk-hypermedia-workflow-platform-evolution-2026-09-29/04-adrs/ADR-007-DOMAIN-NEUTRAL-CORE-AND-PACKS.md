# ADR-007 — Domain-neutral core with domain packs

**Status:** Proposed

## Context

The product began around software development, but the kernel primitives—workflows, evidence, decisions, context, authority, capabilities—are broader. Future workflows such as creating a book should not require a fork of the kernel.

## Decision

Keep core contracts domain-neutral. Move software-specific StepDefinitions, workflows, augmentation profiles and capability mappings into a Software Pack by strangler. Validate the boundary with a second experimental Authoring/Book Pack.

## Consequences

- software remains first-class but not hardwired;
- generic contracts are pressure-tested by a second domain;
- pack SDK becomes a product surface and must remain narrow;
- premature generic abstractions can be detected by Book UAT.

## Rejected

- rename/refactor everything generic before a second consumer;
- keep all software phases in core forever;
- plugins that can bypass kernel storage/authority.
