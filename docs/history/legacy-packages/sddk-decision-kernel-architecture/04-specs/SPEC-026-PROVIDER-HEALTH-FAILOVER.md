# SPEC-026 — Provider Health & Failover

**Status:** Proposed

## Error taxonomy

```text
ProviderFailure
├── RateLimited
├── QuotaExhausted
├── AuthenticationFailed
├── AuthorizationFailed
├── ModelUnavailable
├── ServiceUnavailable
├── Timeout
├── TransportFailure
└── UnknownProviderFailure

ExecutionFailure
├── ContextOverflow
├── InvalidStructuredOutput
├── HostCrashed
└── ToolProtocolFailure

TaskOutcome
├── Success
├── VerificationFailed
├── TestsFailedAsFinding
└── DomainRejected
```

Do not conflate these categories.

## Circuit breaker key
Recommended granularity:

```text
(provider, credential_route, optional model)
```

Weekly quota may be credential-specific; model unavailability may be model-specific.

## States
`Closed -> Open -> HalfOpen -> Closed` plus `Disabled`.

## Sample policy

| Failure | Same-route retry | Open circuit | Reroute |
|---|---:|---:|---:|
| timeout | 1-2 | after threshold | yes |
| 503 | backoff | threshold | yes |
| short 429 | retry-after | maybe | yes if deadline |
| weekly quota | no | immediately | yes |
| invalid auth | no | disable route | yes |
| model unavailable | no | model route open | yes |
| context overflow | no | no | recompile context first |

## Unknown quota visibility
If a provider does not expose remaining weekly allowance, SDDK records **observed usage** and `unknown` remaining budget. It must not invent a percentage. Once a terminal quota error arrives, the route is opened/disabled according to policy.

## Recovery handoff
Before reroute:
- checkpoint known progress;
- compile delta ContextCapsule;
- include previous Attempt failure class, completed/pending work and reusable artifacts.

## Acceptance demo
Inject a deterministic fake `QuotaExhausted` for Attempt #1 and prove Attempt #2 uses another healthy route and completes the same NodeRun.
