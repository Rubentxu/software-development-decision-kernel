---
id: arch-spec-049-sddk-configuration-model-v1
title: SDDK Configuration Model v1 — adoption, profiles, autonomy, human feed
status: implemented
milestone: A5
implemented_by: >-
  `sddk_engine::orchestration_config` (single resolver),
  `sddk config resolve|laws|profiles|set|clear` (crates/sddk-cli/src/config_cmd.rs),
  `~/.config/sddk/profiles/*.yaml` (default/bender/cautious/manual);
  `~/.jcode/bin/sddk-mode` + `~/.jcode/bin/sddk-config` are delegation shims
adrs: [ADR-0129-SINGLE-CONFIG-RESOLVER]
depends_on: []
---

# arch-spec-049 — SDDK Configuration Model v1

> **This document is the single authority for SDDK configuration.**
> The prompt overlay and any tool MUST reference it instead of defining
> key names, precedence, or laws locally. Adding an option one-by-one to
> the overlay is prohibited; extend this model instead.

## §1 Two orthogonal axes

Adoption and behaviour are **different questions** and MUST NOT be mixed:

```text
adoption mode :  ON | OFF | UNDECLARED      "does this project/workspace use SDDK?"
profile       :  <name> | - (inherit)        "if it uses SDDK, how should it work?"
```

## §2 Identity index (`mode-index`) — format v1

`~/.local/share/sddk/mode-index`, one entry per line:

```text
# <id> <mode> [profile]
p-733fb505b5a6bd2d on bender
w-403ce06c96cf4c5adffd451d on bender
p-a912bd...        off -
```

- `<id>` is `project_id` (`p-*`) or `workspace_id` (`w-*`), from
  `sddk project resolve`. Path and name never constitute identity (L7).
- `[profile]` is optional. **Backward compatible**: a two-column line
  (`<id> <mode>`) means `profile = -` (inherit the user default).
- `-` means *inherit*; it is not a profile name.

Deliberately **not** migrated to a rich format while the model is young: a
flat, hard-to-misread index is an asset. Overrides live in profiles
(§5), not in this file.

## §3 Precedence

```text
hard system laws            (non-overridable)          §5
        ↓
workspace declaration       (mode-index, id = w-*)
        ↓
project declaration         (mode-index, id = p-*)
        ↓
profile (selected)          extends → default profile
        ↓
built-in defaults           (mirrored by profiles/default.yaml)
```

Adoption resolution laws (L1..L8, normative):

```text
L1  workspace declaration > project declaration
L2  absent declaration   = UNDECLARED, nunca OFF
L3  invalid declaration  = UNDECLARED, nunca OFF
L4  unknown identity     = UNDECLARED, nunca OFF
L5  duplicate identity   = UNDECLARED, nunca ganador arbitrario
L6  el contenido del repo NUNCA influye en el modo
L7  path y nombre NUNCA constituyen identidad
L8  la perdida del indice no puede desactivar el paraguas en silencio
```

A workspace with no entry under a project `ON` is **not** UNDECLARED in
effective terms: it inherits `ON`, so the adoption question is **not**
asked (`O2`).

## §4 Profiles

Location (editable configuration): `~/.config/sddk/profiles/<name>.yaml`
Generated state stays in `~/.local/share/sddk/`.

Format: **flat dotted keys**, one per line, `key: value`, `#` comments.
Nested YAML is a v2 concern; flat keys are unambiguous and trivially
verifiable.

```yaml
profile: bender
version: 1
extends: default
autonomy.auto_advance: true
human_feed.stage_completion: detailed
```

`extends` builds an inheritance chain (cycles and missing bases fail
closed). `default` is always the implicit base.

Presets shipped in v1: `default`, `bender`, `cautious`, `manual`.

| Preset | Autonomy | Auto-stage | Parallelism | Stage report |
|---|---|---|---|---|
| `bender` | high | yes | up to 4 agents | `detailed` |
| `cautious` | medium | stage gates | up to 2 agents | `detailed` |
| `manual` | none | no (human drives) | no | `normal` |
| `default` | none | no | no | `normal` |

`bender` is the recommended profile for autonomous development.

## §5 Laws — non-overridable

These keys are **not policy**. The resolver imposes them above every
profile; a profile that tries to set one is **rejected** (fail closed).

```text
git.push                          human_gate
git.tag                           human_gate
git.release                       human_gate
git.history_rewrite               human_gate
security.destructive_actions      human_gate
security.irreversible_external    human_gate
security.secrets_exposure         human_gate
evidence.unverified_green         forbidden
evidence.invented                 forbidden
evidence.classes                  mandatory
adoption.absent_is                undeclared
adoption.invalid_is               undeclared
adoption.unknown_is               undeclared
adoption.duplicate_is             undeclared
adoption.silent_disable           forbidden
```

Forbidden keys (their presence rejects the profile outright):

```text
allow_unverified_green  invent_execution_evidence  ignore_human_gate
suppress_evidence       push_without_permission    disable_human_feed
```

A profile may only be **hardenable**, never relaxing: e.g. a profile may
not set `git.push: allowed`. (A stricter-than-law value is conceivable in
a future version; v1 rejects any law key in a profile, which is the safe
default.)

## §6 Key namespace (v1)

`consumption` says who honors the value: `resolver` = the value is
machine-resolved and reported; `overlay` = the prompt contract requires
the agent to honour it. `status: implemented` means it is resolved today.

### Adoption / mode (implemented)

| Key | Type | Default | Consumption |
|---|---|---|---|
| `mode` | `on\|off\|undeclared` | `undeclared` | resolver + overlay |
| `profile` | profile name | `default` | resolver + overlay |

### Autonomy (implemented)

| Key | Type | default | bender | Consumption |
|---|---|---|---|---|
| `autonomy.auto_advance` | bool | `false` | `true` | overlay |
| `autonomy.campaign_mode` | bool | `false` | `true` | overlay |
| `autonomy.stop_only_on_human_gate` | bool | `false` | `true` | overlay |
| `autonomy.reversible_decisions` | `autonomous\|confirm` | `confirm` | `autonomous` | overlay |

Definitions: `auto_advance` = do not ask for confirmation between stages
of the selected workflow; `campaign_mode` = run the whole path without
intermediate human gates; `reversible_decisions` = decide reversible
things from evidence instead of asking.

### Personality (implemented — presentation only)

| Key | Type | default | bender |
|---|---|---|---|
| `personality.preset` | `neutral\|bender\|cautious` | `neutral` | `bender` |
| `personality.tone` | `neutral\|technical_irreverent\|formal` | `neutral` | `technical_irreverent` |
| `personality.humor` | `none\|light` | `none` | `light` |
| `personality.verbosity` | `compact\|normal\|detailed` | `normal` | `detailed` |

Personality affects **presentation only**. It never changes evidence
classes, STOP rules, honesty, or the git-truth law.

### Human feed (implemented)

| Key | Type | default | bender |
|---|---|---|---|
| `human_feed.enabled` | bool | `true` | `true` |
| `human_feed.stage_completion` | `compact\|normal\|detailed\|forensic` | `normal` | `detailed` |
| `human_feed.important_findings` | `immediate\|stage_end` | `stage_end` | `immediate` |
| `human_feed.routine_progress` | `silent\|concise` | `concise` | `concise` |
| `human_feed.persist` | bool | `true` | `true` |

A feed is **not** an approval request. `human_feed.persist` requires the
per-stage report to be durable so a new session can reconstruct
`workflow / current stage / completed stages / evidence / gates / next`.

### Workflow (implemented)

| Key | Type | default | bender |
|---|---|---|---|
| `workflow.selection` | `auto\|suggest_then_lock\|manual` | `manual` | `suggest_then_lock` |
| `workflow.automatic_stage_progression` | bool | `false` | `true` |
| `workflow.persist_stage_state` | bool | `true` | `true` |

### Parallelism (implemented)

| Key | Type | default | bender |
|---|---|---|---|
| `parallelism.enabled` | bool | `false` | `true` |
| `parallelism.max_agents` | int ≥ 1 | `1` | `4` |
| `parallelism.require_file_ownership` | bool | `true` | `true` |
| `parallelism.integrate_through_orchestrator` | bool | `true` | `true` |

### Verification (implemented)

| Key | Type | default | bender |
|---|---|---|---|
| `verification.characterization_first` | bool | `false` | `true` |
| `verification.falsify_new_guards` | bool | `false` | `true` |
| `verification.require_named_red_cause` | bool | `true` | `true` |
| `verification.mutation_check_for_guards` | bool | `false` | `true` |

A guard is only validated if the mutation **reaches** it, it **fails**,
and it fails **for the cause it claims to detect**. A build broken before
the guard does not count.

### Git / documentation / planning / research / recovery (implemented)

| Key | Type | default |
|---|---|---|
| `git.local_commit` | bool | `true` |
| `documentation.update_specs` | bool | `true` |
| `documentation.update_adrs` | `always\|when_needed\|never` | `when_needed` |
| `documentation.update_roadmap` | bool | `true` |
| `documentation.create_receipts` | bool | `true` |
| `planning.stage_size` | `small\|medium\|large` | `medium` |
| `research.characterization_first` | bool | `false` |
| `recovery.resume_from_state` | bool | `true` |

`git.push` / `git.tag` / `git.release` / `git.history_rewrite` are **laws**
(§5), not profile keys.

### Reserved (specified, not yet resolved)

These namespaces are reserved so we do not invent keys ad hoc later. The
resolver does **not** emit them yet; adding one requires extending this
spec **and** the resolver together.

```text
persistence.*      where stage state / receipts / findings live
observability.*    logs, traces, event artefacts
cost.*             agent budgets, analysis depth, expensive checks
tooling.*          preferred/forbidden tools, sandbox, containers
interaction.*      when to ask / report / continue
security.*         beyond the §5 laws
```

## §7 Resolver contract

```bash
sddk-mode                       # MODE=<...> REASON=<...> PROFILE=<...>
sddk-mode set on|off [--project|--workspace] [--profile <name>]
sddk-mode clear    [--project|--workspace]

sddk-config resolve [--json]    # effective view with SOURCE per key
sddk-config get <key>           # effective value of one key
sddk-config profiles            # available profiles
sddk-config laws                # non-overridable laws + forbidden keys
```

`resolve` MUST print the **source** of every value, so no configuration is
magic. Source labels: `system-law`, `profile:<name>`, `<reason>` for the
`mode`/`profile` rows. Unknown key → exit 2, never a silent default.
`--json` emits `{key: {value, source}}`.

`REASON` vocabulary (never implicit): `index-absent`, `no-entry`,
`declared:<scope>`, `duplicate:<scope>`, `invalid-value:<scope>`,
`resolve-failed`.

## §8 Falsification

```text
~/.jcode/bin/sddk-mode-selftest     14 pins  (adoption L1..L5, writer O4/O5, resolver contract)
~/.jcode/bin/sddk-config-selftest   25 pins  (profile column, backward compat,
                                              precedence + sources, JSON integrity,
                                              fail-closed: law key / forbidden key /
                                              extends cycle / missing profile)
```

Required cases: OFF yields an inert profile; a two-column legacy entry
still resolves; a law key in a profile is rejected; a forbidden key is
rejected; an `extends` cycle is rejected; a declared-but-missing profile
is rejected.

## §9 Core law

```text
AUTONOMOUS PROGRESS  +  HUMAN OBSERVABILITY  +  EXPLICIT HUMAN GATES
```

Never `AUTONOMOUS PROGRESS + HUMAN SILENCE`, and never
`HUMAN OBSERVABILITY + APPROVAL AFTER EVERY STEP`.

## §10 Workflow state ≠ conversation state

The workflow belongs to SDDK; the conversation is only a human interface.
`conversation ends ≠ workflow ends`. A new session MUST be able to
reconstruct the selected workflow, the current stage, completed stages,
evidence, commits/checkpoints, gates and next work **without** the chat
history. `human_feed.persist` and `workflow.persist_stage_state` are the
keys that make this true; the concrete storage shape is
`persistence.*` (reserved).
