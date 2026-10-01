---
name: cua-test-orchestrator
description: "Trigger: validate web feature, run CUA test, audit app with Fara, check UI with local multimodal model, test web feature with vision model. Single-agent loop that uses Fara 1.5 9B (local via llama.cpp HTTP) as the visual reasoning engine and a severity rubric borrowed from ui-audit-protocol."
license: MIT
metadata:
  author: OpenCode
  version: "2.0"
  workflow: cua-test
---

# CUA Test Orchestrator (Fara 1.5 9B)

Validates web features using **Fara 1.5 9B** (local multimodal model served by
llama.cpp on `http://localhost:8082/v1`) as the visual reasoning engine, scored
against the severity rubric of
[`skills/ui-audit-protocol/SKILL.md`](../ui-audit-protocol/SKILL.md).

## One agent, three steps

This loop is **not** a multi-agent pipeline. There is no `cua-test-scenarist`,
no `cua-test-runner` and no `cua-test-judge`: those agent files were never
written, and the skill used to order a delegation to all four. Each role is now
a **step you perform yourself**, in order:

| former role | what it actually needs | now |
|---|---|---|
| scenarist | to write 3-7 acceptance criteria | step 1, you write them |
| runner | to POST an image + a question to Fara | step 2, one `curl` |
| judge | to score responses against the rubric | step 3, you score them |

Nothing here needs a second agent. Only Fara itself is a different model, and
Fara is an HTTP endpoint, not a dispatch.

## Pre-flight

```bash
curl -fsS http://localhost:8082/health          || { echo "run: llm fara"; exit 1; }
curl -fsS http://localhost:8082/v1/models | jq -r '.data[0].id'
mkdir -p "tests/cua/$(date +%Y-%m-%d)-${FEATURE_SLUG}"
```

If the health check fails, abort with `status: "server_down"` and tell the user
to run `llm fara`. Do not fall back to another model: the rubric is calibrated
against Fara's answers.

## The loop

**Step 1 — criteria.** Write 3-7 acceptance criteria for the feature. Each must
be checkable from a **static asset** alone. If the feature's correctness depends
on interaction (a click, a scroll, a state transition), it is not testable here:
write it down with a `skip_reason` and move on. Do not weaken a criterion until
it fits.

**Step 2 — ask Fara, once per criterion.** For each criterion, one request:

```bash
curl -fsS http://localhost:8082/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d @<(jq -n --arg img "$(base64 -w0 "$ASSET")" --arg q "$CRITERION" '{
        model: "Fara1.5-9B", temperature: 0, max_tokens: 200,
        messages: [{role: "user", content: [
          {type: "text", text: $q},
          {type: "image_url", image_url: {url: ("data:image/png;base64," + $img)}}]}]}')
```

Parameters are fixed: **`temperature: 0`, `max_tokens: 200`**, one criterion per
call, `TIMEOUT_HTTP_FARA = 180_000` ms. Do not raise `max_tokens` to get a
longer answer — a 9B model at 200 tokens is the configuration the rubric was
calibrated against, and a longer answer is a different, unvalidated experiment.
`MAX_RETRIES_PER_CRITERION = 3`, and a retry re-asks the same question.

**Step 3 — score.** For each response, decide one of:

| response | verdict |
|---|---|
| satisfies the criterion | `pass` |
| contradicts it | `fail` + severity (`critical` / `warning` / `suggestion`) |
| asserts something absent from the asset | `hallucination` + `hallucination_reason` |
| `usage.completion_tokens == 0` | retry, then `unresolved` |

Then normalize to the standard Output Contract of `ui-audit-protocol`:
findings by severity, recommended tests the user runs **manually** outside this
skill, and the final verdict.

## Envelopes

Persisted verbatim as `tests/cua/{date}-{slug}/`:

- `rubric.json` — `{criteria: [{id, text, skip_reason?}]}`
- `responses.json` — one `FaraRunnerEnvelope` per criterion:
  `{criterion_id, finish_reason, completion_tokens, content}`
- `verdict.json` — `JudgeVerdictEnvelope`:
  `{findings: [{criterion_id, severity, reason}],
    overall_verdict, overall_score, hallucination_detected}`
- `REPORT.md` — human-readable scorecard
- `SUMMARY.md` — rollup across features

`REPORT.md` and `SUMMARY.md` carry the standard Output Contract shape, so a CUA
report and an `ui-audit-protocol` report are read the same way.

## Checkpoint

After each criterion, append its envelope to `tests/cua/.state/CHECKPOINT.md` so
a run resumes instead of restarting. Record only completed criteria.

## Hard rules

1. **No browser automation, ever.** `control-browser`, `playwright-cli`,
   `node_repl` and `fara-cli` are forbidden. This loop cannot see a running
   application, and pretending otherwise produces confident findings about
   pages it never loaded.
2. **Fara only via HTTP**, only with `temperature: 0` and `max_tokens: 200`.
3. **Static assets only.** Never fetch a URL, never render HTML. The user
   provides files. A URL in the output is passed through for the user to open,
   never resolved by you.
4. **All artifacts land under `tests/cua/**`.** No edits elsewhere.
5. **An empty Fara response is `unresolved`, not `pass`.** A criterion nobody
   evaluated has not been satisfied. Reporting it as passed is the failure mode
   this whole design exists to prevent.

## Related references

- [`skills/ui-audit-protocol/SKILL.md`](../ui-audit-protocol/SKILL.md) — the
  Output Contract and Severity Rubric reused unchanged by CUA Test Mode.
