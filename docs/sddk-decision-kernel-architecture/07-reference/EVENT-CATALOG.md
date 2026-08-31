# Canonical Event Catalog — Initial Set

This is an initial semantic catalog; payload schemas should live as versioned machine-readable schemas in the implementation.

## Workflow
- `workflow.run.created`
- `workflow.run.started`
- `workflow.run.paused`
- `workflow.run.resumed`
- `workflow.run.completed`
- `workflow.run.failed`
- `workflow.run.cancelled`
- `workflow.node.ready`
- `workflow.node.started`
- `workflow.node.waiting`
- `workflow.node.verification_requested`
- `workflow.node.completed`
- `workflow.node.failed`

## Attempts / agents
- `attempt.created`
- `attempt.started`
- `attempt.interrupted`
- `attempt.failed`
- `attempt.completed`
- `agent.execution.started`
- `agent.execution.completed`
- `agent.execution.failed`
- `agent.session.idle`

## Host/tools
- `host.session.created`
- `host.session.error_observed`
- `tool.execution.started`
- `tool.execution.completed`
- `tool.execution.failed`

## Provider/routing
- `provider.rate_limited`
- `provider.quota.exhausted`
- `provider.authentication.failed`
- `provider.model.unavailable`
- `provider.service.unavailable`
- `provider.health.changed`
- `provider.circuit.opened`
- `provider.circuit.half_opened`
- `provider.circuit.closed`
- `model.route.selected`
- `model.route.invalidated`
- `execution.failover.started`
- `execution.failover.completed`

## Context
- `context.capsule.compiled`
- `context.capsule.invalidated`
- `context.object.read`
- `context.object.stale`
- `context.delta.compiled`

## Supervisor/behaviors
- `behavior.triggered`
- `behavior.completed`
- `behavior.failed`
- `supervisor.signal.created`
- `supervisor.decision.recorded`

## Governance
- `proposal.created`
- `policy.evaluated`
- `approval.requested`
- `approval.granted`
- `approval.denied`
- `capability.granted`
- `capability.executed`
- `capability.verification.failed`
- `receipt.recorded`

## UAT
- `uat.plan.created`
- `uat.plan.approved`
- `uat.scenario.started`
- `uat.scenario.completed`
- `uat.oracle.assessed`
- `uat.human.review.requested`
- `uat.human.decision.recorded`
- `uat.defect.opened`
- `uat.retest.scheduled`
- `uat.retest.completed`
- `uat.signoff.recorded`
- `uat.release_decision.recorded`

## Supply chain
- `artifact.built`
- `artifact.provenance.recorded`
- `artifact.sbom.recorded`
- `artifact.promoted`
- `artifact.deployed`
- `artifact.lifecycle.changed`

## Technical debt (`debt.*` SDD-pack family)
- `debt.report.accepted`
- `debt.incidence.created`
- `debt.incidence.observed`
- `debt.incidence.reprioritized`
- `debt.incidence.deferred`
- `debt.risk.accepted`
- `debt.risk.expired`
- `debt.incidence.resolved`
- `debt.incidence.reopened`
- `debt.fingerprint.aliased`
- `debt.plan.created`
- `debt.plan.overridden`
