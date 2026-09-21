# Proposed CLI Surface

## Workflows

```bash
sddk workflow list
sddk workflow start sdd.full --goal "..."
sddk workflow status wf-123
sddk workflow pause wf-123
sddk workflow resume wf-123
sddk workflow cancel wf-123
```

## Sessions/executions

```bash
sddk execution show at-123
sddk provider status
sddk route explain nr-123
```

## Journal / explanations

```bash
sddk journal --since 4h
sddk why workflow wf-123
sddk why node nr-123
sddk why decision dec-123
sddk why route route-123
```

## Cockpit

```bash
sddk cockpit build
sddk cockpit open
sddk cockpit watch
```

## Replay/fork

```bash
sddk replay wf-123 --verify-projections
sddk fork wf-123 --at evt-999 --model-policy local-first
sddk diff wf-123 fork-abc
```

## Architecture

```bash
sddk check-arch
sddk check-arch --strict
```

## Packs

```bash
sddk pack list
sddk pack validate packs/sddk-incident
sddk pack doctor
```

## UAT

```bash
sddk uat plan ...
sddk uat run ...
sddk uat retest ...
sddk uat signoff ...
```

## Technical debt

```bash
sddk debt validate --report debt-report.json --format json
sddk debt evaluate --run wf-123 --report-artifact art-123 --at 2026-08-21T10:00:00Z
sddk debt queue --project project-123 --scope crates/sddk-engine --format json
sddk debt plan --project project-123 --scope crates/sddk-engine --select INC-000123
sddk debt plan --project project-123 --scope crates/sddk-cli --defer INC-000124="outside current scope"
sddk debt accept-risk --incidence INC-000123 --owner team-core --reason "..." --expires-at 2026-09-21T00:00:00Z --approve
sddk debt why INC-000123
sddk artifact inventory --project project-123 --format json
```

Mutating debt commands require an idempotency key and return a receipt or
artifact reference. Verdicts, priorities and expiry are derived by Rust policy;
the CLI does not accept caller-computed outcomes.

Command names are proposals; preserve existing CLI compatibility aliases during migration.
