# Executable examples as interface tests

## Rule

An example shown to an agent is code. Treat it like code.

Each example has:

```text
id
command + args
fixture
expected exit class
expected machine schema
side-effect mode (none/dry-run/sandbox)
platform constraints
```

## CI

Stable examples run in CI/UAT. Destructive examples use sandbox fixtures. Documentation generation fails if it references missing CommandSpec/example IDs.

## Value

This turns documentation drift into a compile/test failure instead of an LLM hallucination weeks later.
