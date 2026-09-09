# Built-in Target catalog

## `change`

Suggested task chain:

```text
identity.resolve
context.compile
planning.reconcile
plan.compile
implementation.apply
verify
assurance
decision.capture
memory.update
```

## `verify`

```text
context.compile
plan.validate
build
test
pack.verification*
evidence.collect
assurance.evaluate
```

## `ship`

```text
status.check
verification.require
policy.release_admission
release.plan
release.apply
receipt.emit
memory.update
```

## `recover`

```text
facts.verify
projections.rebuild_if_needed
memory.resolve_head
run.rehydrate
context.compile
next.compute
```

## `audit`

```text
facts.verify
objects.verify
memory.audit
projection.checkpoints
architecture.rules
policy.receipts
```

Packs may contribute tasks to extension points, e.g. `verify.after_test`, but target resolution remains deterministic and inspectable.
