#!/usr/bin/env python3
"""Integridad de referencias en las superficies publicables.

Una cita es prosa, no un enlace: nada la resuelve, así que una referencia a un
fichero inexistente sobrevive indefinidamente. No es hipótesis — este guard
nació de medir 394 referencias en las superficies y encontrar 25 pares
(citado -> destino) que no abren, agrupados en cinco familias:

  * las specs E14 (6 citas): `specs/` no existe en el repo, y tres agentes y sus
    tres skills lo citan como "full spec";
  * los agentes `cua-test-*` (5 citas): el flujo entero de
    `cua-test-orchestrator/SKILL.md` llama a tres actores que no existen;
  * `prompts/studio-agents/` (6 citas): los ficheros existen, como
    `agents/studio-*.md`; la ruta citada no;
  * `docs/impeccable-reference/` (2), `test-pyramid-builder` (3), y tres sueltas.

`git log --all` dice de todos ellos lo mismo: **cero commits**. No se
perdieron; nunca se escribieron, y `cua-test-orchestrator/SKILL.md` llegó así en
el import inicial (`34d68c21`).

Este guard congela la línea base. Hoy es verde porque el conjunto coincide
exactamente con lo conocido; **falla en cuanto alguien añade una cita rota
nueva**, y falla igual si alguien arregla una sin actualizar la línea base, que
es la mitad del valor: obliga a decidir qué se hizo con ella.

La clasificación `OUTPUT_PREFIXES` es una heurística a propósito: rutas como
`evidence/sources.yaml` o `build/drift-report.yml` no están rotas porque la
propia superficie las escribe al ejecutarse, no porque falten.
"""

from __future__ import annotations

import os
import re
import shutil
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

# Rutas que una superficie produce en el proyecto del usuario, no en el repo.
OUTPUT_PREFIXES = (
    "book-context/", "planning/", "research/", "examples/", "tests/cua/",
    ".playwright-cli/", "src/", "web-app/", "src-tauri/", "node_modules/",
    "output_dir/", "build/", "evidence/", "metrics/", "reports/", "diagrams/",
    "editorial/", "exercises/", "book-template/",
    "lib/", "milestones/",
)

# Una cita rota solo cuenta si apunta a una superficie del repo. Si apunta a otra
# cosa, casi siempre es un contrato de salida en tiempo de ejecucion.
REPO_SURFACES = (
    "agents/", "skills/", "prompts/", "docs/", "specs/", "crates/", "assets/",
    "scripts/", "githooks/", ".github/", "tests/", "ui-audit-protocol/",
)

PATH_RE = re.compile(
    r"`([A-Za-z0-9_][A-Za-z0-9._-]*/[A-Za-z0-9_./-]+\.[A-Za-z0-9]{1,6})`"
)
LINK_RE = re.compile(r"\]\((?!https?://)([^)#:]+)\)")

KNOWN_EXTENSIONS = {
    ".md", ".mjs", ".js", ".json", ".yaml", ".yml", ".sh", ".rs", ".toml",
    ".py", ".ts", ".kts", ".html", ".css", ".txt",
}

# Los 25 pares medidos y verificados a mano en session-65d. Cero commits los
# toco nunca;ver SESSION-JOURNAL session-65d (barrido de referencias).
KNOWN_BROKEN = {
    ("agents/deep-research-orchestrator.md", "skills/deep-research-methodology-hub/SKILL.md"),
    ("agents/impeccable-primary.md", "docs/impeccable-reference/README.md"),
    ("agents/impeccable-primary.md", "docs/impeccable-reference/impeccable-antipatterns.md"),
    ("agents/studio-orchestrator.md", "prompts/studio-agents/studio-analyzer.md"),
    ("agents/studio-orchestrator.md", "prompts/studio-agents/studio-block.md"),
    ("agents/studio-orchestrator.md", "prompts/studio-agents/studio-component.md"),
    ("agents/studio-orchestrator.md", "prompts/studio-agents/studio-page.md"),
    ("agents/studio-orchestrator.md", "prompts/studio-agents/studio-token.md"),
    ("agents/studio-orchestrator.md", "prompts/studio-agents/studio-validator.md"),
    ("agents/test-pyramid-builder.md", "assets/grill-test-coverage.md"),
    ("agents/test-pyramid-builder.md", "assets/test-strategy-doc.md"),
    ("agents/test-pyramid-builder.md", "docs/test-strategy.md"),
    ("agents/uat-discovery.md", "specs/E14-uat-guided-pipeline/E14.4-TEST-DISCOVERY-AGENT.md"),
    ("agents/uat-form-quality.md", "specs/E14-uat-guided-pipeline/E14.2-FORM-QUALITY-AGENT.md"),
    ("agents/uat-ux-form.md", "specs/E14-uat-guided-pipeline/E14.3-UX-FORM-AGENT.md"),
    ("skills/cua-test-orchestrator/SKILL.md", "agents/cua-test-judge.md"),
    ("skills/cua-test-orchestrator/SKILL.md", "agents/cua-test-orchestrator.body.md"),
    ("skills/cua-test-orchestrator/SKILL.md", "agents/cua-test-runner.md"),
    ("skills/cua-test-orchestrator/SKILL.md", "agents/cua-test-scenarist.md"),
    ("skills/cua-test-orchestrator/SKILL.md", "ui-audit-protocol/SKILL.md"),
    ("skills/skill-registry/SKILL.md", "docs/skill-style-guide.md"),
    ("skills/uat-discovery/SKILL.md", "specs/E14-uat-guided-pipeline/E14.4-TEST-DISCOVERY-AGENT.md"),
    ("skills/uat-form-quality/SKILL.md", "specs/E14-uat-guided-pipeline/E14.2-FORM-QUALITY-AGENT.md"),
    ("skills/uat-ux-form/SKILL.md", "specs/E14-uat-guided-pipeline/E14.3-UX-FORM-AGENT.md"),
    ("skills/ui-audit-protocol/SKILL.md", "agents/cua-test-judge.md"),
}

# Descartadas por verificacion manual: la superficie declara un fallback
# explicito para la ausencia. `skill-creator` y `skill-improver` dicen, en sus
# propias lineas, que si `docs/skill-style-guide.md` no esta, se use la copia
# empaquetada en references/ y, si tampoco, reglas inline. La ausencia del doc
# de repo esta disenada; marcarla como rota seria un falso positivo.
DECLARED_FALLBACK = {
    ("skills/skill-creator/SKILL.md", "docs/skill-style-guide.md"),
    ("skills/skill-improver/SKILL.md", "docs/skill-style-guide.md"),
}


def surface_files(root: Path):
    for base in (root / "agents", root / "prompts/sddk"):
        if base.is_dir():
            for path in sorted(base.glob("*.md")):
                yield path
    skills = root / "skills"
    if skills.is_dir():
        for path in sorted(skills.glob("*/SKILL.md")):
            yield path


def cited_tokens(text: str):
    tokens = {m.group(1) for m in PATH_RE.finditer(text)}
    tokens |= {m.group(1) for m in LINK_RE.finditer(text)}
    return tokens


def find_broken(root: Path):
    """Pares (superficie, destino) que una superficie cita y no abren.

    Las citas con fallback declarado se descuentan: la ausencia del destino es
    un caso disenado, no un defecto. Sin ese descuento, `skill-creator` y
    `skill-improver` —que dicen en sus propias lineas que usan la copia
    empaquetada si el doc de repo no esta— saldrian rotas por diseno.
    """
    broken = set()
    for path in surface_files(root):
        text = path.read_text(encoding="utf-8", errors="replace")
        rel = os.path.relpath(path, root)
        for token in cited_tokens(text):
            if "://" in token or any(ch in token for ch in "*?{"):
                continue
            if os.path.splitext(token)[1] not in KNOWN_EXTENSIONS or "/" not in token:
                continue
            if not token.startswith(REPO_SURFACES):
                continue
            if token.startswith(OUTPUT_PREFIXES):
                continue
            if (rel, token) in DECLARED_FALLBACK:
                continue
            if (path.parent / token).exists() or (root / token).exists():
                continue
            broken.add((rel, token))
    return broken


class SurfaceReferenceIntegrity(unittest.TestCase):
    def test_broken_set_matches_the_measured_baseline(self) -> None:
        found = find_broken(ROOT)
        new = found - KNOWN_BROKEN
        healed = KNOWN_BROKEN - found
        self.assertEqual(
            new, set(),
            "citas rotas nuevas en las superficies: corrige la ruta o anade el "
            "fichero, y despues actualiza KNOWN_BROKEN en este guard",
        )
        self.assertEqual(
            healed, set(),
            "una cita de la linea base ha dejado de romperse: actualiza "
            "KNOWN_BROKEN con la ruta nueva y registra el arreglo",
        )

    def test_declared_fallbacks_are_not_counted_as_broken(self) -> None:
        found = find_broken(ROOT)
        for pair in DECLARED_FALLBACK:
            self.assertNotIn(
                pair, found,
                "una cita con fallback declarado no puede registrarse como rota",
            )

    def test_detector_catches_an_injected_break(self) -> None:
        """Control negativo: el detector no puede pasar por vacuidad."""
        with tempfile.TemporaryDirectory() as tmp:
            sandbox = Path(tmp) / "repo"
            (sandbox / "skills" / "probe").mkdir(parents=True)
            target = sandbox / "skills" / "probe" / "SKILL.md"
            target.write_text(
                "---\nname: probe\ndescription: d\n---\n\n"
                "## Purpose\n\nprobe\n\n"
                "## References\n\n- `agents/never-written.md` — full spec\n",
                encoding="utf-8",
            )
            self.assertIn(("skills/probe/SKILL.md", "agents/never-written.md"),
                          find_broken(sandbox))

            # Y debe dejar de reportarlo cuando el destino existe de verdad.
            (sandbox / "agents").mkdir()
            (sandbox / "agents" / "never-written.md").write_text("x\n", encoding="utf-8")
            self.assertNotIn(("skills/probe/SKILL.md", "agents/never-written.md"),
                             find_broken(sandbox))


if __name__ == "__main__":
    unittest.main(verbosity=2)
