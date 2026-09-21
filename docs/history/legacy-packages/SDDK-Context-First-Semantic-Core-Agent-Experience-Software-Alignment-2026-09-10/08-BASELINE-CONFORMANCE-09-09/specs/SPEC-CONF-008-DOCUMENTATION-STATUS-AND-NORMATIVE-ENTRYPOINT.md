# SPEC-CONF-008 — Documentation Status and Single Normative Entrypoint

## Baseline contract

Closes M9 requirements for one normative roadmap/architecture entry point and resolves implementation/status drift.

## Requirements

- each 09/09 SPEC mirror has status consistent with actual adoption (`accepted`, `implemented`, `superseded`, or project vocabulary equivalent);
- no delivered architecture spec remains marked `proposed` without explicit reason;
- one architecture README points to the currently normative package;
- superseded packages retain history but carry clear banners;
- generated command examples are linked to the command registry tests;
- migration/deprecation register identifies every surviving compatibility symbol and removal trigger;
- architecture docs distinguish 09/09 baseline obligations from 10/09 evolutive requirements;
- no requirement is marked delivered solely because a release gate passed.

## Status model

Recommended minimal lifecycle:

```text
proposed -> accepted -> implemented -> superseded
                    \-> rejected
```

`implemented` requires executable evidence. `accepted` means normative but not yet fully implemented.

## Acceptance

- documentation status audit reports zero contradictions;
- every spec row links to implementation/test evidence;
- only one current normative architecture entry point is advertised;
- stale/superseded roadmap references fail a docs fitness check if presented as current.
