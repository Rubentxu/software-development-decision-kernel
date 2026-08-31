---
name: cognicode-sdd
description: >
  CogniCode code intelligence integration for SDD phases.
  Trigger: Automatic when any sddk-* sub-agent needs codebase analysis,
  impact assessment, safe refactoring, or architecture validation.
  Applies to: sddk-explore, sddk-propose, sddk-design, sddk-tasks, sddk-apply, sddk-verify.
license: MIT
metadata:
  author: rubentxu
  version: "1.0"
---

## Purpose

CogniCode is the code intelligence backbone for SDD phases. It provides
structural understanding of the codebase without reading raw files line by line,
quantified impact analysis before proposing changes, safe refactoring with
preview + validation, and architecture cycle detection.

**CogniCode is enhancement, not a requirement.** If the MCP server is
unavailable, proceed with the standard SDD phase workflow. Never block
on CogniCode.

---

## Critical Rule: build_graph First

Most CogniCode tools require a built graph. At the start of any session
that will use CogniCode, call this ONCE before anything else:

```
cognicode_build_graph(
  directory: "{current_working_directory}",
  strategy: "full"
)
```

- The graph is persisted to `.cognicode/` on disk — subsequent sessions
  can skip `full` and use `build_lightweight_index` for fast symbol updates.
- If speed is critical, use `strategy: "lightweight"` for symbol-only ops;
  upgrade to `"full"` only when impact analysis or hot paths are needed.
- If `build_graph` fails (server unavailable), proceed without CogniCode.

---

## Phase Integration Guide

### sddk-explore — Code Intelligence First

Before reading raw files, build the graph and get a structural view.
This replaces manually opening dozens of files to understand the codebase.

**Step 0 (before investigating):**

```
1. cognicode_build_graph        strategy: "full"
2. cognicode_get_entry_points   compressed: true   → public API surface
3. cognicode_get_leaf_functions compressed: true   → low-level primitives
4. cognicode_get_hot_paths      limit: 10, min_fan_in: 3  → load-bearing functions
```

**For each function or module mentioned in the exploration topic:**

```
5. cognicode_analyze_impact     symbol_name: "{function}"
   → Risk level + impacted files — surface this in the Risks section
6. cognicode_get_complexity     file_path: "{relevant_file}"
   → Functions with cyclomatic > 10 = refactor candidates — note them
```

**For finding relevant symbols by intent (not just name):**

```
7. cognicode_semantic_search    query: "{intent}", kinds: ["struct","trait","fn"]
```

Use `compressed: true` on all explore calls to preserve context window.

---

### sddk-propose — Quantified Risk

Before writing the proposal, get a risk baseline for the change:

```
1. cognicode_analyze_impact     symbol_name: "{main_symbol_to_change}"
   → Include risk_level in the proposal's Risks section
2. cognicode_check_architecture
   → If score < 80, note it as existing technical debt in the proposal
```

Map risk levels to proposal language:
- `Low` → "minimal blast radius"
- `Medium` → "moderate impact, N files affected"
- `High` → "significant impact — recommend incremental approach"
- `Critical` → "wide blast radius — recommend compatibility shim over direct change"

---

### sddk-design — Architecture Validation

Validate design decisions don't introduce technical debt BEFORE writing the design:

```
1. cognicode_check_architecture
   → Baseline score — document in Design Decisions section
   → Score < 80 = flag as existing debt, design must not worsen it

2. cognicode_analyze_impact     symbol_name: "{key_function_in_design}"
   → If High/Critical: reconsider the design approach

3. cognicode_trace_path         source: "{entry_point}", target: "{target_function}"
   → Use to document actual data flow paths in the design
```

Post-design validation rule: the design MUST NOT introduce new cycles.
If `check_architecture` after design shows new SCCs, revise the design.

---

### sddk-tasks — Dependency-Aware Ordering

Use call hierarchy to order tasks correctly — implement dependencies first:

```
1. cognicode_get_call_hierarchy symbol_name: "{function_to_implement}"
                                direction: "outgoing", depth: 2
   → Functions it calls = what must exist BEFORE this task
   → Use this to order tasks so lower-level work comes first
```

For each task group, identify which files will be touched:

```
2. cognicode_find_usages        symbol_name: "{symbol_to_modify}"
   → All call sites = all files the task must update
   → Include this in the task's "Files to touch" list
```

---

### sddk-apply — Safe Refactoring Protocol

**Pre-implementation (before touching any file):**

```
1. cognicode_validate_syntax    file_path: "{file_to_modify}"
   → Confirm it parses before touching it (catches pre-existing issues)

2. cognicode_analyze_impact     symbol_name: "{function_to_change}"
   → Know the blast radius — stop if Critical and not expected
```

**For any rename, extract, or move operation — MANDATORY SEQUENCE:**

```
3. cognicode_safe_refactor      action: "rename|extract|move"
                                preview: true   ← ALWAYS preview first
   → Review affected_files list against the task's expected scope
   → If unexpected files appear: STOP and report to orchestrator

4. cognicode_safe_refactor      action: "rename|extract|move"
                                preview: false  ← Only after reviewing preview

5. cognicode_validate_syntax    file_path: "{modified_file}"
   → MANDATORY after every safe_refactor application
   → If invalid: report as CRITICAL issue, do not continue
```

**After implementing each task:**

```
6. cognicode_find_usages        symbol_name: "{changed_function}"
   → Verify no orphaned callers exist after the change
```

**Never skip the preview step.** `preview: false` without a prior `preview: true`
is a hard violation of this protocol.

---

### sddk-verify — Structural Validation Layer

Run these AFTER the standard spec compliance matrix and test execution,
as an additional structural validation layer:

```
1. cognicode_find_usages        symbol_name: "{each_changed_function}"
   → All callers must still resolve correctly
   → Any caller in a different module that's NOT in the apply-progress
     file list = potential missed update → FLAG as WARNING

2. cognicode_check_architecture
   → Compare score to baseline from sddk-design (if available)
   → New cycle introduced = CRITICAL issue
   → Score drop > 10 points = WARNING

3. cognicode_get_hot_paths      limit: 5, min_fan_in: 3
   → Critical paths must still be reachable
   → Entry point connectivity lost = CRITICAL issue
```

Add a **CogniCode Structural Analysis** section to the verify-report:

```markdown
### CogniCode Structural Analysis

| Check | Result | Notes |
|-------|--------|-------|
| Orphaned callers | ✅ None / ⚠️ {N} found | {files with issues} |
| Architecture score | ✅ {N}/100 (no change) | / ⚠️ dropped {N} pts | / ❌ new cycles |
| Hot paths reachable | ✅ All intact / ❌ {function} disconnected | |
```

---

## Risk Thresholds (all phases)

| `analyze_impact` risk | Action |
|-----------------------|--------|
| `Low` (≤2 symbols) | Apply directly |
| `Medium` (≤5 symbols) | Show impact list, request confirmation |
| `High` (≤10 symbols) | Warn orchestrator, suggest smaller increments |
| `Critical` (>10 symbols) | Recommend compatibility shim — not direct signature change |

---

## Context Efficiency Rules

- Always use `compressed: true` for `get_entry_points`, `get_leaf_functions`,
  `get_file_symbols` — prose summaries save 80%+ context vs. full JSON
- Limit `analyze_impact` to the 3 most critical symbols per phase — not every symbol
- Use `build_lightweight_index` for re-analysis after apply (fast symbol update)
- Use `build_graph strategy: "full"` only once per session unless architecture
  analysis is needed again

---

## Worked Examples by SDD Phase

### Example A — sddk-explore: Onboarding a New Codebase

> "I just cloned this repo. Help me understand what it does, what the main
> entry points are, and which functions are called the most."

```
1. cognicode_build_graph        directory: ".", strategy: "full"
2. cognicode_get_entry_points   compressed: true   → public API surface
3. cognicode_get_leaf_functions compressed: true   → low-level primitives
4. cognicode_get_hot_paths      limit: 10, min_fan_in: 3
```

Interpret:
- **Entry points** → public surface area; start reading here
- **Leaf functions** → pure utility code; safe to ignore initially
- **Hot paths** (high fan-in) → changes here ripple widely; flag in Risks

---

### Example B — sddk-propose / sddk-design: Analyzing Change Impact

> "I'm about to change the signature of `UserRepository::find_by_email`.
> What's the blast radius?"

```
1. cognicode_analyze_impact     symbol_name: "UserRepository::find_by_email"
2. cognicode_get_call_hierarchy symbol_name: "UserRepository::find_by_email"
                                direction: "incoming", depth: 4
```

Interpret:
- `risk_level` values: `Low` (≤2 symbols), `Medium` (≤5), `High` (≤10), `Critical` (>10)
- `impacted_files` → exact list of files the task must update
- If `Critical`: recommend compatibility shim over direct signature change

---

### Example C — sddk-design: Architecture Health Baseline

> "Is there any circular dependency? Give me an architecture health score."

```
1. cognicode_build_graph        directory: ".", strategy: "full"
2. cognicode_check_architecture
```

Score table:

| Score | Meaning |
|-------|---------|
| 100   | No cycles. Clean architecture. |
| 80–99 | Minor cycles, worth noting in design. |
| 50–79 | Significant coupling — design must not worsen it. |
| < 50  | Architecture needs refactoring — flag as existing debt. |

For each cycle found: use `cognicode_trace_path` between the two endpoints
to understand why the dependency exists, then document it in Design Decisions.

---

### Example D — sddk-apply: Safe Rename

> "Rename `calc_total` to `calculate_order_total` everywhere. Make sure
> nothing breaks."

```
1. cognicode_analyze_impact     symbol_name: "calc_total"
   → If High/Critical: warn before proceeding

2. cognicode_find_usages        symbol_name: "calc_total"
                                include_declaration: true
   → Confirm expected scope

3. cognicode_safe_refactor      action: "rename"
                                symbol_name: "calc_total"
                                new_name: "calculate_order_total"
                                file_path: "src/order/calculator.rs"
                                line: 15, column: 7
                                preview: true
   → Review affected_files — confirm with orchestrator if unexpected files appear

4. cognicode_safe_refactor      action: "rename"   preview: false
   → Only after reviewing preview

5. cognicode_validate_syntax    file_path: "src/order/calculator.rs"
   → MANDATORY — if invalid: CRITICAL, stop
```

---

### Example E — sddk-tasks / sddk-verify: Finding Critical Functions

> "Which functions should I never break? Show me the most depended-upon code."

```
1. cognicode_get_hot_paths      limit: 15, min_fan_in: 2
2. cognicode_analyze_impact     symbol_name: "<top_function_from_step_1>"
```

Interpret:
- Top 3–5 by `fan_in` = "never-break" symbols → add as `HIGH RISK` notes in tasks
- For each: call `cognicode_find_usages` and check for `test_` prefixed callers
  to verify test coverage exists

---

### Example F — sddk-apply: Full Refactoring Workflow (multi-step)

> "Refactor `UserService` — it's doing too much. Help me split it safely."

```
Step 1 — Understand current state
  → cognicode_get_file_symbols("src/services/user_service.rs", compressed=false)
  → cognicode_get_outline("src/services/user_service.rs")
  → cognicode_get_complexity("src/services/user_service.rs")

Step 2 — Understand dependencies
  → cognicode_analyze_impact("UserService")
  → cognicode_get_call_hierarchy("UserService", direction="incoming", depth=3)
  → cognicode_get_call_hierarchy("UserService", direction="outgoing", depth=2)

Step 3 — Identify extraction candidates
  Sort methods by cyclomatic complexity.
  Group by domain concern (auth vs. profile vs. notifications).

Step 4 — Execute extractions (one at a time)
  → cognicode_safe_refactor(action="extract", preview=true)
  → [confirm]
  → cognicode_safe_refactor(action="extract", preview=false)
  → cognicode_validate_syntax(file_path)

Step 5 — Move to new files
  → cognicode_safe_refactor(action="move", preview=true)  → [confirm]
  → cognicode_safe_refactor(action="move", preview=false)

Step 6 — Verify final state
  → cognicode_check_architecture()
  → cognicode_find_usages("UserService")   ← should show fewer direct usages
  → cognicode_get_hot_paths()              ← confirm new services have reasonable fan-in
```

## Compact Rules

- ALWAYS call `build_graph` before any other CogniCode tool — it is the prerequisite
- Use `compressed: true` in explore phases to preserve context window
- `safe_refactor preview=true` is MANDATORY before `preview=false` — no exceptions
- `analyze_impact` before any non-trivial change — surface the blast radius
- `validate_syntax` after every `safe_refactor preview=false` — non-negotiable
- `check_architecture` score < 80 = flag as existing debt in proposals and designs
- New cycles after apply = CRITICAL in verify — not a warning
- CogniCode is enhancement, not a requirement — if unavailable, proceed without it
- Never block an SDD phase waiting for CogniCode — report unavailability and continue

---

## Sprint 5 Update (59 tools, June 2026)

### New Graphify-Inspired Tools

| Old approach | New (better) |
|-------------|--------------|
| Multiple file reads | `cognicode_graph_query(question: "what connects X to Y?")` |
| 5 separate calls | `cognicode_project_insights()` — dashboard in one call |
| Manual codebase overview | `cognicode_codebase_map(format: "compact")` — LLM-optimized |
| Manual PR review | `cognicode_review_pr(files: ["src/auth.rs"])` — impact analysis |

### New Analysis Tools

- `cognicode_solid_audit()` — SOLID violations (SRP, OCP, LSP, ISP, DIP)
- `cognicode_graph_diff(baseline_date: "2026-06-01")` — compare graph snapshots
- `cognicode_graph_timeline(days: 30)` — trend data over time
- `cognicode_iac_query(resource_id: "tf:main.tf:aws_instance.web")` — infrastructure graph

### New Edge-Type Tools

- `cognicode_get_type_references(symbol: "UserRepository")` — type annotations
- `cognicode_get_imports(file_path: "src/main.rs")` — file imports
- `cognicode_get_implementors(trait_name: "Repository")` — trait implementations
- `cognicode_get_members(class_name: "UserController")` — class members

### Consolidated Composites (replace multiple old tools)

- `cognicode_smart_search(query, algorithm: "fuzzy"|"ranked"|"idf")` — replaces semantic_search + ranked_symbols + graph_search_idf
- `cognicode_graph_analyze(mode: "scc"|"reduced"|"feedback_arcs")` — replaces graph_condensed + graph_reduced + graph_feedback_arcs
- `cognicode_project_overview(detail: "quick"|"medium"|"detailed")` — replaces smart_overview + auto_diagnose + system_prompt_context + suggest_context
- `cognicode_compare_graph(mode: "diff"|"api"|"quality")` — replaces compare_call_graphs + detect_api_breaks + evaluate_refactor_quality

### sddk-explore Updated Workflow

```
1. cognicode_build_graph              strategy: "full"
2. cognicode_project_insights()       → dashboard overview
3. cognicode_codebase_map(format: "compact") → LLM context
4. cognicode_graph_query(question: "...") → specific investigation
```

### sddk-verify Updated Workflow

```
1. cognicode_solid_audit()            → SOLID analysis
2. cognicode_review_pr(files: [...])  → PR impact
3. cognicode_graph_diff(baseline_date: "...") → compare changes
```
