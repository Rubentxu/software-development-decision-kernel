# Target Repository Layout

A possible end-state, introduced incrementally:

```text
crates/
  sddk-kernel/
  sddk-app/
  sddk-orchestration/
  sddk-context/
  sddk-ledger/
  sddk-graph/
  sddk-testkit/
  sddk-cli/

  adapters/
    sddk-adapter-sqlite/
    sddk-adapter-fs/
    sddk-adapter-git/
    sddk-adapter-opencode/
    sddk-adapter-mcp/
    sddk-adapter-browser/

  packs/
    sddk-pack-sdd/
    sddk-pack-uat/
    sddk-pack-incident/
    sddk-pack-security/

assets/
  agents/
  schemas/
  routing-policies/

workflows/
  ...

docs/
  adr/
  specs/
```

## Important caveat
Do not create crates solely to mirror this picture. Start as internal modules if boundaries are still moving; extract once dependency direction is stable and compile-time isolation buys something.
