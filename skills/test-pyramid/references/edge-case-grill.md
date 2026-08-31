# Edge-Case Discovery with `auto-grill-loop`

How to use the `auto-grill-loop` skill (and the `auto-grill-loop-orchestrator` primary agent) to discover edge cases systematically **before** writing tests.

## Why grill for edge cases

LLMs are good at writing tests for the happy path and a few obvious failure modes. They are bad at:

- **Adversarial inputs** (empty strings, unicode, negative numbers, timezones, leap seconds).
- **Concurrency** (race conditions, deadlocks, partial failures).
- **Boundary conditions** (max int, empty collection, single element, all elements).
- **Trust boundaries** (untrusted input crossing into trusted state).
- **Failure cascades** (DB down, cache down, network slow, partial retry).
- **State transitions** (invalid transitions, double-spend, idempotency).
- **Domain-specific weirdness** (auth expiry mid-flow, token replay, currency rounding).

`auto-grill-loop` systematically generates questions about all of these and resolves them against the codebase, docs, and (when needed) the internet.

## When to grill

| Scope | Recommendation |
|---|---|
| Single function, well-known | Inline edge-case list (5–10 items) |
| New module / service | 1 short grill session (≤ 3 passes) |
| New feature (UI + API + state) | 1 medium grill session (3–4 passes) |
| Refactor / coverage audit | 1 full grill session (4–6 passes) |
| Release prep | 1 full grill session + manual review |

## How to write a good grill topic

A bad topic: "test `UserService`". The grill will go wide and shallow.

A good topic includes: **the unit + the goal + the failure modes you care about**.

```
Edge cases, failure modes, and adversarial inputs for
{user_service_fn_or_path} in {stack}.
Focus on: {concurrency | trust boundary | persistence | auth | money | time}.
Constraints: {max iterations, deterministic only, no real network}.
```

Examples of good topics:

- "Adversarial inputs and edge cases for `parse_execution_request` in `crates/execution-engine/src/parser.rs`. Focus on: malformed JSON, missing fields, unicode, very large payloads, JSON with duplicate keys, JSON with null bytes. Constraints: no real network, must run under `cargo test`."
- "Edge cases and failure modes of the payment webhook handler in `src/webhooks/payment.ts`. Focus on: retries, idempotency, partial state, signature verification bypass, replay attacks. Constraints: must run as a unit test with mocked HTTP."
- "Accessibility gaps in the login form on the web-app. Focus on: keyboard navigation, screen reader labels, color contrast, focus management, error announcement."
- "Concurrency bugs in the cache layer at `crates/hodei-cache/src/lru.rs`. Focus on: lock contention, poisoning, eviction during read, double-get races."

## How to invoke the grill

The `test-pyramid-builder` agent has the `auto-grill-loop-orchestrator` agent whitelisted in its `task` permission. To grill:

```
# Option A: Direct invocation (agent → agent delegation)
task(auto-grill-loop-orchestrator, topic="...")

# Option B: Skill load (agent loads the skill and runs the protocol inline)
skill(auto-grill-loop)
# then follow the algorithm in the skill
```

The orchestrator will:
1. Detect prior session state (resume if interrupted).
2. Ask the Interviewer for a batch of QuestionCards.
3. For each: ask the User Proxy → research if needed → challenge → judge → record.
4. Audit coverage after each pass.
5. Continue until COMPLETE / BLOCKED / MAX_PASSES_REACHED.
6. Produce a final report at `{grill-reports-dir}/{date}-{topic}.report.md` (XDG `cycle-artifacts/{cycle_id}/grill/` under SDDK adoption; `docs/grill/` only standalone).

## Reading the report

The final report has these sections (see `auto-grill-loop` skill for the full schema):

- **Accepted decisions** — auto-resolved; high confidence. Map to test cases that assert the resolved behavior.
- **Modified decisions** — auto-resolved with caveats. Map to test cases plus a comment explaining the caveat.
- **Decisions requiring user validation** — DO NOT write tests for these until the user decides. Surface to the user.
- **Rejected alternatives** — use these as the *opposite* of the test: a test that asserts the rejected behavior is rejected.
- **Risks** — surface to the user in the test plan.
- **Proposed CONTEXT.md / ADR patches** — read and apply if the user agrees.

## Mapping grill output to tests

For each accepted decision, write one or more tests:

| Grill output | Test type |
|---|---|
| "empty input returns 400" | Unit test on the validator |
| "concurrent writes serialize correctly" | Integration test with `tokio::join!` |
| "user sees toast on save error" | Playwright E2E with mocked 500 |
| "unauthenticated user is redirected to /login" | Playwright E2E |
| "token expires after 24h" | Unit test with `tokio::time::pause` + advance |
| "decimal rounding uses banker's rounding" | Property test with `proptest` / `fast-check` |
| "screen reader announces form errors" | Component test with `jest-axe` + `toHaveAccessibleName` |

## Limits

- The grill **cannot replace** runtime evidence. If a bug only manifests under load or in production, use the `diagnose` skill + Chronos MCP.
- The grill **cannot invent** requirements. If the user has not specified the behavior, the grill will surface that as a "decision requiring validation".
- The grill is **not a substitute** for manual exploratory testing. Use it as a structured complement.
- The grill can be **wrong**. Treat its output as one LLM's view, not as ground truth. Cross-check critical decisions against docs and the codebase.

## When the grill returns BLOCKED

`BLOCKED` means the grill could not resolve a critical question even with research. This usually means:

- Missing docs / CONTEXT.md.
- Ambiguous domain language.
- A decision that requires a human stakeholder.

**Do not** write tests for the blocked scope. Surface to the user with the full report. After the user resolves, re-grill the resolved parts.

## Re-grilling

Re-grill when:

- The codebase changed significantly (new module, refactor, new dependency).
- A bug was found in production that the original grill did not surface.
- A test failure pattern reveals a gap in the question set.
- After every release (coverage refresh).

Re-grilling is **not** rerunning the same questions. It is a fresh pass that may reuse prior answers but generates new questions for the changed scope.
