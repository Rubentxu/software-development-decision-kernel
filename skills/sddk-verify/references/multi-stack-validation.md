# Multi-Stack Verification Reference

Load only the sections matching changed production/test files. These are
candidate-generation heuristics; adjudicate every hit through reachability,
behavior, evidence, and exemptions before creating a finding.

## Shared Rules

- A placeholder is blocking only when reachable from a required production path.
- A hard-coded value is blocking when it bypasses required input/configuration or
  only satisfies known examples. Constants, fixtures, protocol values, and test
  vectors are not defects by shape alone.
- A mock/fake/in-memory adapter is blocking when the production composition root
  selects it in place of the required real adapter. Test-only wiring is exempt.
- A negative control must fail when the claimed behavior is deliberately broken;
  coverage or a passing happy path alone is insufficient.
- Abstract APIs and intentionally unreachable exhaustiveness guards require
  concrete call-path evidence before classification.

## Rust

Candidates include `todo!`, `unimplemented!`, placeholder panics, unconditional
`Ok(())`, constant `Some/None`, and production `cfg(test)` leakage. Exempt trait
defaults that are unreachable in the selected implementation and explicit
infallible adapters proven by callers. Public contracts use rustdoc (`///`).
Inspect binaries/examples and dependency injection sites for actual wiring.

## Go

Candidates include empty bodies, unconditional `nil`, constant success structs,
panic placeholders, and production constructors returning fakes. Exempt
interface-only declarations and deliberately empty marker methods. Public
contracts use Go doc comments. Trace `main`, constructors, and provider sets.

## Python

Candidates include `pass`, `...`, `NotImplementedError`, constant success
responses, monkeypatched production paths, and dependency defaults selecting
fakes. Exempt abstract methods/protocols and type-checking-only bodies when no
production call can reach them. Public contracts use docstrings. Trace the real
entry point and dependency container.

## TypeScript And JavaScript

Candidates include empty functions, unconditional resolved promises, constant
HTTP success, `throw new Error("not implemented")`, casts that bypass behavior,
and production containers selecting mocks. Exempt type declarations, abstract
members, exhaustive `never` guards, and test fixtures. Public APIs use JSDoc when
documentation is required. Trace exported entry points and production bootstrap.

## Required Evidence

For each confirmed finding record the stable rule ID, exact location, subject,
production reachability, concrete observation, confidence, exemption decision,
and owner phase according to
`prompts/sddk/contracts/verify-finding.schema.json`.
