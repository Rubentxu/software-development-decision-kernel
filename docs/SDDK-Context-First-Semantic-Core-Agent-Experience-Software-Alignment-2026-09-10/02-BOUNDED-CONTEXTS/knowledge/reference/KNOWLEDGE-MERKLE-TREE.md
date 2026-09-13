# Knowledge Merkle Tree

## Why

Agents need cheap answers to "what changed enough to distrust previous knowledge?".

A content-only hash is insufficient; use a fingerprint vector so comment-only changes need not invalidate dependency/architecture conclusions.

```text
repository KRoot
  contexts
    packages/crates
      modules
        files
          optional symbols
```

## Impact overlay

A file can affect another branch via calls/imports/runtime relationships. Those edges live in SemanticGraph. KMT gives structural invalidation; the overlay gives semantic invalidation.

## Basis comparison

`knowledge diff K104 K105` should report source/structure/dependency/behavior/knowledge/intent deltas separately.
