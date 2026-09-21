# SPEC-CONF-007 — Command Registry and Agent Asset Closeout

## Baseline contract

Closes ADR-014, SPEC-015, M7 and M9 removal of handwritten command knowledge and obsolete prompt paths.

## Problem

Current CommandSpec parity is useful, but a hand-curated registry relative to the public CLI parser still permits two maintenance sources. M7/M9 targeted one typed command contract from which agent/human surfaces derive.

## Required end-state

One typed command substrate owns stable public command semantics and generates or validates:

- parser/help metadata;
- machine schema;
- AgentCommandSurface;
- cheat sheets/examples;
- AllowedCommandSet;
- documentation fixtures.

Implementation MAY keep Clap as the syntactic parser, but stable semantics/examples MUST originate from or be mechanically linked to one typed registry with no hand-maintained duplication that can silently drift.

## Requirements

- all stable examples are executable fixtures;
- active AGENTS/prompts/skills/templates contain no deprecated internal command names;
- routine agent tasks do not need exploratory `--help` calls;
- command visibility remains knowledge, never permission;
- contextual command minimization is deterministic and refreshable;
- provider adapters consume semantic ExecutionRequest/AgentCommandSurface, not CLI internals;
- obsolete monolithic prompt paths are removed after parity;
- compatibility renderer, if retained, is generated from new contracts and time-bounded.

## Acceptable implementation strategies

1. CommandRegistry generates Clap command metadata;
2. shared declarative command spec generates both Clap and agent surfaces;
3. compile-time/runtime parity gate proves every public stable command maps exactly once and all examples are registry-owned.

Strategy 3 is only conformant if no independently handwritten authoritative syntax remains.

## Acceptance

- UAT-13..UAT-22 all pass;
- deliberately changing an option without updating contract fails CI;
- repository scan finds no active handwritten authoritative cheat sheet;
- examples execute against installed binary, not only library unit tests.
