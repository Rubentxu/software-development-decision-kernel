# -*- coding: utf-8 -*-
"""Contract test: C3i objetivo 2 — bootstrap de adopción repetido es no-op.

CTX-UAT-001 (roadmap C3i): 20 reinicios/bootstrap del mismo proyecto convergido
no re-piden adopción y no mutan el estado. La parte de runtime está pinada en
crates (apply_on_converged_adoption_is_byte_stable_across_repeats); este test
pinea la SUPERFICIE contractual que los prompts/skills enseñan al agente:

1. `sddk adopt apply` sobre un proyecto ya adoptado es el mecanismo de
   convergencia y es idempotente; NUNCA se enseña como "re-adopción".
2. `sddk adopt refresh` es el verbo EXPLÍCITO para refrescar runtime metadata
   (timestamp/actor) — no `apply`.
3. Ninguna superficie de orquestación exige re-adopción por sesión: la skill
   de resume consulta `adopt status` (lectura), no `apply` (escritura), y el
   estatuto "adoption remains orchestrator-owned ... never applies it" de
   init.md se mantiene.

Falsación observada (session-37): revertir el short-circuit de converge()
hacía mutar el hash del recibo en cada re-apply; el pin de runtime lo detecta.
Este test detecta la reincidencia documental: alguien volviendo a enseñar
"ejecuta adopt apply en cada sesión" o borrando el contrato de idempotencia.
"""
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

checks = []


def check(name, condition, detail=""):
    checks.append((name, bool(condition), detail))


def read(rel):
    return (ROOT / rel).read_text(encoding="utf-8")


# 1. cycle-resume skill: la reconstrucción lee `adopt status` (idempotente,
#    read-only), nunca recomienda `apply` como paso por sesión.
resume = read("skills/sddk-cycle-resume/SKILL.md")
check(
    "resume-consulta-status-no-apply",
    "sddk adopt status" in resume and "sddk adopt apply" not in resume,
    "la skill de resume debe leer status; apply no es paso de sesión",
)

# 2. init phase: la adopción sigue siendo orchestrator-owned y init nunca
#    la aplica (evita re-adopción en cascada por fase).
init_md = read("prompts/sddk/phases/init.md")
check(
    "init-nunca-aplica-adopcion",
    "adoption remains orchestrator-owned" in init_md and "init never applies it" in init_md,
    "init.md debe mantener el estatuto anti-re-adopcion",
)

# 3. cli-usage-contract: apply es convergencia idempotente, refresh es el
#    verbo de runtime metadata. Ambos descritos, sin orden de sesión.
shared = read("skills/_shared/cli-usage-contract.md")
check(
    "contrato-cli-describe-adopt",
    "adopt status" in shared,
    "el contrato compartido debe documentar adopt status",
)

# 4. La prueba de runtime del no-op existe y está anclada al objetivo C3i.
engine = read("crates/sddk-engine/src/adoption.rs")
check(
    "runtime-pin-existe",
    "apply_on_converged_adoption_is_byte_stable_across_repeats" in engine,
    "el pin de byte-estabilidad debe existir en adoption.rs",
)
check(
    "runtime-short-circuit-existe",
    "PIN C3i" in engine and "no-op semantico" in engine,
    "el short-circuit de converge debe estar documentado como pin C3i",
)

# 5. refresh sigue declarando su contrato "always refreshes" (vía explícita
#    para actualizar timestamp/actor sin tocar identidad).
check(
    "refresh-mantiene-contrato",
    "Always refreshes the on-disk receipt" in engine,
    "refresh_adoption debe seguir documentado como el verbo de runtime metadata",
)

failures = [c for c in checks if not c[1]]
for name, ok, detail in checks:
    print(f"  [{'OK' if ok else 'FAIL'}] {name}" + (f" — {detail}" if not ok else ""))
print(f"adopt-convergence contract: {len(checks) - len(failures)}/{len(checks)} checks")
sys.exit(1 if failures else 0)
