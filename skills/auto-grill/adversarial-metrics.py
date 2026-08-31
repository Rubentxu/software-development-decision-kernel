#!/usr/bin/env python3
"""
Adversarial Entropy Metrics Calculator for SDD Verify.

When the adversarial judgment runs, the LLM estimates entropy metrics
for each deficiency found. This script:
1. Computes the Adversarial Entropy Score (AES) per finding
2. Classifies findings by type and severity
3. Produces the correction priority list
4. Generates the spec alignment report

Usage:
  # Score individual findings:
  python3 adversarial-metrics.py --findings '[
    {"id":"F1","type":"spec_gap","description":"Missing TTL scenario",
     "spec_coverage":0.6,"impl_entropy":1.2,"blast_radius":3,"reversibility":0.8},
    {"id":"F2","type":"code_bug","description":"Race in cache",
     "spec_coverage":1.0,"impl_entropy":2.8,"blast_radius":5,"reversibility":0.9}
  ]'

  # From file:
  python3 adversarial-metrics.py --file findings.json

  # Spec alignment report:
  python3 adversarial-metrics.py --spec-alignment --file findings.json

  # Correction priority list:
  python3 adversarial-metrics.py --correction-plan --file findings.json
"""

import argparse
import json
import sys
from dataclasses import dataclass, field
from typing import List, Optional
from enum import Enum


# ──────────────────────────────────────────────────────────
#  Finding types and severity
# ──────────────────────────────────────────────────────────

class FindingType(str, Enum):
    SPEC_GAP = "spec_gap"              # Spec doesn't cover implemented behavior
    SPEC_AMBIGUITY = "spec_ambiguity"  # Spec is vague, multiple interpretations
    SPEC_STALE = "spec_stale"          # Spec describes old behavior, code changed
    CODE_BUG = "code_bug"              # Code doesn't do what spec says
    CODE_MISSING = "code_missing"      # Spec requires something not implemented
    DESIGN_DRIFT = "design_drift"      # Implementation deviates from design
    DESIGN_OMISSION = "design_omission"  # Design didn't anticipate a real concern
    ENTROPY_REGRESSION = "entropy_regression"  # Coupling/cohesion worsened


class Severity(str, Enum):
    CRITICAL = "CRITICAL"
    WARNING = "WARNING"
    SUGGESTION = "SUGGESTION"


# ──────────────────────────────────────────────────────────
#  Data model
# ──────────────────────────────────────────────────────────

@dataclass
class Finding:
    id: str
    type: str           # FindingType value
    description: str
    
    # Entropy metrics (estimated by LLM, values 0.0-1.0 unless noted)
    spec_coverage: float     # How much of the intended behavior is covered by spec (0=none, 1=complete)
    impl_entropy: float     # I(A;B) bits of coupling introduced by the deficiency (0=none, max ~5)
    blast_radius: float     # Normalized fan-out: how many modules affected (0=isolated, 1=system-wide)
    reversibility: float    # How easy to fix (0=rewrite needed, 1=trivial fix)
    entropy_delta: float    # ΔH introduced by this deficiency (bits, can be negative if improvement)
    information_loss: float # I(X;T) leakage — how much internal state leaks through interface (0=clean, 1=severe)
    
    # Computed (not from LLM)
    aes_score: float = field(init=False)        # Adversarial Entropy Score
    severity: str = field(init=False)            # CRITICAL / WARNING / SUGGESTION
    correction_effort: float = field(init=False) # Estimated correction effort (0=trivial, 1=major rewrite)
    priority_rank: int = field(init=False)       # Correction priority (1=highest)

    def __post_init__(self):
        for dim in ["spec_coverage", "blast_radius", "reversibility", "information_loss"]:
            val = getattr(self, dim)
            setattr(self, dim, max(0.0, min(1.0, val)))
        self.impl_entropy = max(0.0, min(8.0, self.impl_entropy))
        self.entropy_delta = max(-5.0, min(8.0, self.entropy_delta))


@dataclass
class SpecAlignment:
    """Spec alignment assessment."""
    total_requirements: int
    covered_requirements: int
    partial_requirements: int
    missing_requirements: int
    ambiguous_requirements: int
    stale_requirements: int
    coverage_score: float = field(init=False)  # 0-1
    alignment_score: float = field(init=False)  # 0-1

    def __post_init__(self):
        self.coverage_score = (self.covered_requirements + 0.5 * self.partial_requirements) / max(1, self.total_requirements)
        issues = self.missing_requirements + self.ambiguous_requirements + self.stale_requirements
        self.alignment_score = 1.0 - (issues / max(1, self.total_requirements))


@dataclass
class CorrectionPlan:
    """Prioritized list of corrections."""
    findings: List[Finding]
    spec_updates: List[dict]
    code_fixes: List[dict]
    design_updates: List[dict]
    total_effort: float = field(init=False)
    
    def __post_init__(self):
        self.total_effort = sum(f.correction_effort for f in self.findings)


# ──────────────────────────────────────────────────────────
#  Core metrics
# ──────────────────────────────────────────────────────────

def compute_aes(finding: Finding) -> float:
    """
    Adversarial Entropy Score — quantifies how bad a deficiency is.
    
    AES = w_spec × (1 - spec_coverage)
        + w_entropy × min(impl_entropy / 5.0, 1.0)
        + w_blast × blast_radius
        + w_leak × information_loss
        + w_delta × max(entropy_delta, 0) / 5.0
    
    Higher AES = worse deficiency = higher priority to fix.
    
    Weights emphasize blast radius and entropy impact because:
    - A bug that affects 1 module is less urgent than one affecting 8
    - Coupling introduced by a deficiency compounds over time
    - Information leakage breaks encapsulation permanently
    """
    w_spec = 0.20     # Spec gap weight
    w_entropy = 0.25   # Implementation entropy weight (highest — coupling is expensive)
    w_blast = 0.25     # Blast radius weight (highest — impact is what matters)
    w_leak = 0.15      # Information leakage weight
    w_delta = 0.15     # Entropy delta weight
    
    normalized_entropy = min(finding.impl_entropy / 5.0, 1.0)
    normalized_delta = max(finding.entropy_delta, 0.0) / 5.0
    
    aes = (
        w_spec * (1.0 - finding.spec_coverage) +
        w_entropy * normalized_entropy +
        w_blast * finding.blast_radius +
        w_leak * finding.information_loss +
        w_delta * normalized_delta
    )
    
    # Reversibility modifier: hard-to-fix issues get a boost (they're more dangerous)
    # because they'll stay in the codebase longer
    reversibility_penalty = 1.0 - (0.2 * (1.0 - finding.reversibility))
    
    return round(min(aes * reversibility_penalty, 1.0), 4)


def classify_severity(aes: float, finding_type: str) -> str:
    """Classify finding severity based on AES and type."""
    
    # Entropy regressions are always at least WARNING
    if finding_type == "entropy_regression" and aes >= 0.3:
        return Severity.CRITICAL
    
    # Code bugs with high blast radius are CRITICAL
    if finding_type in ["code_bug", "code_missing"] and aes >= 0.5:
        return Severity.CRITICAL
    
    # Spec gaps with high entropy are CRITICAL (they hide coupling)
    if finding_type == "spec_gap" and aes >= 0.6:
        return Severity.CRITICAL
    
    # General thresholds
    if aes >= 0.5:
        return Severity.CRITICAL
    elif aes >= 0.25:
        return Severity.WARNING
    else:
        return Severity.SUGGESTION


def estimate_correction_effort(finding: Finding) -> float:
    """Estimate effort to fix: 0=trivial, 1=major rewrite."""
    
    # Base effort from reversibility (hard to reverse = more effort)
    base = 1.0 - finding.reversibility
    
    # Type-specific modifiers
    type_modifiers = {
        "spec_gap": 0.2,        # Writing specs is relatively cheap
        "spec_ambiguity": 0.15, # Clarifying is cheap
        "spec_stale": 0.3,      # Updating needs investigation
        "code_bug": 0.4,        # Fixing bugs needs care
        "code_missing": 0.6,    # Implementing from scratch is expensive
        "design_drift": 0.5,    # Realigning takes thought
        "design_omission": 0.4, # Adding design decisions
        "entropy_regression": 0.7, # Refactoring coupling is expensive
    }
    
    modifier = type_modifiers.get(finding.type, 0.3)
    
    # Blast radius increases effort (more files to touch)
    blast_modifier = finding.blast_radius * 0.3
    
    return round(min(base * modifier + blast_modifier, 1.0), 4)


def evaluate_finding(finding: Finding) -> Finding:
    """Evaluate a single finding: compute AES, severity, effort."""
    finding.aes_score = compute_aes(finding)
    finding.severity = classify_severity(finding.aes_score, finding.type)
    finding.correction_effort = estimate_correction_effort(finding)
    return finding


def prioritize(findings: List[Finding]) -> List[Finding]:
    """Sort findings by priority: highest AES first, ties broken by effort."""
    for f in findings:
        evaluate_finding(f)
    
    # Sort by: CRITICAL first, then AES descending, then effort ascending (easy wins first)
    severity_order = {"CRITICAL": 0, "WARNING": 1, "SUGGESTION": 2}
    sorted_findings = sorted(
        findings,
        key=lambda f: (severity_order.get(f.severity, 3), -f.aes_score, f.correction_effort)
    )
    
    for i, f in enumerate(sorted_findings):
        f.priority_rank = i + 1
    
    return sorted_findings


# ──────────────────────────────────────────────────────────
#  Spec alignment
# ──────────────────────────────────────────────────────────

def compute_spec_alignment(findings: List[Finding], total_requirements: int = 0) -> SpecAlignment:
    """Compute spec alignment from findings."""
    covered = 0
    partial = 0
    missing = 0
    ambiguous = 0
    stale = 0
    
    for f in findings:
        if f.type == "spec_gap":
            if f.spec_coverage >= 0.8:
                partial += 1
            elif f.spec_coverage >= 0.4:
                partial += 1
            else:
                missing += 1
        elif f.type == "spec_ambiguity":
            ambiguous += 1
        elif f.type == "spec_stale":
            stale += 1
        elif f.type in ["code_bug", "code_missing"]:
            if f.spec_coverage >= 0.8:
                covered += 1
            else:
                partial += 1
    
    # If no total given, estimate from findings
    if total_requirements == 0:
        total_requirements = covered + partial + missing + ambiguous + stale
    
    return SpecAlignment(
        total_requirements=max(total_requirements, 1),
        covered_requirements=covered,
        partial_requirements=partial,
        missing_requirements=missing,
        ambiguous_requirements=ambiguous,
        stale_requirements=stale,
    )


# ──────────────────────────────────────────────────────────
#  Correction plan
# ──────────────────────────────────────────────────────────

def generate_correction_plan(findings: List[Finding]) -> CorrectionPlan:
    """Generate prioritized correction plan from findings."""
    prioritized = prioritize(findings)
    
    spec_updates = []
    code_fixes = []
    design_updates = []
    
    for f in prioritized:
        entry = {
            "id": f.id,
            "priority": f.priority_rank,
            "description": f.description,
            "severity": f.severity,
            "aes_score": f.aes_score,
            "effort": f.correction_effort,
        }
        
        if f.type in ["spec_gap", "spec_ambiguity", "spec_stale"]:
            entry["action"] = {
                "spec_gap": "add_missing_scenarios",
                "spec_ambiguity": "clarify_language",
                "spec_stale": "update_to_current",
            }[f.type]
            spec_updates.append(entry)
        
        elif f.type in ["code_bug", "code_missing", "entropy_regression"]:
            entry["action"] = {
                "code_bug": "fix_behavior",
                "code_missing": "implement_missing",
                "entropy_regression": "refactor_coupling",
            }[f.type]
            code_fixes.append(entry)
        
        elif f.type in ["design_drift", "design_omission"]:
            entry["action"] = {
                "design_drift": "realign_implementation",
                "design_omission": "add_design_decision",
            }[f.type]
            design_updates.append(entry)
    
    return CorrectionPlan(
        findings=prioritized,
        spec_updates=spec_updates,
        code_fixes=code_fixes,
        design_updates=design_updates,
    )


# ──────────────────────────────────────────────────────────
#  Output formatting
# ──────────────────────────────────────────────────────────

def format_findings_table(findings: List[Finding]) -> str:
    prioritized = prioritize(findings)
    
    lines = []
    lines.append("")
    lines.append("╔════╦═════════════════╦═══════════════════════════════╦═════════╦═════════╦═════════╦═════════╦═════════╦══════════╦══════════╗")
    lines.append("║ #  ║ Tipo            ║ Descripción                   ║ Spec Cov║ Entropía║ Blast R ║ InfoLeak║ EntΔ    ║   AES    ║ Severidad║")
    lines.append("╠════╬═════════════════╬═══════════════════════════════╬═════════╬═════════╬═════════╬═════════╬═════════╬══════════╬══════════╣")
    
    for f in prioritized:
        ftype = f.type[:15].ljust(15)
        desc = f.description[:29].ljust(29)
        severity_str = f.severity.value if hasattr(f.severity, 'value') else str(f.severity)
        severity_icon = {"CRITICAL": "🔴", "WARNING": "🟡", "SUGGESTION": "🔵"}.get(severity_str, "⚪")
        
        lines.append("║ {:>2} ║ {} ║ {} ║  {:.2f}  ║  {:.2f}  ║  {:.2f}  ║  {:.2f}  ║ {:+.2f}  ║  {:.4f}  ║ {} {:<8} ║".format(
            f.priority_rank, ftype, desc,
            f.spec_coverage, f.impl_entropy, f.blast_radius,
            f.information_loss, f.entropy_delta,
            f.aes_score, severity_icon, severity_str
        ))
    
    lines.append("╚════╩═════════════════╩═══════════════════════════════╩═════════╩═════════╩═════════╩═════════╩═════════╩══════════╩══════════╝")
    
    # Summary
    criticals = sum(1 for f in prioritized if f.severity == "CRITICAL")
    warnings = sum(1 for f in prioritized if f.severity == "WARNING")
    suggestions = sum(1 for f in prioritized if f.severity == "SUGGESTION")
    avg_effort = sum(f.correction_effort for f in prioritized) / max(1, len(prioritized))
    
    lines.append("")
    lines.append("Resumen: {} 🔴 CRITICAL | {} 🟡 WARNING | {} 🔵 SUGGESTION".format(criticals, warnings, suggestions))
    lines.append("Esfuerzo promedio de corrección: {:.2f} (0=trivial, 1=major rewrite)".format(avg_effort))
    
    # Correction priority
    lines.append("")
    lines.append("── Orden de corrección (prioridad por AES + esfuerzo) ──")
    for f in prioritized:
        if f.severity == "SUGGESTION":
            continue
        effort_bar = "█" * int(f.correction_effort * 10) + "░" * (10 - int(f.correction_effort * 10))
        lines.append("  {:>2}. {} [AES={:.3f}] esfuerzo: |{}| {:.1f}".format(
            f.priority_rank, f.description[:40], f.aes_score, effort_bar, f.correction_effort
        ))
    
    return "\n".join(lines)


def format_spec_alignment(alignment: SpecAlignment) -> str:
    lines = []
    lines.append("")
    lines.append("── Spec Alignment ──")
    lines.append("  Coverage:       {:.1f}% ({}/{} requirements)".format(
        alignment.coverage_score * 100,
        alignment.covered_requirements,
        alignment.total_requirements
    ))
    lines.append("  Alignment:      {:.1f}%".format(alignment.alignment_score * 100))
    lines.append("  Parciales:      {}".format(alignment.partial_requirements))
    lines.append("  Faltantes:      {}".format(alignment.missing_requirements))
    lines.append("  Ambiguas:       {}".format(alignment.ambiguous_requirements))
    lines.append("  Desactualizadas: {}".format(alignment.stale_requirements))
    
    if alignment.alignment_score >= 0.8:
        lines.append("  Veredicto:      🟢 ALINEADO — specs reflejan la implementación")
    elif alignment.alignment_score >= 0.5:
        lines.append("  Veredicto:      🟡 PARCIAL — gaps detectados, actualizar specs")
    else:
        lines.append("  Veredicto:      🔴 DESALINEADO — specs necesitan revisión significativa")
    
    return "\n".join(lines)


def format_correction_plan(plan: CorrectionPlan) -> str:
    lines = []
    lines.append("")
    lines.append("╔══════════════════════════════════════════════════════════════╗")
    lines.append("║                   CORRECTION PLAN                             ║")
    lines.append("╠══════════════════════════════════════════════════════════════╣")
    
    if plan.spec_updates:
        lines.append("║                                                              ║")
        lines.append("║ 📋 SPEC UPDATES ({} items)".format(len(plan.spec_updates)))
        for u in plan.spec_updates:
            lines.append("║   {:>2}. [AES={:.3f}] {} → {}".format(
                u["priority"], u["aes_score"], u["description"][:30], u["action"]
            ))
    
    if plan.code_fixes:
        lines.append("║                                                              ║")
        lines.append("║ 🔧 CODE FIXES ({} items)".format(len(plan.code_fixes)))
        for u in plan.code_fixes:
            lines.append("║   {:>2}. [AES={:.3f}] {} → {}".format(
                u["priority"], u["aes_score"], u["description"][:30], u["action"]
            ))
    
    if plan.design_updates:
        lines.append("║                                                              ║")
        lines.append("║ 📐 DESIGN UPDATES ({} items)".format(len(plan.design_updates)))
        for u in plan.design_updates:
            lines.append("║   {:>2}. [AES={:.3f}] {} → {}".format(
                u["priority"], u["aes_score"], u["description"][:30], u["action"]
            ))
    
    lines.append("║                                                              ║")
    lines.append("║ Esfuerzo total estimado: {:.2f}".format(plan.total_effort))
    lines.append("╚══════════════════════════════════════════════════════════════╝")
    
    return "\n".join(lines)


def format_json(plan: CorrectionPlan) -> str:
    return json.dumps({
        "findings": [
            {
                "id": f.id,
                "type": f.type,
                "description": f.description,
                "metrics": {
                    "spec_coverage": f.spec_coverage,
                    "impl_entropy": f.impl_entropy,
                    "blast_radius": f.blast_radius,
                    "reversibility": f.reversibility,
                    "entropy_delta": f.entropy_delta,
                    "information_loss": f.information_loss,
                },
                "computed": {
                    "aes_score": f.aes_score,
                    "severity": f.severity,
                    "correction_effort": f.correction_effort,
                    "priority_rank": f.priority_rank,
                }
            }
            for f in plan.findings
        ],
        "spec_alignment": {
            "coverage_score": round(compute_spec_alignment(plan.findings).coverage_score, 4),
            "alignment_score": round(compute_spec_alignment(plan.findings).alignment_score, 4),
        },
        "correction_plan": {
            "spec_updates": plan.spec_updates,
            "code_fixes": plan.code_fixes,
            "design_updates": plan.design_updates,
            "total_effort": round(plan.total_effort, 4),
        },
    }, indent=2, ensure_ascii=False)


# ──────────────────────────────────────────────────────────
#  CLI
# ──────────────────────────────────────────────────────────

def parse_findings(data: list) -> List[Finding]:
    return [
        Finding(
            id=d["id"],
            type=d["type"],
            description=d["description"],
            spec_coverage=d.get("spec_coverage", 0.5),
            impl_entropy=d.get("impl_entropy", 1.0),
            blast_radius=d.get("blast_radius", 0.3),
            reversibility=d.get("reversibility", 0.5),
            entropy_delta=d.get("entropy_delta", 0.0),
            information_loss=d.get("information_loss", 0.2),
        )
        for d in data
    ]


def main():
    parser = argparse.ArgumentParser(
        description="Adversarial Entropy Metrics Calculator para SDD Verify",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Dimensiones por finding (valores 0.0-1.0 salvo impl_entropy y entropy_delta):
  spec_coverage    → cuánto cubre la spec del comportamiento real (0=nada, 1=completo)
  impl_entropy     → I(A;B) bits de acoplamiento introducido por la deficiencia (0-8)
  blast_radius     → módulos afectados normalizado (0=aislado, 1=sistema completo)
  reversibility    → facilidad de corregir (0=rewrite, 1=trivial)
  entropy_delta    → ΔH introducido por esta deficiencia en bits (negativo=mejora)
  information_loss → I(X;T) leakage de estado interno (0=clean, 1=severo)

Tipos de finding:
  spec_gap, spec_ambiguity, spec_stale, code_bug, code_missing,
  design_drift, design_omission, entropy_regression
""",
    )
    parser.add_argument("--findings", type=str, help="JSON array of findings")
    parser.add_argument("--file", type=str, help="JSON file with findings")
    parser.add_argument("--json", action="store_true", help="JSON output")
    parser.add_argument("--correction-plan", action="store_true", help="Show correction plan")
    parser.add_argument("--spec-alignment", action="store_true", help="Show spec alignment")
    
    args = parser.parse_args()
    
    findings_data = []
    if args.findings:
        findings_data = json.loads(args.findings)
    elif args.file:
        with open(args.file) as f:
            findings_data = json.load(f)
    else:
        parser.print_help()
        sys.exit(1)
    
    findings = parse_findings(findings_data)
    plan = generate_correction_plan(findings)
    
    if args.json:
        print(format_json(plan))
    else:
        print(format_findings_table(plan.findings))
        
        if args.spec_alignment:
            alignment = compute_spec_alignment(plan.findings)
            print(format_spec_alignment(alignment))
        
        if args.correction_plan:
            print(format_correction_plan(plan))


if __name__ == "__main__":
    main()
