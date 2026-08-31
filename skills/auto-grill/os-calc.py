#!/usr/bin/env python3
"""
Opportunity Score Calculator for Auto-Grill.

The LLM estimates the 6 dimensions per option (values 0.0-1.0).
This script computes the weighted Opportunity Score and ranks options.

Usage:
  python3 os-calc.py --options '[{"name":"A","coupling":0.15,"free_energy":0.10,"openness":0.85,"flexibility":0.88,"depth":0.90,"irreversibility":0.25}]'

  # Or read from a JSON file:
  python3 os-calc.py --file options.json

  # Or interactive mode (prompts for each dimension):
  python3 os-calc.py --interactive --names "A: Fachadas" "B: Split por crate" "C: Registry"

Output: Ranked table with OS scores, ratings, and dimension breakdown.
"""

import argparse
import json
import sys
import math
from dataclasses import dataclass, field, asdict
from typing import List, Optional


# ──────────────────────────────────────────────────────────
#  Default weights (adjustable via --weights)
# ──────────────────────────────────────────────────────────

DEFAULT_WEIGHTS = {
    "coupling":       0.20,  # Less new coupling = more maintainable
    "free_energy":    0.15,  # Lower F = better cohesion
    "openness":       0.20,  # Extension > modification (OCP)
    "flexibility":    0.25,  # More future scenarios = more valuable
    "depth":          0.10,  # Deep modules > shallow wrappers
    "irreversibility": 0.10, # Reversible decisions are safer
}

RATING_THRESHOLDS = [
    (0.70, "🟢 EXCELENTE", "excelente"),
    (0.40, "🟡 BUENO",     "bueno"),
    (0.20, "🟠 REGULAR",   "regular"),
    (0.00, "🔴 POBRE",     "pobre"),
]


# ──────────────────────────────────────────────────────────
#  Data model
# ──────────────────────────────────────────────────────────

@dataclass
class Option:
    name: str
    coupling: float        # 0.0 (no new coupling) to 1.0 (severe coupling)
    free_energy: float     # 0.0 (cohesion improves) to 1.0 (cohesion worsens)
    openness: float        # 0.0 (pure modification) to 1.0 (pure extension)
    flexibility: float     # 0.0 (no future scenarios) to 1.0 (many scenarios)
    depth: float           # 0.0 (shallow wrapper) to 1.0 (deep module)
    irreversibility: float # 0.0 (trivial to revert) to 1.0 (very hard to revert)
    os_score: float = field(init=False)
    rating: str = field(init=False)
    rating_emoji: str = field(init=False)
    dimension_breakdown: dict = field(init=False, default_factory=dict)

    def __post_init__(self):
        # Clamp all values to [0.0, 1.0]
        for dim in ["coupling", "free_energy", "openness", "flexibility", "depth", "irreversibility"]:
            val = getattr(self, dim)
            setattr(self, dim, max(0.0, min(1.0, val)))


@dataclass
class OSResult:
    options: List[Option]
    weights: dict
    recommended: Option

    def to_json(self) -> str:
        return json.dumps({
            "options": [
                {
                    "name": o.name,
                    "dimensions": {
                        "coupling": o.coupling,
                        "free_energy": o.free_energy,
                        "openness": o.openness,
                        "flexibility": o.flexibility,
                        "depth": o.depth,
                        "irreversibility": o.irreversibility,
                    },
                    "os_score": round(o.os_score, 4),
                    "rating": o.rating_emoji,
                    "breakdown": o.dimension_breakdown,
                }
                for o in sorted(self.options, key=lambda x: x.os_score, reverse=True)
            ],
            "weights": self.weights,
            "recommended": {
                "name": self.recommended.name,
                "os_score": round(self.recommended.os_score, 4),
                "rating": self.recommended.rating_emoji,
            },
        }, indent=2, ensure_ascii=False)


# ──────────────────────────────────────────────────────────
#  Core calculation
# ──────────────────────────────────────────────────────────

def calculate_os(option: Option, weights: dict) -> float:
    """
    OS = w_coupling     × (1 - coupling)
       + w_free_energy  × (1 - free_energy)
       + w_openness     × openness
       + w_flexibility  × flexibility
       + w_depth        × depth
       + w_irrevers     × (1 - irreversibility)
    """
    return (
        weights["coupling"]       * (1.0 - option.coupling) +
        weights["free_energy"]    * (1.0 - option.free_energy) +
        weights["openness"]       * option.openness +
        weights["flexibility"]    * option.flexibility +
        weights["depth"]          * option.depth +
        weights["irreversibility"] * (1.0 - option.irreversibility)
    )


def get_rating(score: float) -> tuple:
    for threshold, emoji, label in RATING_THRESHOLDS:
        if score >= threshold:
            return emoji, label
    return "🔴 POBRE", "pobre"


def evaluate(option: Option, weights: dict) -> Option:
    """Evaluate a single option: compute OS, rating, and dimension breakdown."""
    option.os_score = calculate_os(option, weights)
    option.rating_emoji, option.rating = get_rating(option.os_score)

    # Dimension breakdown (contribution to OS)
    option.dimension_breakdown = {
        "acoplamiento":  round(weights["coupling"] * (1.0 - option.coupling), 4),
        "cohesion":      round(weights["free_energy"] * (1.0 - option.free_energy), 4),
        "apertura":      round(weights["openness"] * option.openness, 4),
        "flexibilidad":  round(weights["flexibility"] * option.flexibility, 4),
        "profundidad":   round(weights["depth"] * option.depth, 4),
        "reversibilidad": round(weights["irreversibility"] * (1.0 - option.irreversibility), 4),
    }
    return option


def evaluate_all(options: List[Option], weights: dict) -> OSResult:
    """Evaluate all options and determine recommendation."""
    for opt in options:
        evaluate(opt, weights)
    ranked = sorted(options, key=lambda x: x.os_score, reverse=True)
    return OSResult(options=ranked, weights=weights, recommended=ranked[0])


# ──────────────────────────────────────────────────────────
#  Output formatting
# ──────────────────────────────────────────────────────────

def format_table(result: OSResult) -> str:
    """Format results as a readable table."""
    lines = []
    lines.append("")
    lines.append("╔══════════════════════════════════════════════════════════════════════════════════════════╗")
    lines.append("║                         OPPORTUNITY SCORE — RESULTADOS                                  ║")
    lines.append("╠══════════════════════════════════════════════════════════════════════════════════════════╣")
    lines.append("║ Pesos: coupling={:.2f} cohesion={:.2f} apertura={:.2f} flex={:.2f} depth={:.2f} revers={:.2f} ║".format(
        result.weights["coupling"], result.weights["free_energy"],
        result.weights["openness"], result.weights["flexibility"],
        result.weights["depth"], result.weights["irreversibility"]
    ))
    lines.append("╠══════════╦═════════╦═════════╦═════════╦═════════╦═════════╦═════════╦═════════╦═══════╣")
    lines.append("║ Opción   ║ ΔI(norm)║ ΔF(norm)║ Apertur.║ Flexibi.║ Depth   ║ Revers. ║   OS    ║ Rating║")
    lines.append("╠══════════╬═════════╬═════════╬═════════╬═════════╬═════════╬═════════╬═════════╬═══════╣")

    for i, opt in enumerate(result.options):
        marker = "→" if opt == result.recommended else " "
        name = opt.name[:8].ljust(8)
        lines.append("║{}{}║  {:.3f}  ║  {:.3f}  ║  {:.3f}  ║  {:.3f}  ║  {:.3f}  ║  {:.3f}  ║  {:.4f} ║ {} ║".format(
            marker, name,
            opt.coupling, opt.free_energy, opt.openness,
            opt.flexibility, opt.depth, opt.irreversibility,
            opt.os_score, opt.rating_emoji
        ))

    lines.append("╠══════════╩═════════╩═════════╩═════════╩═════════╩═════════╩═════════╩═════════╩═══════╣")
    lines.append("║ RECOMENDADA: {:<20} OS={:.4f}  {}                                              ║".format(
        result.recommended.name, result.recommended.os_score, result.recommended.rating_emoji
    ))
    lines.append("╚══════════════════════════════════════════════════════════════════════════════════════════╝")

    # Breakdown for recommended
    lines.append("")
    lines.append("Desglose de la opción recomendada ({}):".format(result.recommended.name))
    rec = result.recommended
    for dim_name, dim_label in [
        ("acoplamiento",  "Acoplamiento  (w×(1-ΔI))"),
        ("cohesion",      "Cohesión      (w×(1-ΔF))"),
        ("apertura",      "Apertura      (w×openness)"),
        ("flexibilidad",  "Flexibilidad  (w×flex)    "),
        ("profundidad",   "Profundidad   (w×depth)   "),
        ("reversibilidad","Reversibilidad(w×(1-rev)) "),
    ]:
        val = rec.dimension_breakdown[dim_name]
        bar_len = int(val / 0.25 * 20)
        bar = "█" * bar_len + "░" * (20 - bar_len)
        lines.append("  {} = {:.4f} |{}|".format(dim_label, val, bar))
    lines.append("  {} = {:.4f} (sum)".format(" " * 27, rec.os_score))

    return "\n".join(lines)


def format_html_snippet(result: OSResult) -> str:
    """Generate HTML table rows for the report."""
    lines = []
    for opt in sorted(result.options, key=lambda x: x.os_score, reverse=True):
        css_class = {
            "🟢 EXCELENTE": "bg-emerald-50 border-l-4 border-emerald-500",
            "🟡 BUENO": "bg-amber-50 border-l-4 border-amber-400",
            "🟠 REGULAR": "bg-orange-50 border-l-4 border-orange-500",
            "🔴 POBRE": "bg-red-50 border-l-4 border-red-500",
        }.get(opt.rating_emoji, "")

        badge_class = {
            "🟢 EXCELENTE": "os-green",
            "🟡 BUENO": "os-yellow",
            "🟠 REGULAR": "os-orange",
            "🔴 POBRE": "os-red",
        }.get(opt.rating_emoji, "os-yellow")

        recommended = " font-semibold" if opt == result.recommended else ""
        lines.append(f'      <tr class="{css_class}">')
        lines.append(f'        <td class="py-3{recommended}">{opt.name}</td>')
        lines.append(f'        <td class="py-3 text-center">{opt.coupling:.2f}</td>')
        lines.append(f'        <td class="py-3 text-center">{opt.free_energy:.2f}</td>')
        lines.append(f'        <td class="py-3 text-center">{opt.openness:.2f}</td>')
        lines.append(f'        <td class="py-3 text-center">{opt.flexibility:.2f}</td>')
        lines.append(f'        <td class="py-3 text-center">{opt.depth:.2f}</td>')
        lines.append(f'        <td class="py-3 text-center">{opt.irreversibility:.2f}</td>')
        lines.append(f'        <td class="py-3 text-center"><span class="{badge_class} text-white text-xs font-bold px-2 py-1 rounded-full">{opt.os_score:.2f}</span></td>')
        lines.append(f'        <td class="py-3 text-center">{opt.rating_emoji}</td>')
        lines.append(f'      </tr>')

    return "\n".join(lines)


# ──────────────────────────────────────────────────────────
#  Input parsing
# ──────────────────────────────────────────────────────────

def parse_options_from_json(data: list) -> List[Option]:
    return [
        Option(
            name=d["name"],
            coupling=d.get("coupling", 0.5),
            free_energy=d.get("free_energy", 0.5),
            openness=d.get("openness", 0.5),
            flexibility=d.get("flexibility", 0.5),
            depth=d.get("depth", 0.5),
            irreversibility=d.get("irreversibility", 0.5),
        )
        for d in data
    ]


def interactive_input(names: List[str]) -> List[Option]:
    options = []
    dims = [
        ("coupling",        "Acoplamiento (0=sin acoplamiento nuevo, 1=severo)"),
        ("free_energy",     "Energía libre ΔF (0=cohesión mejora, 1=empeora)"),
        ("openness",        "Apertura OCP (0=pura modificación, 1=pura extensión)"),
        ("flexibility",     "Flexibilidad (0=sin escenarios futuros, 1=muchos)"),
        ("depth",           "Profundidad (0=wrapper superficial, 1=módulo profundo)"),
        ("irreversibility", "Irreversibilidad (0=fácil revertir, 1=muy difícil)"),
    ]
    for name in names:
        print(f"\n── Opción: {name} ──")
        vals = {}
        for dim_key, dim_desc in dims:
            while True:
                try:
                    v = float(input(f"  {dim_desc}: "))
                    if 0.0 <= v <= 1.0:
                        vals[dim_key] = v
                        break
                    print("    ⚠ Valor debe estar entre 0.0 y 1.0")
                except ValueError:
                    print("    ⚠ Ingresa un número entre 0.0 y 1.0")
        options.append(Option(name=name, **vals))
    return options


# ──────────────────────────────────────────────────────────
#  CLI entry point
# ──────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Opportunity Score Calculator para Auto-Grill",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Ejemplos:
  # Desde JSON inline:
  python3 os-calc.py --options '[{"name":"A","coupling":0.15,"free_energy":0.10,"openness":0.85,"flexibility":0.88,"depth":0.90,"irreversibility":0.25}]'

  # Desde archivo:
  python3 os-calc.py --file options.json

  # Interactivo:
  python3 os-calc.py --interactive --names "Opción A" "Opción B" "Opción C"

  # Con pesos custom:
  python3 os-calc.py --weights '{"flexibility":0.35,"coupling":0.15}' --file options.json

  # Solo JSON output (para piping):
  python3 os-calc.py --json --options '[...]'

Dimensiones (valores 0.0-1.0):
  coupling        → acoplamiento nuevo (0=sin acoplamiento, 1=severo)
  free_energy     → cambio en cohesión (0=mejora, 1=empeora)
  openness        → ratio OCP (0=modificación, 1=extensión pura)
  flexibility     → escenarios futuros (0=ninguno, 1=muchos)
  depth           → profundidad del módulo (0=superficial, 1=profundo)
  irreversibility → dificultad de revertir (0=fácil, 1=muy difícil)
""",
    )
    parser.add_argument("--options", type=str, help="JSON array of options inline")
    parser.add_argument("--file", type=str, help="JSON file with options array")
    parser.add_argument("--interactive", action="store_true", help="Interactive mode")
    parser.add_argument("--names", nargs="+", help="Option names for interactive mode")
    parser.add_argument("--weights", type=str, help="Custom weights as JSON")
    parser.add_argument("--json", action="store_true", help="Output as JSON only")
    parser.add_argument("--html", action="store_true", help="Output HTML table rows")

    args = parser.parse_args()

    # Load weights
    weights = DEFAULT_WEIGHTS.copy()
    if args.weights:
        custom = json.loads(args.weights)
        weights.update(custom)

    # Load options
    options = []
    if args.options:
        options = parse_options_from_json(json.loads(args.options))
    elif args.file:
        with open(args.file) as f:
            options = parse_options_from_json(json.load(f))
    elif args.interactive:
        if not args.names:
            print("Error: --interactive requiere --names")
            sys.exit(1)
        options = interactive_input(args.names)
    else:
        parser.print_help()
        sys.exit(1)

    if not options:
        print("Error: no se proporcionaron opciones")
        sys.exit(1)

    # Evaluate
    result = evaluate_all(options, weights)

    # Output
    if args.json:
        print(result.to_json())
    elif args.html:
        print(format_html_snippet(result))
    else:
        print(format_table(result))


if __name__ == "__main__":
    main()
