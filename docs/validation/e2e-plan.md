# E2E Validation Plan — Instalación, despliegue, multi-lenguaje y render

**ADR:** [ADR-0001](ADR-0001-e2e-validation-sandbox.md)
**Estado:** implementado (2026-08-06) — 7/7 suites + checklist N3 PASS (PR #91)
**Ciclo de referencia:** post-v1-2-0 (CLOSED) — este plan abre el siguiente milestone

---

## 1. Objetivo

Probar el framework SDK completo como lo usaría un usuario real:
1. **Instalación**: `install.sh` real (binario + bundle + verificación de integridad) en un entorno limpio
2. **Despliegue**: `dev link`/`dev doctor`/`completion install` contra la estructura de un editor
3. **Uso sobre código real**: ciclos formales en repos de **lenguajes diferentes**
4. **Render**: los diagramas y HTML que genera el framework se renderizan y **se observa qué se muestra**

## 2. Niveles de prueba

### N1 — Instalación (sandbox hermético, sin git)

**Imagen:** `debian:12-slim` (sin git, sin curl → se instala curl mínimo; sin opencode)

**Pipeline:**
```
container limpio
  → curl install.sh (de GitHub Releases o SDDK_BASE_URL local)
  → binario instalado en ~/.local/bin + sha256 verificado
  → [variante cosign] firma keyless verificada
  → bundle software-development-decision-kernel.tar.gz extraído en ~/.local/share/sddk/framework (XDG runtime)
  → dev link --editor all → estructura de editor simulada (~/.config/opencode vacío)
  → verificación: 67 agents, 93 skills, 35 prompts, 4 workflows, opencode.json registrado
  → dev doctor all_present
  → completion install --shell fish → archivo escrito
  → USO: sddk adopt + cycle start + generate docs + vault export en un repo de ejemplo
```

**Variantes:**
| Variante | Condición | Verifica |
|----------|-----------|----------|
| N1a | sin cosign instalado | fallback sha256 + mensaje skip |
| N1b | cosign instalado | firma keyless verificada (real) |
| N1c | `--editor none` | solo binario, hints correctos |
| N1d | `--version v1.3.0` pinneado | versión exacta descargada |

**Criterios de aceptación N1:**
- [ ] exit 0 en todas las variantes
- [ ] `sddk --version` reporta la versión esperada
- [ ] 67 agents + 93 skills + 35 prompts + 4 workflows linkeados (symlinks reales)
- [ ] `opencode.json` contiene el registro con `prompt:{file:...}`
- [ ] `dev doctor` → `all_present: true`
- [ ] `completion install` escribe el archivo del shell
- [ ] `adopt` + `cycle start` + `generate docs` + `vault export` funcionan con el binario instalado

### N2 — Render y verificación visual (sandbox + mmdc)

**Herramientas:** `mmdc` (mermaid-cli, ya verificado: renderiza el state diagram → SVG 36KB)

**Pipeline:**
```
sddk generate docs --root <repo>
  → workflow.md (bloque mermaid) → mmdc → workflow-states.svg + .png
  → verificación programática del SVG: nodos esperados presentes
sddk vault export → vault-inspector.html → screenshot (chromium headless)
sddk archive/release → closing-report.html → screenshot
  → análisis de screenshot: página no blanca, dimensiones válidas, elementos clave
```

**Criterios de aceptación N2:**
- [ ] SVG/PNG generados sin errores mermaid
- [ ] Nodos esperados en el SVG: `OPEN_explore`, `CLOSED_archive`, `RELEASE_PENDING_release`, `cycle.start`, `archive.complete`
- [ ] Screenshots no vacíos (>5KB, dimensiones > 400px)
- [ ] El inspector HTML muestra los nodos del vault (tabla no vacía)

### N3 — Editor real (entorno del desarrollador, checklist)

**No es reproducible en CI puro** (opencode headless limitado). Se ejecuta en el entorno donde opencode ya corre. **Ejecutado 2026-08-06 — todos los checks PASS:**

- [x] `sddk dev link --root <framework> --editor all` (opencode + zcode) — 67 agents / 93 skills / 35 prompts / 4 workflows, 0 errors
- [x] opencode arranca y carga los 67 agents registrados — `sddk dev doctor` all_present: true (opencode + zcode)
- [x] El agente `orchestrator` responde con el prompt SDDK — prompt resuelto desde `agents/orchestrator.md`
- [x] Un prompt de prueba con `sddk-verify` (subagent) funciona — registrado como subagent, prompt resuelto desde `agents/sddk-verify.md`
- [x] Los skills (judgment-day, impeccable, sddk-*) aparecen disponibles — 12 skills `sddk-*` symlinked en opencode + zcode

## 3. Matrix multi-lenguaje (extensión de validate-project.sh)

| Lenguaje | Imagen | Comando test | Proyecto real candidato |
|----------|--------|--------------|-------------------------|
| Rust | `rust:1.91-slim` | `cargo test` | sharkdp/fd (ya validado, reusar) |
| Python | `python:3.12-slim` | `pytest` | pydantic/pydantic |
| Go | `golang:1.23` | `go test ./...` | spf13/cobra |
| Node | `node:22-slim` | `npm test` | expressjs/express |
| C | `gcc:13` | `make test` | redis/redis (subconjunto) |

**Extensión del script:** `validate-project.sh --lang <lang>` selecciona imagen + comando de test + pasos de build. Report JSON por lenguaje con: clone_sha, baseline_tests, adopt_done, cycle_open, after_tests, verdict.

**Criterios de aceptación multi-lenguaje:**
- [ ] 5/5 lenguajes: adopt + cycle start + baseline tests OK
- [ ] 0 regresiones en cada proyecto (tests after ≥ baseline)
- [ ] Report JSON por lenguaje generado

## 4. Entregables

| Artefacto | Descripción |
|-----------|-------------|
| `scripts/e2e-install.sh` | Pipeline N1 (variantes a-d) |
| `scripts/e2e-render.sh` | Pipeline N2 (render + verificación) |
| `scripts/validate-project.sh --lang` | Multi-lenguaje (extensión) |
| `scripts/e2e-all.sh` | Orquestador N1+N2+matrix → report consolidado |
| `docs/validation/e2e-report.md` | Report final con evidencia (screenshots embebidos) |
| `docs/validation/e2e-evidence/` | SVG/PNG/screenshots generados |

## 5. Ejecución

```bash
./scripts/e2e-all.sh                     # todo
./scripts/e2e-install.sh                 # solo N1
./scripts/e2e-render.sh                  # solo N2
./scripts/validate-project.sh pydantic/pydantic --lang python
```

**Hermetismo opcional:** `SDDK_BASE_URL=file:///mirror` para N1 sin red.

## 6. Riesgos y mitigaciones

| Riesgo | Mitigación |
|--------|-----------|
| N1 requiere red a GitHub Releases | `SDDK_BASE_URL` local + retry en download |
| mmdc/chromium pesado en CI | Solo el job de render lo usa; N1 no lo necesita |
| Proyectos reales grandes (redis) | Subconjunto (módulos con tests acotados) |
| opencode headless limitado | N3 en entorno real con checklist, no en CI |
| Versionado de imágenes | Pins por digest en el script |

## 7. Trazabilidad

- ADR: `docs/adr/ADR-0001-e2e-validation-sandbox.md`
- Este plan: `docs/validation/e2e-plan.md`
- Roadmap: `docs/sddk-stabilization-plan/ROADMAP.md` (milestone E2E-2026-08)
- Vault: `milestones/M-NNN-e2e-validation.md` (creado por el ciclo formal)
