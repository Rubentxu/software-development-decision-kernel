---
status: accepted
date: 2026-08-06
deciders: [orchestrator, rubentxu]
linked-cycles: [post-v1-2-0]
---

# ADR-0001 — Ampliación del sandbox de validación E2E

## Contexto

La validación real del framework (gate 1.0.0) cubrió solo:
- 3 proyectos **Rust** (fd, zoxide, hyperfine) vía `scripts/validate-project.sh` (container rust:1.91-slim)
- Comportamiento del CLI en fixtures (`tests/cli.rs`, 3953 líneas)
- **Nunca** se probó la instalación real (`install.sh` → binario + bundle + firma) en un entorno limpio
- **Nunca** se renderizó/verificó visualmente lo que genera el framework (Mermaid de `generate docs`, HTML de `vault export`, closing reports)

Objetivo: ampliar el alcance para probar instalación, despliegue, uso sobre código real de **lenguajes diferentes**, y el **render de los diagramas** observando qué se muestra.

## Decisión

Arquitectura de validación en **tres niveles**, todos ejecutables en sandbox podman:

| Nivel | Qué valida | Dónde corre | Editor |
|-------|-----------|-------------|--------|
| **N1 — Instalación** | `install.sh` real (sha256 + firma cosign + bundle), `dev link`, `dev doctor`, `completion install`, uso del CLI (`adopt` + `cycle` + `generate`) | Sandbox Debian slim **sin git** | Estructura de editor **simulada** (`~/.config/opencode/` vacío) |
| **N2 — Render** | `generate docs` → Mermaid → SVG/PNG verificado (mmdc); `vault export` → HTML → screenshot; closing report → screenshot | Sandbox + mmdc (chromium headless) | — |
| **N3 — Editor real** (opcional) | opencode real arranca con los 67 agents + 93 skills linkeados | Entorno del desarrollador (donde opencode ya corre) | opencode real |

Principios:
1. **Hermético primero**: N1+N2 no dependen de opencode real (el `dev link` solo escribe symlinks + `opencode.json`; se verifica la estructura resultante).
2. **Sin git en N1**: prueba explícita del "no git required" del instalador.
3. **Multi-lenguaje**: matrix de imágenes (rust, python, go, node, c) con su comando de test nativo.
4. **Evidencia visual**: screenshots de todo lo renderizado, embebidos en el report.
5. **Fidelidad del release**: N1 descarga binarios REALES de GitHub Releases (no compilación local).

## Consecuencias

Positivas:
- Valida el instalador y el despliegue como los usaría un usuario real
- Detecta roturas de render (Mermaid/HTML) automáticamente
- Cubre lenguajes distintos (los agentes del framework son agnósticos al stack)

Negativas / costes:
- N1 requiere red hacia GitHub Releases (o `SDDK_BASE_URL` apuntando a un mirror local para hermetismo total)
- N2 añade dependencia de `mmdc`/chromium (solo en el job de render, no en N1)
- N3 no es reproducible en CI puro (opencode headless limitado) — se ejecuta en el entorno del desarrollador con checklist

## Alternativas consideradas

- **Instalar opencode real en el sandbox para todo**: descartado — peso y fragilidad (npm, red, versiones) sin aportar a la validez de N1/N2.
- **Probar solo con fixtures locales**: descartado para N1 — el objetivo es el binario publicado tal cual.
- **Mantener solo Rust**: descartado — el framework promete agnosticismo de stack; hay que demostrarlo.

## Decisiones relacionadas

- Gate 1.0.0 (2026-08-05): validación real inicial — superado, este ADR lo amplía.
- G1 (#52): adopt planta workflow manifest — prerequisito de N1 (adopt sin copia manual).
