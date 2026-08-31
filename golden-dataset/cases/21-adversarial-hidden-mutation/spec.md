# Spec: Config manager with defaults

## Requirement

A `ConfigManager` that loads configuration from a file, applies defaults, and caches the result.

## Why this is adversarial

The code looks clean: small class, injected dependency, tests present. But it has a **hidden mutation bug**: `defaults` is a module-level mutable object that `applyDefaults` modifies in-place. If `load()` is called twice, the second call sees the mutated defaults from the first call. This is hard to spot by reading — the test only calls `load()` once.

This tests whether `debt-coupling-cluster` catches the `module-level-var` + `mutable` signal.
