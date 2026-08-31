---
name: cua-test-orchestrator
description: "Trigger: validate web feature, run CUA test, audit app with Fara, check UI with local multimodal model, test web feature with vision model. Loads the multi-agent CUA test loop that uses Fara 1.5 9B (local via llama.cpp HTTP) + MiniMax-M3 (cloud) subagents."
license: MIT
metadata:
  author: OpenCode
  version: "1.0"
  workflow: cua-test
---

# CUA Test Orchestrator (Fara 1.5 9B)

Multi-agent orchestration that validates web features using **Fara 1.5 9B** (local multimodal model served by llama.cpp on `http://localhost:8082/v1`) as the visual reasoning engine, coordinated by **MiniMax-M3** (cloud).

## What this skill does

For each feature the user wants to validate:

1. The orchestrator calls **`cua-test-scenarist`** (cloud M3) to generate 3-7 verifiable acceptance criteria.
2. For each criterion, the orchestrator calls **`cua-test-runner`** (model: `llamacpp/Fara1.5-9B`) which sends the static asset(s) + the criterion as a multimodal prompt to Fara via HTTP and returns Fara's analysis.
3. The orchestrator calls **`cua-test-judge`** (cloud M3) to synthesize Fara's responses against the rubric.
4. The orchestrator writes `tests/cua/{date}-{feature-slug}/REPORT.md` and `SUMMARY.md`.

## Activation Contract

You are **not** the orchestrator. If this skill loaded into a session where you are the orchestrator, you are **cua-test-orchestrator** (primary M3). Stop reading this skill and follow the algorithm in `agents/cua-test-orchestrator.body.md`.

If you are a subagent that received a `task(...)` call from the orchestrator, follow the **Return Envelope** specified by your agent file (`cua-test-runner`, `cua-test-judge`, or `cua-test-scenarist`). Do not delegate further.

## Pre-flight (orchestrator only)

```bash
# 1. Verify Fara server is up
curl -fsS http://localhost:8082/health || { echo "Fara server down. Run: llm fara"; exit 1; }

# 2. Verify Fara is the active model
curl -fsS http://localhost:8082/v1/models | jq '.data[0].id'

# 3. Create output dir
mkdir -p tests/cua/$(date +%Y-%m-%d)-${FEATURE_SLUG}
```

If any step fails, abort the loop with `status: "server_down"` and instruct the user to run `llm fara` and retry.

## State Machine

```
INIT
  └─→ SCENARIST(feature) → rubric.json
        └─→ for each criterion:
              RUNNER(criterion, assets) → fara_response
                └─→ JUDGE(rubric, all_fara_responses) → verdict.json
        └─→ REPORT.md + SUMMARY.md
  └─→ DONE
```

State is persisted to `tests/cua/.state/CHECKPOINT.md` after each criterion so the orchestrator can resume after interruption.

## Loop parameters

- `MAX_FEATURES_PARALLEL = 3` (features evaluated concurrently).
- `MAX_RETRIES_PER_CRITERION = 3` (if Fara returns empty/garbled content).
- `MAX_TOKENS_PER_FARA_CALL = 200` (deterministic, short responses).
- `TEMPERATURE_FARA = 0`.
- `TIMEOUT_HTTP_FARA = 180_000` (ms).

## Failure modes

| Failure | Detection | Recovery |
|---|---|---|
| Fara server down | `curl /health` fails at preflight | Abort loop, instruct user to run `llm fara`. |
| Fara returns empty content | `usage.completion_tokens == 0` | Retry with reformulated question (max 3). |
| Fara returns `"length"` finish_reason | `choices[0].finish_reason == "length"` | Truncate criterion, retry. |
| Scenarist returns `insufficient_info` | Envelope `status == "insufficient_info"` | Ask user for clarification; do not retry. |
| Judge finds hallucination | `hallucination_detected == true` | Mark criterion as `verdict: "partial"` with `hallucination_reason` set. |

## Output Contract

The orchestrator writes:

- `tests/cua/{date}-{slug}/rubric.json` — ScenarioEnvelope from scenarist.
- `tests/cua/{date}-{slug}/responses.json` — array of FaraRunnerEnvelope from runner.
- `tests/cua/{date}-{slug}/verdict.json` — JudgeVerdictEnvelope from judge.
- `tests/cua/{date}-{slug}/REPORT.md` — human-readable Markdown scorecard.
- `tests/cua/{date}-{slug}/SUMMARY.md` — overall scorecard across all features.

## Hard rules (orchestrator + subagents)

1. **No browser automation** anywhere in the loop. `control-browser`, `playwright-cli`, `node_repl`, `microsoft/fara-cli` are forbidden.
2. **Fara is invoked only by `cua-test-runner`**, only via `POST /v1/chat/completions`, only with `temperature: 0` and `max_tokens: 200`.
3. **All artifacts land in `tests/cua/**` or `docs/cua/**`**. No edits elsewhere.
4. **No subagent may invoke another subagent.** Only the orchestrator dispatches.
5. **Static assets only**: the orchestrator never downloads a URL, never fetches a page, never renders HTML. The user provides everything as files.

## Related references

- `ui-audit-protocol/SKILL.md` — Section "CUA Test Mode" for the Output Contract and Severity Rubric reuse.
- `agents/cua-test-orchestrator.body.md` — full orchestrator algorithm.
- `agents/cua-test-runner.md` — Fara HTTP invocation envelope.
- `agents/cua-test-judge.md` — synthesis envelope.
- `agents/cua-test-scenarist.md` — rubric generation envelope.