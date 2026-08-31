---
name: entropy-sdd
description: >
  Entropy-based design analysis for SDD phases. Provides connascence metrics,
  SOLID entropy verification, Information Bottleneck interface checks,
  and Design Quality Score computation.
  Trigger: Mandatory in all SDD phases. The LLM always produces entropy metrics.
  When CogniCode is available: quantitative estimation using call graphs and
  architecture analysis. When not: qualitative estimation using code reading heuristics.
  Applies to: sddk-explore, sddk-propose, sddk-design, sddk-verify, sddk-archive.
license: MIT
metadata:
  author: rubentxu
  version: "1.0"
---

## Purpose

Entropy-sdd is the **informational analysis layer** for SDD. It quantifies design
quality using information theory: coupling as mutual information I(A;B), cohesion
as free energy F, LSP compliance as KL divergence, and interface quality as
Information Bottleneck optimization.

**entropy-sdd is MANDATORY.** Every phase that references an entropy protocol
MUST execute it. There is no "skip if unavailable" option.

When CogniCode is available, use it as quantitative foundation for I(A;B) estimations.
When not available, use code reading heuristics — the output format is identical,
only the confidence level differs.

---

## Activation Model

### When CogniCode is available (preferred path):

| Metric | CogniCode Tool | Derivation |
|--------|---------------|------------|
| I(Name) | `find_usages` count | log2(usage_count) |
| I(Type) | `get_call_hierarchy` depth | log2(depth) |
| I(Meaning) | `semantic_search` for undocumented assumptions | Qualitative |
| H(Δ_existing) | `analyze_impact` files modified | log2(files_modified) |
| Architecture health | `check_architecture` score | (100 - score) / 100 |
| Coupling | Hot paths fan-in | fan_in / max_fan_in |

### When CogniCode is NOT available (fallback path):

| Metric | Estimation Method | Accuracy |
|--------|-----------------|----------|
| I(Name) | Count files referencing a symbol manually → log2(N) | ±1 bit |
| I(Type) | Count shared type users → log2(N) | ±1 bit |
| I(Meaning) | Check for undocumented shared assumptions | Qualitative |
| H(Δ_existing) | Count files in change scope → log2(N) | ±2 bits |
| Cohesion | Ratio of related vs total methods per module | Qualitative |
| KL(LSP) | Compare subtype behavior against contract | Qualitative |

Both paths produce the **same output format**. Every report MUST include:
- The metric value (in bits, probability, or 0-1 score)
- The estimation method: `method: CogniCode` or `method: heuristic`
- The threshold comparison: OK / WARNING / CRITICAL

---

## Foundation: Information-Theoretic Metrics

### H(X) — Shannon Entropy

```
H(X) = -Σ p(x) · log2 p(x)

H = uncertainty in bits. A coin flip: H = 1 bit.
A die roll: H = log2(6) ≈ 2.58 bits.
```

### I(A;B) — Mutual Information

```
I(A;B) = H(A) + H(B) - H(A,B) = H(A) - H(A|B) = H(B) - H(B|A)

I = information shared between components.
I(A;B) > 0 means knowing A reduces uncertainty about B.
This is CONNASCENCE — the fundamental coupling measure.

I(A;B) = 0: components are independent (ideal).
I(A;B) > 4 bits: severe coupling — refactor.
I(A;B) > 6 bits: critical coupling — imminent redesign needed.
```

**Example:**
```
Module A exports function validate_zipcode
Module B imports and uses validate_zipcode

find_usages("validate_zipcode") → 5 files
I(Name) = log2(5) ≈ 2.32 bits of coupling
```

### D_KL(P||Q) — KL Divergence

```
D_KL(P || Q) = Σ P(x) · log(P(x) / Q(x))

Measures information lost when using Q to approximate P.
This is LSP: KL(P_sub || P_base) should be ≈ 0.

KL ≈ 0: subtype behavior matches base contract (LSP satisfied).
KL > 0.05: LSP violated — subtype adds behavior not in base.
KL = ∞: subtype has impossible states under base contract.
```

**Example:**
```
Base contract: GuardrailResult = {Pass, Fail}
Subtype: adds third state "Ambiguous"

KL(Subtype || Base) = ∞ (P(Ambiguous) > 0 but Q(Ambiguous) = 0)
LSP severely violated.
```

### F = H(X) - H(X|context) — Free Energy

```
F = "information NOT explained by the purpose"
Low F = high cohesion (elements share a clear purpose)
High F = low cohesion (elements are unrelated)

Example:
  Module "TaxCalculator" with methods: compute(), format_pdf(), send_email()
  H(methods) = log2(3) ≈ 1.58 bits
  H(methods | purpose="tax calculation") ≈ 0.39 bits
  F = 1.58 - 0.39 ≈ 1.19 bits not explained by purpose

  Split: separate EmailSender → F drops for both modules.
  F_after < F_before → split is justified.
```

### CE(P||Q) — Cross-Entropy

```
CE(P || Q) = -Σ P(x) · log(Q(x)) = H(P) + D_KL(P || Q)

Minimizing CE(P_observed || P_desired) pushes behavior toward contract.
This is DIP in action: learning θ to minimize cross-entropy
between observed behavior and desired contract.
```

---

## Connascence Severity Scale

| Bits | Severity | Action |
|------|----------|--------|
| 0 – 0.5 | ✅ OK | No action needed |
| 0.5 – 1.0 | ⚠️ Low | Review, monitor |
| 1.0 – 3.0 | ⚠️ Medium | Plan refactoring in next cycle |
| 3.0 – 5.0 | ❌ High | Refactor before adding features |
| > 5.0 | 🔴 Critical | Immediate refactoring required |

**Connascence types mapped to I(A;B):**

| Type | Formula | Typical Range | Most Dangerous |
|------|---------|--------------|----------------|
| Name | log2(rename_propagation_count) | 0.1 – 4 bits | Medium |
| Type | log2(type_dependency_depth) | 0.1 – 3 bits | Medium |
| Meaning | I(sem_A; sem_B) | 0.1 – 2 bits (hidden!) | HIGH |
| Position | log2(invalid_reorderings) | 0.1 – 6.9 bits | High |
| Algorithm | I(out_A; out_B \| alg) | 0.1 – 4 bits | Medium |
| Execution | log2(valid_execution_orders) | 0 – 6.9 bits | High |
| Timing | P(timeout_violation) | 0.001 – 0.5 | Medium |
| Value | H(v_j \| v_i) | 0.1 – 8 bits | High |
| Identity | log2(instance_users) | 0.1 – log2(N) | Low |

---

## SOLID Entropy Thresholds

| Principle | Entropic Reformulation | Metric | Threshold |
|-----------|----------------------|--------|-----------|
| **SRP** | Split when F(component) > F(split_A) + F(split_B) | F = H(methods) - H(methods \| purpose) | F_before > F_after + C(split) |
| **OCP** | Extension adds H only in Δ_new, not Δ_existing | H(Δ_existing) | < 1.0 bit |
| **LSP** | KL(P_sub \|\| P_base) ≈ 0 | D_KL(P_sub \|\| P_base) | < 0.05 |
| **ISP** | H(client_view) = H(client_needs) | H(view) - H(needs) | < 1.0 bit |
| **DIP** | Depend on high-H abstractions | H(abstract) - H(concrete) | > 0 (abstract > concrete) |

---

## Design Quality Score Formula

```
DQS = w₁ × (1 - H_coupling)
    + w₂ × H_cohesion
    - w₃ × Σ KL(LSP_violations)
    - w₄ × Σ I(connascence_pairs)

Default weights:
  w₁ = 0.30 (coupling)
  w₂ = 0.30 (cohesion)
  w₃ = 0.25 (LSP violations)
  w₄ = 0.15 (connascence)

Components:
  H_coupling = average I(A;B) over all component pairs
  H_cohesion = average (1 - F/H) over all components
  Σ KL = sum of KL divergences for subtype/base pairs
  Σ I = sum of mutual information for all connascence pairs
```

**Interpretation:**

| Score | Rating | Meaning |
|-------|--------|---------|
| > 0.7 | 🟢 EXCELLENT | Low coupling, high cohesion, SOLID compliant |
| 0.3 – 0.7 | 🟡 ACCEPTABLE | Some connascence present, mostly SOLID |
| 0.0 – 0.3 | 🟠 NEEDS REFACTORING | Significant coupling issues |
| < 0.0 | 🔴 CRITICAL | Brittle system, immediate action required |

---

## Protocol A: Connascence Landscape (for sddk-explore)

**When:** Mandatory in sddk-explore Step 3.

**Purpose:** Map all connascence pairs in the affected areas, identify
hidden coupling (meaning/timing connascence), and surface critical pairs.

**Input:** Affected areas from exploration analysis.

**Procedure:**

```
1. For each affected component:
   a. cognicode_find_usages(symbol_name) → rename propagation count
   b. cognicode_get_call_hierarchy(direction: "incoming", depth: 2) → type dependencies
   c. cognicode_semantic_search for undocumented shared assumptions → meaning connascence

2. For each pair detected:
   a. Estimate I(A;B) in bits
   b. Classify connascence type (Name/Type/Meaning/Position/Algorithm/Execution/Timing/Value/Identity)
   c. Compare against severity thresholds

3. Build connascence landscape table:
   | Component A | Component B | Type | I(bits) | Severity | Hidden? |
```

**Output format:**
```markdown
### Entropy Analysis (Connascence Landscape)

**Method**: CogniCode / Heuristic

| Component A | Component B | Connascence Type | I(bits) | Severity |
|-------------|-------------|------------------|---------|----------|
| {file A} | {file B} | Meaning | 0.82 | ⚠️ HIGH | YES |
| {module} | {module} | Name | 0.32 | ✅ OK | No |

**Critical Pairs (I > 3.0 bits)**: {list}
**Hidden Connascence (Meaning/Timing)**: {list with explanation}
**SOLID-Entropy Violations**: {list if any}

**Coupling Score**: {H_external estimated}
**Recommendation**: {split/refactor/accept}
```

---

## Protocol B: Entropy Budget Prediction (for sddk-propose)

**When:** Mandatory in sddk-propose Step 3.

**Purpose:** Predict how much coupling this change will introduce before
writing any code. Quantify OCP compliance.

**Procedure:**

```
1. Analyze change scope from Capabilities section:
   a. New capabilities → new components (H_new = estimated bits)
   b. Modified capabilities → delta to existing (H_existing = estimated bits)

2. For modified components:
   a. cognicode_analyze_impact → files modified count
   b. cognicode_find_usages → propagation count if names change
   c. Estimate I(new pairs) introduced

3. OCP Check:
   H(Δ_existing) = bits of information that must change in EXISTING components
   H(Δ_new) = bits of new information

   If H(Δ_existing) >> 0 → OCP violated, extension requires modification
   If H(Δ_existing) ≈ 0 → OCP satisfied, pure extension
```

**Output format:**
```markdown
### Entropy Budget

**Method**: CogniCode / Heuristic

| Metric | Estimate (bits) | Threshold | Status |
|--------|-----------------|-----------|--------|
| H(Δ_existing) | {N} | < 1.0 | ✅ / ❌ |
| H(Δ_new) | {N} | > 0 | ✅ |
| New connascence pairs introduced | {N} | < 3 | ✅ / ⚠️ |
| OCP compliant? | {yes/no} | yes | ✅ / ❌ |

**Breaking Change Indicators**:
- H(Δ_existing) > 1.0 bits → existing code must change
- KL > 0.05 on any subtype → LSP risk
- H(Δ) > 3 bits → significant coupling introduction

**Verdict**: {green/yellow/red} — {one-line summary}
```

---

## Protocol C: Information Bottleneck Interface Check (for sddk-design)

**When:** Mandatory in sddk-design Step 2 (after architecture check).

**Purpose:** Validate that every new or modified interface is an optimal
information bottleneck: minimizes I(X;T) while maximizing I(T;Y).

**Procedure:**

```
For each NEW or MODIFIED interface in the design:

1. Identify the interface: trait, trait object, or abstract class.

2. Estimate I(X;T) — "leakage":
   X = internal state of implementing component
   T = the interface as seen by callers

   I(X;T) is HIGH when the interface exposes internal details.
   I(X;T) is LOW when the interface is a clean abstraction.

3. Estimate I(T;Y) — "coverage":
   T = the interface
   Y = what callers actually need

   I(T;Y) is HIGH when the interface provides exactly what callers need.
   I(T;Y) is LOW when the interface has too much or too little.

4. SRP Check:
   F = H(methods) - H(methods | purpose)
   If F_before > F(split_A) + F(split_B) → split recommended.

5. DIP Check:
   Depend on high-H abstractions (traits), not low-H concretions.
   H(trait) should be > H(concrete implementation).

6. ISP Check:
   H(client_view) should ≈ H(client_needs).
   If H(view) >> H(needs) → interface is too broad.
```

**Output format:**
```markdown
### Entropy Constraints

**Method**: CogniCode / Heuristic

| Interface | I(X;T) Leakage | I(T;Y) Coverage | Bottleneck Quality | SOLID Check |
|-----------|---------------|-----------------|-------------------|-------------|
| {trait name} | {bits} (Low/Med/High) | {bits} (Low/Med/High) | ✅ Optimal / ⚠️ Review | SRP ✅/⚠️ ISP ✅/⚠️ DIP ✅/⚠️ |
| {trait name} | HIGH | Low | ❌ Over-exposed | ISP violated |

**Interface Design Issues**:
- {list of interfaces that leak too much or provide too little}
**SRP Split Candidates**: {list with F analysis}
**ISP Violations**: {list of overly-broad interfaces}
**DIP Assessment**: {list of dependencies on low-H concretions}
```

---

## Protocol D: Entropy Verification (for sddk-verify)

**When:** Mandatory in sddk-verify Step 5b (between TDD Compliance and Testing).

**Purpose:** Verify that the implementation did not worsen entropy metrics
and that SOLID entropy principles are satisfied.

**Procedure:**

```
1. Connascence Delta Audit:
   For each pair modified in this change:
   a. Estimate I(A;B) BEFORE (from sddk-propose entropy budget)
   b. Estimate I(A;B) AFTER (from current codebase reading)
   c. ΔI = I_after - I_before

   ΔI > 0.5 bits → WARNING
   ΔI > 2.0 bits → CRITICAL

2. SOLID-Entropy Compliance Matrix:
   | Principle | Metric | Value | Threshold | Status |
   | SRP | F(component) | {val} | F < threshold | ✅/❌ |
   | OCP | H(Δ_existing) actual | {bits} | < 1.0 | ✅/❌ |
   | LSP | KL(P_sub | P_base) | {val} | < 0.05 | ✅/❌ |
   | ISP | H(view) - H(needs) | {bits} | < 1.0 | ✅/❌ |
   | DIP | H(abstract) - H(concrete) | {bits} | > 0 | ✅/❌ |

3. Design Quality Score:
   Compute DQS using the formula in Section 2.
   Compare against baseline from sddk-design (if available).

4. Entropy Budget vs Actual:
   Compare sddk-propose predictions against actual measurements.
   | Metric | Predicted | Actual | Delta | Status |
```

**Output format:**
```markdown
### Entropy Analysis

**Method**: CogniCode / Heuristic

**Design Quality Score**: {N}/1.0 ({rating})

| Component | Score | Status |
|-----------|-------|--------|
| Coupling | {val} | ✅/⚠️/❌ |
| Cohesion | {val} | ✅/⚠️/❌ |
| LSP compliance | {val} | ✅/⚠️/❌ |
| Connascence | {val} | ✅/⚠️/❌ |

**SOLID-Entropy Compliance**:
| Principle | Value | Threshold | Status |
|-----------|-------|-----------|--------|
| SRP | F={val} | < threshold | ✅/❌ |
| OCP | H(Δ)={bits} | < 1.0 | ✅/❌ |
| LSP | KL={val} | < 0.05 | ✅/❌ |
| ISP | waste={bits} | < 1.0 | ✅/❌ |
| DIP | ΔH={bits} | > 0 | ✅/❌ |

**Connascence Delta**: {N added, M removed, K changed}
**Entropy Budget Accuracy**: {predicted vs actual comparison}
```

---

## Protocol E: Entropy Trend (for sddk-archive)

**When:** Mandatory in sddk-archive Step 4 (after syncing specs).

**Purpose:** Record entropy metrics alongside archived specs and compare
with previous archives to detect improving or degrading trends.

**Procedure:**

```
1. Compute final entropy metrics for the completed change:
   a. Final DQS score
   b. Connascence pair count and distribution
   c. SOLID-entropy compliance summary

2. Check for previous archives:
   a. Look for sddk-archive/{previous_change}/entropy-metrics
   b. If found, compare:
      - DQS trend: improving / stable / degrading
      - Connascence pairs: fewer / same / more
      - Critical pairs: resolved / persisting / new

3. Generate trend narrative.
```

**Output format:**
```markdown
### Entropy Trend

**Change**: {change-name}
**Final DQS**: {N}/1.0 ({rating})
**Connascence Pairs**: {N total} ({N} added, {N} removed)

**Trend vs Previous Archive**:
| Metric | Previous | Current | Trend |
|--------|----------|---------|-------|
| DQS | {val} | {val} | ↑ improving / → stable / ↓ degrading |
| Critical pairs | {N} | {N} | ↓ resolved / → same / ↑ new |
| Coupling | {val} | {val} | ↑↓ |

**Improvements**: {list}
**Regressions**: {list or "None"}
**Recommendation**: {continue current approach / address regressions before next change}
```

---

## Estimation Heuristics (when CogniCode unavailable)

### Manual Connascence Estimation

```
Name connascence:
  Count files/imports referencing the symbol by name.
  I(Name) = log2(count)

Type connascence:
  Count how many modules use a shared type.
  I(Type) = log2(user_count)

Meaning connascence (hardest to detect):
  Look for: magic numbers shared across modules, undocumented enums,
  convention comments like "// 0 means success", error code assumptions.
  If found: flag as ⚠️ MEANING and estimate I = 0.5-1.0 bits.

Position connascence:
  If function calls must happen in exact order (no reordering valid):
    I(Position) = log2(1) = 0 (completely ordered, high connascence)
  If steps can be reordered freely:
    I(Position) = log2(valid_reorderings) (high uncertainty = low connascence)

Algorithm connascence:
  If the same logic is copy-pasted across N modules:
    I(Algorithm) = log2(N) (one change propagates to N places)

Execution connascence:
  Count the minimum linear ordering constraints.
  I(Execution) = log2(valid_orderings)

Value connascence:
  If changing field X in module A requires changing field Y in module B:
    H(Y | X) ≈ 0 → high connascence
```

### Coupling Estimation from File Count

```
Low coupling (H < 1.0 bit): 1-2 files touched, no shared types
Medium coupling (H 1-3 bits): 3-5 files, 1-2 shared types
High coupling (H 3-5 bits): 6-10 files, multiple shared types
Critical coupling (H > 5 bits): >10 files, circular dependencies
```

---

## Compact Rules

- **entropy-sdd is MANDATORY** — never skip entropy analysis in any SDD phase
- **Always report method**: `CogniCode` or `Heuristic` — never omit confidence source
- **Always report confidence**: `quantitative` (CogniCode) or `estimated` (heuristic)
- **I(A;B) > 3.0 bits = CRITICAL** — flag immediately in any phase
- **H(Δ_existing) > 1.0 bit = OCP violated** — extension touches existing code
- **KL(P_sub||P_base) > 0.05 = LSP violated** — subtype breaks contract
- **H(view) >> H(needs) = ISP violated** — client sees too much
- **DQS < 0.3 = NEEDS REFACTORING** — design quality is poor
- **Report ALL metrics** in every phase — partial entropy data is insufficient
- **Trend analysis** in archive is mandatory — always compare with previous if available
