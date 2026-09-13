# Migration / Cutover Checklist

Use once per deprecated path.

- [ ] semantic owner identified
- [ ] canonical replacement identified
- [ ] all production writers enumerated
- [ ] new writers blocked
- [ ] compatibility adapter is read-only
- [ ] historical data mapping documented
- [ ] dry-run migration exists where data changes
- [ ] migration is idempotent/restart-safe
- [ ] parity fixture compares legacy and canonical reads
- [ ] fresh-repo fixture passes
- [ ] migrated-repo fixture passes
- [ ] rebuild/recovery fixture passes
- [ ] direct dependencies on legacy API blocked
- [ ] removal trigger/version recorded
- [ ] docs/agent assets no longer teach legacy path
- [ ] final dependency graph proves no deprecated production reachability
- [ ] final receipt contains evidence

## Destructive removal gate

Never delete legacy bytes/table/files before:

1. export/recovery test exists;
2. canonical rebuild is proven;
3. migration dry-run reports expected counts/digests;
4. rollback or deterministic re-import is documented.
