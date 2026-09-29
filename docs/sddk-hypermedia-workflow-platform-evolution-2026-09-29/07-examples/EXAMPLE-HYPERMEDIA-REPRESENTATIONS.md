# Ejemplos — Representaciones hipermedia

## Step Ready

```json
{
  "representation_version": 1,
  "resource": "sddk://runs/R77/steps/S-design",
  "resource_type": "StepRun",
  "state": "ready",
  "basis": {
    "workflow_revision": "sha256:wf...",
    "context": "sddk://contexts/C91",
    "context_digest": "sha256:ctx...",
    "policy_digest": "sha256:pol..."
  },
  "summary": "Design architecture for durable context recovery",
  "relations": [
    {"rel":"workflow-run","href":"sddk://runs/R77"},
    {"rel":"work-item","href":"sddk://work-items/W12"},
    {"rel":"context","href":"sddk://contexts/C91"},
    {"rel":"observation","href":"sddk://observations/O31"}
  ],
  "actions": [
    {
      "rel":"execute",
      "action_id":"act:SR77:execute:ctx91",
      "target":"sddk://runs/R77/steps/S-design",
      "input_schema":"sddk://schemas/StepExecutionInputV1",
      "output_schema":"sddk://schemas/ContributionEnvelopeV1",
      "required_authority":"write",
      "idempotency":"attempt-scoped",
      "basis":"sha256:ctx..."
    },
    {
      "rel":"expand-context",
      "action_id":"act:C91:expand",
      "target":"sddk://contexts/C91",
      "input_schema":"sddk://schemas/ContextExpandRequestV1",
      "output_schema":"sddk://schemas/ContextFragmentV1",
      "required_authority":"read",
      "idempotency":"pure",
      "basis":"sha256:ctx..."
    }
  ]
}
```

## Step blocked by REQUIRED runtime evidence

```json
{
  "representation_version":1,
  "resource":"sddk://runs/R77/steps/S-verify",
  "resource_type":"StepRun",
  "state":"blocked",
  "basis":{"context_digest":"sha256:ctx102..."},
  "problem":{
    "type":"required-capability-unavailable",
    "capability":"runtime.trace",
    "provider_status":"unavailable"
  },
  "actions":[
    {
      "rel":"retry-provider-resolution",
      "action_id":"act:S-verify:resolve-runtime-trace",
      "required_authority":"read"
    },
    {
      "rel":"request-human-decision",
      "action_id":"act:S-verify:request-decision",
      "required_authority":"read"
    }
  ]
}
```

No aparece `complete` porque no es legal.

## Ambiguous active work

```json
{
  "resource":"sddk://projects/P1",
  "resource_type":"Project",
  "state":"attention-required",
  "problem":{
    "type":"ambiguous-active-work",
    "candidates":[
      "sddk://runs/R10",
      "sddk://runs/R11"
    ]
  },
  "actions":[
    {"rel":"select-run","target":"sddk://runs/R10"},
    {"rel":"select-run","target":"sddk://runs/R11"}
  ]
}
```

## Completed step

```json
{
  "resource":"sddk://runs/R77/steps/S-design",
  "state":"succeeded",
  "relations":[
    {"rel":"decision","href":"sddk://decisions/D33"},
    {"rel":"artifact","href":"sddk://artifacts/A19"},
    {"rel":"next","href":"sddk://runs/R77/steps/S-tasks"}
  ],
  "actions":[
    {"rel":"continue","action_id":"act:S-tasks:enter"},
    {"rel":"inspect-lineage","action_id":"act:S-design:why"}
  ]
}
```
