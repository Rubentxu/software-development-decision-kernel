# SDDK HTML Closing Report Format

Resolve `report_locale` under `prompts/sddk/phase-contracts.md`; the default is
`es`. Localize presentation prose and labels only. Machine keys, IDs, verdicts,
commands, paths, hashes, and API names remain unchanged.

The SDDK closing report is a self-contained, human-readable projection generated
after local release consolidation. Source reports, manifests, ledger receipts,
artifact hashes, and evidence remain authoritative. HTML explains those facts;
it never becomes their source of truth.

The report must include:
- The verified main SHA, annotated tag version, and local receipt references
- The semver bump reasoning (why major/minor/patch)
- The full MCW status (which steps completed, which were skipped if any — should be NONE)
- Link to all artifacts (proposal, spec, design, tasks, verify-report, archive-report)

Inline static CSS, fonts (or system fonts), and pre-rendered diagrams. Scripts
are forbidden. No CDN or network dependency is allowed.

## Security Contract

- Treat every value from source reports, Git, commands, paths, telemetry, and
  user-authored documents as untrusted text. HTML-escape `&`, `<`, `>`, `"`, and
  `'` before inserting it into text or attribute contexts.
- Never inject source HTML, Markdown-rendered raw HTML, CSS, Mermaid text, SVG,
  URLs, IDs, class names, or attribute names directly. Generate lists/tables
  item by item and derive classes only from fixed allowlists.
- Allow links only when the parsed URL is a same-document fragment, a safe
  relative artifact path, or `https:`. Reject control characters and every
  other scheme. External links use `rel="noreferrer noopener"`.
- Render diagrams before report assembly. Sanitize SVG by removing scripts,
  event-handler attributes, `foreignObject`, external references, and non-local
  URLs; then embed the sanitized SVG. Never ship a Mermaid runtime.
- Static CSS comes only from the framework template. Artifact content must not
  alter a `<style>` block.
- The examples below define layout only. Replace every placeholder with actual
  evidence or an explicit missing-evidence panel; never retain sample systems,
  actors, dates, metrics, or relationships in a generated report.

---

## Naming Convention

| Artifact Store | HTML Destination |
|----------------|------------------|
| XDG operational | `{cycle-artifacts-dir}/reports/cierre.html` |

Resolve temp dir from `$TMPDIR`, fallback to `/tmp`.

Write an optional disposable presentation copy under `$TMPDIR` only when useful.
Open a browser only when the user explicitly requests it.

---

## Progressive Disclosure (20 sections, 4 layers)

Every fact is available in every audience mode. `report_audience` changes only
which `<details>` panels start open:

| Layer | Purpose | Novice | Standard | Expert |
|---|---|---|---|---|
| **Summary** | Verdict, impact, risk, next action | open | open | open |
| **Guide** | What changed and how to use/recover | open | open | closed |
| **Technical detail** | Architecture, behavior, implementation | closed | open | open |
| **Evidence** | Commands, hashes, receipts, traceability | closed | closed | open |

The existing 20 sections are organised into four narrative blocks:

| Block | Question | Sections |
|-------|----------|----------|
| **A. Proceso SDDK** | ¿Qué pasó en el pipeline? | 1. Hero · 2. Pipeline Timeline · 3. Proposal & Scope |
| **B. Arquitectura técnica** | ¿Cómo está construido? | 4. Visión C4 · 5. Container Diagram · 6. Component Diagram · 7. Mapa de módulos · 8. Modelo de dominio · 9. Patrones aplicados · 10. Stack tecnológico |
| **C. Comportamiento** | ¿Qué hace y cómo fluye? | 11. Casos de uso · 12. Flujos de secuencia (callbacks completos) · 13. Modelo de datos |
| **D. Cierre** | ¿Qué riesgos quedan y qué sigue? | 14. Verification · 15. Git & Release · 16. Knowledge Impact · 17. Deuda técnica · 18. Operational characteristics · 19. Métricas & coste · 20. Next Steps |

Optional non-applicable sections may be omitted only with an `N/A` reason in
the Guide. Missing required evidence MUST render an explicit missing-evidence
panel with impact and recovery action; never silently omit it. The archive agent
looks for data in this priority order:
1. `{cycle-artifacts-dir}/proposal.md` — problem, scope, glossary
2. `{cycle-artifacts-dir}/spec.md` or `specs/` — scenarios and capabilities
3. `{cycle-artifacts-dir}/design.md` — components, contracts, invariants, patterns
4. `{cycle-artifacts-dir}/tasks.md` — file lists, commit messages, scope boundaries
5. `{cycle-artifacts-dir}/apply-progress.yaml` — actual files changed
6. `{cycle-artifacts-dir}/verify-report.md` and debt reports — quality evidence
7. `{cycle-artifacts-dir}/release-report.md` and `archive-report.md` — release and closure evidence
8. `git log` — commits, diff stats
9. `~/.local/share/opencode/telemetry/events.log` — phase durations, costs
10. Project `CONTEXT.md`, `CONTEXT-MAP.md`, `docs/adr/` — bounded contexts, ADRs

---

## Scaffold

```html
<!doctype html>
<html lang="{report_locale}">
  <head>
    <meta charset="utf-8" />
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; font-src data:; base-uri 'none'; form-action 'none'" />
    <title>SDDK — {change-name} — Cierre</title>
    <style>{compiled-inline-css}</style>
    <style>
      * { font-family: 'Inter', system-ui, sans-serif; }
      .font-mono { font-family: 'JetBrains Mono', monospace; }
      .card { background: white; border-radius: 16px; border: 1px solid #e2e8f0; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.05); }
      .section-dark { background: linear-gradient(135deg, #1e293b, #0f172a); color: #f1f5f9; }
      .section-blueprint { background: linear-gradient(135deg, #f8fafc, #f1f5f9); }
      .badge-feat { background: linear-gradient(135deg, #059669, #047857); }
      .badge-fix { background: linear-gradient(135deg, #dc2626, #b91c1c); }
      .badge-refactor { background: linear-gradient(135deg, #2563eb, #1d4ed8); }
      .badge-chore { background: linear-gradient(135deg, #6b7280, #4b5563); }
      .badge-pass { background: linear-gradient(135deg, #059669, #047857); }
      .badge-warn { background: linear-gradient(135deg, #d97706, #b45309); }
      .badge-fail { background: linear-gradient(135deg, #dc2626, #b91c1c); }
      .tag { display: inline-block; padding: 2px 8px; border-radius: 6px; font-size: 0.75rem; font-weight: 600; }
      .nav-pill { padding: 4px 12px; border-radius: 9999px; font-size: 0.75rem; background: rgba(255,255,255,0.1); color: #cbd5e1; }
      .nav-pill:hover { background: rgba(255,255,255,0.2); color: white; }
      .kbd { font-family: 'JetBrains Mono', monospace; font-size: 0.75rem; padding: 1px 6px; border-radius: 4px; background: #f1f5f9; border: 1px solid #cbd5e1; color: #475569; }
      .toc-link { display: block; padding: 6px 12px; border-radius: 8px; color: #475569; font-size: 0.85rem; }
      .toc-link:hover { background: #f1f5f9; color: #0f172a; }
      .toc-link.active { background: #1e293b; color: white; }
      .sticky-toc { position: sticky; top: 20px; max-height: calc(100vh - 40px); overflow-y: auto; }
      .uml-class { fill: #fef3c7; stroke: #92400e; }
      .arch-adapter { fill: #dbeafe; stroke: #1e40af; }
      .arch-port { fill: #fce7f3; stroke: #9d174d; }
      .arch-domain { fill: #d1fae5; stroke: #065f46; }
      .metric-card { background: white; border-radius: 12px; border: 1px solid #e2e8f0; padding: 16px; }
      .metric-value { font-size: 1.875rem; font-weight: 700; line-height: 1.1; }
      .metric-label { font-size: 0.75rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin-top: 4px; }
      .pattern-card { background: linear-gradient(135deg, #ecfdf5, #d1fae5); border-left: 4px solid #059669; border-radius: 8px; padding: 16px; }
      .risk-row { display: grid; grid-template-columns: 80px 1fr 100px; gap: 12px; padding: 8px 0; border-bottom: 1px solid #f1f5f9; align-items: center; }
      .risk-HIGH { background: #fef2f2; color: #b91c1c; padding: 2px 8px; border-radius: 4px; font-weight: 600; font-size: 0.7rem; text-align: center; }
      .risk-MEDIUM { background: #fefbeb; color: #b45309; padding: 2px 8px; border-radius: 4px; font-weight: 600; font-size: 0.7rem; text-align: center; }
      .risk-LOW { background: #f0fdf4; color: #047857; padding: 2px 8px; border-radius: 4px; font-weight: 600; font-size: 0.7rem; text-align: center; }
      details summary { cursor: pointer; user-select: none; }
      details summary::-webkit-details-marker { display: none; }
      .collapsible-arrow { display: inline-block; transition: transform 0.2s; }
      details[open] .collapsible-arrow { transform: rotate(90deg); }
    </style>
  </head>
  <body class="bg-gradient-to-br from-slate-100 to-stone-100 min-h-screen">
    <main class="max-w-7xl mx-auto px-6 py-12 grid grid-cols-12 gap-8">

      <!-- Sticky TOC sidebar -->
      <aside class="col-span-3">
        <nav class="sticky-toc card p-4">
          <h3 class="font-semibold text-slate-800 mb-3 text-sm uppercase tracking-wider">Contenido</h3>
          <div class="space-y-1">
            <a href="#summary" class="toc-link">1 · Hero</a>
            <a href="#pipeline" class="toc-link">2 · Pipeline Timeline</a>
            <a href="#proposal" class="toc-link">3 · Proposal & Scope</a>
            <a href="#c4-context" class="toc-link">4 · C4 — Contexto</a>
            <a href="#c4-container" class="toc-link">5 · C4 — Containers</a>
            <a href="#c4-component" class="toc-link">6 · C4 — Componentes</a>
            <a href="#module-map" class="toc-link">7 · Mapa de módulos</a>
            <a href="#domain-model" class="toc-link">8 · Modelo de dominio</a>
            <a href="#patterns" class="toc-link">9 · Patrones aplicados</a>
            <a href="#tech-stack" class="toc-link">10 · Stack tecnológico</a>
            <a href="#use-cases" class="toc-link">11 · Casos de uso</a>
            <a href="#sequences" class="toc-link">12 · Flujos de secuencia</a>
            <a href="#data-model" class="toc-link">13 · Modelo de datos</a>
            <a href="#verification" class="toc-link">14 · Verificación</a>
            <a href="#git" class="toc-link">15 · Git & Release</a>
            <a href="#knowledge" class="toc-link">16 · Impacto en conocimiento</a>
            <a href="#tech-debt" class="toc-link">17 · Deuda técnica</a>
            <a href="#ops" class="toc-link">18 · Características operacionales</a>
            <a href="#metrics" class="toc-link">19 · Métricas & coste</a>
            <a href="#next-steps" class="toc-link">20 · Próximos pasos</a>
          </div>
        </nav>
      </aside>

      <!-- Content column -->
      <div class="col-span-9 space-y-10">

        <!-- BLOCK A — PROCESO SDDK -->
        <!-- Section 1: Hero Summary -->
        <!-- Section 2: Pipeline Timeline -->
        <!-- Section 3: Proposal & Scope -->

        <!-- BLOCK B — ARQUITECTURA TÉCNICA -->
        <!-- Section 4: C4 Context -->
        <!-- Section 5: C4 Container -->
        <!-- Section 6: C4 Component -->
        <!-- Section 7: Module dependency map -->
        <!-- Section 8: Domain model -->
        <!-- Section 9: Patterns applied -->
        <!-- Section 10: Tech stack -->

        <!-- BLOCK C — COMPORTAMIENTO -->
        <!-- Section 11: Use cases -->
        <!-- Section 12: Sequence flows -->
        <!-- Section 13: Data model -->

        <!-- BLOCK D — CIERRE -->
        <!-- Section 14: Verification -->
        <!-- Section 15: Git & Release -->
        <!-- Section 16: Knowledge impact -->
        <!-- Section 17: Technical debt -->
        <!-- Section 18: Operational characteristics -->
        <!-- Section 19: Metrics & cost -->
        <!-- Section 20: Next Steps -->

      </div>
    </main>
  </body>
</html>
```

---

## Section 1: Hero — Executive Summary

One dark hero card with change name, project, date, verdict, and key stats. Includes **three KPI rows**: change stats, quality metrics, and pipeline cost.

```html
<section id="summary">
  <div class="section-dark rounded-2xl p-10 space-y-8">
    <div class="text-center space-y-4">
      <div class="flex justify-center gap-3">
        <span class="tag badge-{feat|fix|refactor|chore} text-white">{change-type}</span>
        <span class="tag badge-{pass|warn|fail} text-white">{verdict}</span>
        <span class="tag bg-white/10 text-white">v{semver-tag}</span>
      </div>
      <h1 class="text-5xl font-bold">{change-name}</h1>
      <p class="text-lg text-slate-300">{project} — {date}</p>
      <p class="text-sm text-slate-400 max-w-2xl mx-auto">{one-line-description}</p>
    </div>

    <!-- Row 1: Change size -->
    <div class="grid grid-cols-4 gap-4 max-w-3xl mx-auto">
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{N}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Tareas</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{M}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Commits</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{K}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Archivos</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold {delta-sign}">±{L}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Líneas</div>
      </div>
    </div>

    <!-- Row 2: Quality -->
    <div class="grid grid-cols-4 gap-4 max-w-3xl mx-auto">
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold text-emerald-400">{tests-pct}%</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Tests pasaron</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{coherence-score}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Coherence</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{connascence-risks}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Riesgos diseño</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{tech-debt-count}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Deuda técnica</div>
      </div>
    </div>

    <!-- Row 3: Pipeline cost -->
    <div class="grid grid-cols-4 gap-4 max-w-3xl mx-auto">
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{cycle-duration}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Duración total</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">${cost-usd}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Coste LLM (est.)</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{phases-completed}/{phases-total}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Fases</div>
      </div>
      <div class="text-center p-4 bg-white/5 rounded-xl">
        <div class="text-3xl font-bold">{escalations}</div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wider">Escalations</div>
      </div>
    </div>
  </div>
</section>
```

**Data sources**: archive-report (stats), apply-progress (file counts), telemetry events.log (duration, cost), `coherence/*.md` (scores), entropy_sdd output (connascence, DQS).

---

## Section 2: Pipeline Timeline

Gantt-style or waterfall showing phase durations, pre-rendered as sanitized SVG.

```html
<section id="pipeline">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-6">Pipeline SDDK — Línea de tiempo</h2>
    <figure aria-label="{escaped-pipeline-description}">{sanitized-pipeline-svg}</figure>
    <div class="mt-6 text-sm text-slate-600">
      <strong>Duración total</strong>: {total-duration}. <strong>Coste estimado</strong>: ${cost}. <strong>Fase más larga</strong>: {longest-phase} ({longest-duration}).
    </div>
  </div>
</section>
```

**Data source**: telemetry events.log filtered for `phase.completed`, sorted by timestamp.

---

## Section 3: Proposal & Scope

Same as before — what was proposed, scope boundaries, context quality. Kept for completeness.

```html
<section id="proposal">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-4">Propuesta y Alcance</h2>
    <div class="space-y-4">
      <div>
        <h3 class="text-sm font-semibold text-slate-400 uppercase tracking-wider">Problema</h3>
        <p class="text-slate-700 mt-1">{problem-statement}</p>
      </div>
      <div>
        <h3 class="text-sm font-semibold text-slate-400 uppercase tracking-wider">Solución propuesta</h3>
        <p class="text-slate-700 mt-1">{solution-summary}</p>
      </div>
      <div class="grid grid-cols-2 gap-4 pt-2">
        <div>
          <h3 class="text-sm font-semibold text-emerald-600 uppercase tracking-wider">Dentro de alcance</h3>
          <ul class="list-disc list-inside text-slate-700 mt-1 space-y-1">{in-scope-items}</ul>
        </div>
        <div>
          <h3 class="text-sm font-semibold text-red-600 uppercase tracking-wider">Fuera de alcance</h3>
          <ul class="list-disc list-inside text-slate-700 mt-1 space-y-1">{out-of-scope-items}</ul>
        </div>
      </div>
      <details class="border border-slate-200 rounded-xl p-4 mt-4">
        <summary class="font-semibold text-slate-700"><span class="collapsible-arrow">▶</span> Contexto del router (advanced)</summary>
        <div class="grid grid-cols-2 gap-4 pt-3 text-sm">
          <div><strong class="text-slate-500">Calidad de contexto</strong>: {C0/C1/C2/C3}</div>
          <div><strong class="text-slate-500">Taxonomía dominante</strong>: {axes}</div>
          <div><strong class="text-slate-500">Esfuerzo recomendado</strong>: {effort}</div>
          <div><strong class="text-slate-500">Lentes aplicadas</strong>: {lens-list}</div>
        </div>
      </details>
    </div>
  </div>
</section>
```

---

## Section 4: C4 — Diagrama de Contexto (Nivel 1)

The "system in its environment". Shows the change as a black box and its users,
dependencies, and external systems using a sanitized pre-rendered C4 SVG.

**Data sources**: design.md (system name, summary), project CONTEXT.md (users, integrations), apply-progress (external dependencies touched).

```html
<section id="c4-context">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">4 · Visión de contexto (C4 Nivel 1)</h2>
    <p class="text-sm text-slate-500 mb-6">El sistema modificado y sus interacciones con usuarios y sistemas externos.</p>
    <figure aria-label="{escaped-c4-context-description}">{sanitized-c4-context-svg}</figure>
    <div class="mt-4 text-sm text-slate-600">
      <strong>Sistema</strong>: {systemName} — {one-line-system-description}.<br>
      <strong>Usuarios primarios</strong>: {user-list}.<br>
      <strong>Dependencias externas</strong>: {external-deps-list}.
    </div>
  </div>
</section>
```

If the configured renderer is unavailable, render a textual relationship table;
do not add a browser-side diagram runtime.

---

## Section 5: C4 — Diagrama de Containers (Nivel 2)

The system broken into deployable units (apps, services, databases, queues). Shows technology choices.

**Data sources**: design.md (container list), tech-stack section of proposal, apply-progress (new containers/services created).

```html
<section id="c4-container">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">5 · Containers (C4 Nivel 2)</h2>
    <p class="text-sm text-slate-500 mb-6">Unidades desplegables. Una sola tecnología por container.</p>
    <figure aria-label="{escaped-c4-container-description}">{sanitized-c4-container-svg}</figure>

    <details class="mt-6">
      <summary class="text-sm font-semibold text-slate-700 cursor-pointer"><span class="collapsible-arrow">▶</span> Tabla de containers ({N} containers)</summary>
      <table class="w-full text-sm mt-3">
        <thead class="bg-slate-50">
          <tr><th class="text-left p-2">Container</th><th class="text-left p-2">Tecnología</th><th class="text-left p-2">Responsabilidad</th><th class="text-left p-2">Lenguaje</th></tr>
        </thead>
        <tbody>
          {for each container:}
          <tr class="border-b border-slate-100">
            <td class="p-2 font-mono text-xs">{name}</td>
            <td class="p-2">{tech}</td>
            <td class="p-2">{responsibility}</td>
            <td class="p-2">{language}</td>
          </tr>
        </tbody>
      </table>
    </details>
  </div>
</section>
```

---

## Section 6: C4 — Diagrama de Componentes (Nivel 3)

Inside the most-changed container. Shows modules, interfaces, ports.

**Data sources**: design.md (component list, ports/adapters), apply-progress (new files mapped to components).

```html
<section id="c4-component">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">6 · Componentes (C4 Nivel 3)</h2>
    <p class="text-sm text-slate-500 mb-6">Detalle del container más afectado: <span class="kbd">{main-container}</span></p>
    <figure aria-label="{escaped-c4-component-description}">{sanitized-c4-component-svg}</figure>

    <details class="mt-6">
      <summary class="text-sm font-semibold text-slate-700 cursor-pointer"><span class="collapsible-arrow">▶</span> Tabla de componentes ({N} componentes)</summary>
      <table class="w-full text-sm mt-3">
        <thead class="bg-slate-50">
          <tr><th class="text-left p-2">Componente</th><th class="text-left p-2">Capa</th><th class="text-left p-2">Responsabilidad</th><th class="text-left p-2">Path</th></tr>
        </thead>
        <tbody>
          {for each component:}
          <tr class="border-b border-slate-100">
            <td class="p-2 font-mono text-xs">{name}</td>
            <td class="p-2">{layer}</td>
            <td class="p-2">{responsibility}</td>
            <td class="p-2 font-mono text-xs text-slate-500">{path}</td>
          </tr>
        </tbody>
      </table>
    </details>
  </div>
</section>
```

---

## Section 7: Mapa de módulos (dependencias)

Module-level dependency graph. Shows what imports what, direction of dependencies, cycle detection.

**Data sources**: apply-progress files (use `cognicode_get_hot_paths` or grep imports), design.md (dependency rules).

```html
<section id="module-map">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">7 · Mapa de módulos</h2>
    <p class="text-sm text-slate-500 mb-6">Dependencias entre módulos del cambio. Flecha = "depende de".</p>
    <figure aria-label="{escaped-module-map-description}">{sanitized-module-map-svg}</figure>

    <div class="mt-6 grid grid-cols-3 gap-4 text-sm">
      <div class="metric-card">
        <div class="metric-value">{N}</div>
        <div class="metric-label">Módulos afectados</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">{M}</div>
        <div class="metric-label">Edges de dependencia</div>
      </div>
      <div class="metric-card">
        <div class="metric-value {cycle-color}">{cycle-count}</div>
        <div class="metric-label">Ciclos detectados</div>
      </div>
    </div>

    {if cycles > 0:}
    <div class="mt-4 p-4 bg-red-50 border border-red-200 rounded-xl">
      <strong class="text-red-700">⚠ Ciclos detectados:</strong>
      <ul class="list-disc list-inside text-red-600 text-sm mt-2">{cycle-list}</ul>
    </div>
    {endif}

    {if dependency-rule-violations:}
    <div class="mt-4 p-4 bg-amber-50 border border-amber-200 rounded-xl">
      <strong class="text-amber-700">⚠ Violaciones de regla de dependencia:</strong>
      <ul class="list-disc list-inside text-amber-700 text-sm mt-2">{violation-list}</ul>
    </div>
    {endif}
  </div>
</section>
```

---

## Section 8: Modelo de dominio (UML Class Diagram)

Class diagram for the most important domain types: aggregates, entities, value objects, domain events.

**Data sources**: design.md (domain types), code (struct/class definitions from apply-progress).

```html
<section id="domain-model">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">8 · Modelo de dominio</h2>
    <p class="text-sm text-slate-500 mb-6">Aggregates, entities, value objects y domain events principales.</p>
    <figure aria-label="{escaped-domain-model-description}">{sanitized-domain-model-svg}</figure>

    <div class="mt-6 grid grid-cols-3 gap-4 text-sm">
      <div class="metric-card">
        <div class="metric-value">{aggregates-count}</div>
        <div class="metric-label">Aggregates</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">{entities-count}</div>
        <div class="metric-label">Entities</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">{value-objects-count}</div>
        <div class="metric-label">Value objects</div>
      </div>
    </div>

    {if aggregate-boundaries:}
    <details class="mt-6">
      <summary class="text-sm font-semibold text-slate-700 cursor-pointer"><span class="collapsible-arrow">▶</span> Aggregate boundaries</summary>
      <table class="w-full text-sm mt-3">
        <thead class="bg-slate-50">
          <tr><th class="text-left p-2">Aggregate</th><th class="text-left p-2">Root</th><th class="text-left p-2">Entities dentro</th><th class="text-left p-2">Invariantes</th></tr>
        </thead>
        <tbody>
          {for each aggregate:}
          <tr class="border-b border-slate-100">
            <td class="p-2 font-mono text-xs">{name}</td>
            <td class="p-2">{root}</td>
            <td class="p-2 text-xs">{entities}</td>
            <td class="p-2 text-xs">{invariants}</td>
          </tr>
        </tbody>
      </table>
    </details>
    {endif}
  </div>
</section>
```

---

## Section 9: Patrones aplicados

Catalog of design patterns actually used in the implementation, with rationale.

**Data sources**: design.md (pattern mentions), code (grep for pattern usage).

```html
<section id="patterns">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">9 · Patrones aplicados</h2>
    <p class="text-sm text-slate-500 mb-6">Patrones de diseño identificados en la implementación, con dónde se usan y por qué.</p>

    <div class="space-y-4">
      {for each pattern:}
      <details class="pattern-card" open>
        <summary class="flex items-center gap-3">
          <span class="font-semibold text-slate-800">{pattern-name}</span>
          <span class="tag bg-emerald-100 text-emerald-700 text-xs">{category}</span>
          <span class="text-xs text-slate-500 ml-auto">{used-in N places}</span>
        </summary>
        <div class="mt-3 space-y-3 text-sm">
          <div>
            <strong class="text-slate-600">Problema que resuelve</strong>
            <p class="text-slate-700">{problem-statement}</p>
          </div>
          <div>
            <strong class="text-slate-600">Aplicación concreta</strong>
            <p class="text-slate-700">{concrete-application}</p>
          </div>
          <div>
            <strong class="text-slate-600">Trade-offs aceptados</strong>
            <p class="text-slate-700">{tradeoffs}</p>
          </div>
          <div>
            <strong class="text-slate-600">Ubicación</strong>
            <ul class="list-disc list-inside text-slate-700 font-mono text-xs">{locations}</ul>
          </div>
        </div>
      </details>
    </div>
  </div>
</section>
```

Common patterns to detect:
- **Hexagonal / Ports & Adapters** — port interfaces in domain layer, adapter implementations in infra
- **Repository** — data access abstraction
- **Aggregate Root** — domain consistency boundary
- **Value Object** — immutable types with no identity
- **Domain Event** — past-tense events for state changes
- **Strategy** — interchangeable algorithms behind an interface
- **Factory** — complex object construction
- **Decorator** — wrapping behavior
- **Observer / Pub-Sub** — event emission
- **Outbox** — transactional event publishing
- **Saga / Process Manager** — long-running workflows
- **CQRS** — separate read/write models
- **Specification** — composable business rules

---

## Section 10: Stack tecnológico

Tech inventory: language, framework, libraries, databases, infrastructure.

```html
<section id="tech-stack">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">10 · Stack tecnológico</h2>
    <p class="text-sm text-slate-500 mb-6">Tecnologías usadas, con versión y propósito.</p>

    <div class="grid grid-cols-2 gap-6">
      {for each category — language, framework, database, queue, infra, observability:}
      <div>
        <h3 class="text-sm font-semibold text-slate-500 uppercase tracking-wider mb-3">{category}</h3>
        <table class="w-full text-sm">
          <tbody>
            {for each tech:}
            <tr class="border-b border-slate-100">
              <td class="py-2 font-mono text-xs">{name}</td>
              <td class="py-2 text-xs text-slate-500">{version}</td>
              <td class="py-2 text-xs text-slate-700">{purpose}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    {if new-tech-introduced:}
    <div class="mt-6 p-4 bg-blue-50 border border-blue-200 rounded-xl">
      <strong class="text-blue-700">🆕 Tecnologías nuevas introducidas por este cambio:</strong>
      <ul class="list-disc list-inside text-blue-700 text-sm mt-2">{new-tech-list}</ul>
    </div>
    {endif}
  </div>
</section>
```

---

## Section 11: Casos de uso

Use case catalog with actor, preconditions, postconditions, and link to spec scenarios.

**Data sources**: spec.md (Given/When/Then scenarios), proposal.md (actors).

```html
<section id="use-cases">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">11 · Casos de uso</h2>
    <p class="text-sm text-slate-500 mb-6">Escenarios cubiertos por este cambio, derivados de la spec Given/When/Then.</p>

    <div class="space-y-3">
      {for each use case:}
      <details class="border border-slate-200 rounded-xl p-4">
        <summary class="flex items-center gap-3 cursor-pointer">
          <span class="text-emerald-500">●</span>
          <strong class="text-slate-800">{use-case-name}</strong>
          <span class="text-xs text-slate-500 ml-auto">{scenario-count} escenarios</span>
        </summary>
        <div class="mt-3 space-y-3 text-sm">
          <div class="grid grid-cols-3 gap-4">
            <div><strong class="text-slate-500">Actor principal</strong>: {actor}</div>
            <div><strong class="text-slate-500">Precondiciones</strong>: {preconditions}</div>
            <div><strong class="text-slate-500">Postcondiciones</strong>: {postconditions}</div>
          </div>
          <div>
            <strong class="text-slate-600">Escenarios (Given/When/Then)</strong>
            <ol class="list-decimal list-inside text-slate-700 mt-1 space-y-1 font-mono text-xs">
              {for each scenario:}
              <li><span class="text-slate-400">Given</span> {given} <span class="text-slate-400">When</span> {when} <span class="text-slate-400">Then</span> {then}</li>
            </ol>
          </div>
          <div>
            <strong class="text-slate-600">Cobertura de tests</strong>
            <ul class="list-disc list-inside text-slate-700 text-xs">
              {for each test type — unit, integration, e2e:}
              <li>{type}: {covered-by-test-id-or-link}</li>
            </ul>
          </div>
        </div>
      </details>
    </div>
  </div>
</section>
```

---

## Section 12: Flujos de secuencia (callbacks completos)

End-to-end sequence diagrams for the most important flows. **This is the section that explains "where does the callback go and what happens"**, the user's exact ask.

**Data sources**: design.md (flows), code (controller + service + repo paths from apply-progress), spec.md (scenarios).

```html
<section id="sequences">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">12 · Flujos de secuencia — Callbacks completos</h2>
    <p class="text-sm text-slate-500 mb-6">Diagramas de secuencia end-to-end. Cada flecha representa una llamada o evento. Los callbacks (callbacks de async, retries, eventos) están marcados con línea punteada.</p>

    {for each critical flow:}

    <details class="mb-6 border border-slate-200 rounded-xl" open>
      <summary class="p-4 bg-slate-50 cursor-pointer flex items-center gap-3">
        <span class="text-2xl">{flow-icon}</span>
        <div>
          <strong class="text-slate-800 block">{flow-name}</strong>
          <span class="text-xs text-slate-500">{flow-description}</span>
        </div>
      </summary>

      <div class="p-4">
        <figure aria-label="{escaped-sequence-description}">{sanitized-sequence-svg}</figure>

        <div class="mt-4 grid grid-cols-2 gap-4 text-sm">
          <div class="metric-card">
            <strong class="text-slate-600 text-xs uppercase tracking-wider">Path crítico</strong>
            <p class="text-xs text-slate-700 mt-1 font-mono">{critical-path}</p>
          </div>
          <div class="metric-card">
            <strong class="text-slate-600 text-xs uppercase tracking-wider">Latencia esperada</strong>
            <p class="text-xs text-slate-700 mt-1">{p50} p50 / {p95} p95</p>
          </div>
          <div class="metric-card">
            <strong class="text-slate-600 text-xs uppercase tracking-wider">Side effects</strong>
            <ul class="text-xs text-slate-700 list-disc list-inside">{side-effects}</ul>
          </div>
          <div class="metric-card">
            <strong class="text-slate-600 text-xs uppercase tracking-wider">Failure modes</strong>
            <ul class="text-xs text-slate-700 list-disc list-inside">{failure-modes}</ul>
          </div>
        </div>

        {if async-callbacks:}
        <details class="mt-3">
          <summary class="text-xs font-semibold text-slate-600 cursor-pointer"><span class="collapsible-arrow">▶</span> Async callbacks / webhooks</summary>
          <div class="mt-2 text-xs text-slate-600 space-y-1">
            {for each callback:}
            <div><span class="kbd">{callback-event}</span> → <span class="kbd">{callback-handler}</span> → {what-happens}</div>
          </div>
        </details>
        {endif}
      </div>
    </details>

  </div>
</section>
```

Sequence diagram guidelines:
- Use `autonumber` for ordered steps
- Mark synchronous calls with solid arrow `->>`
- Mark async/callback with dashed arrow `-->>`
- Include error paths with `alt ... else ... end` blocks
- Include `Note over` for important context

---

## Section 13: Modelo de datos (ER / schema)

Entity-relationship diagram for the database state. Or schema diagram if not relational.

```html
<section id="data-model">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">13 · Modelo de datos</h2>
    <p class="text-sm text-slate-500 mb-6">Esquema relacional y migraciones introducidas.</p>

    <figure aria-label="{escaped-data-model-description}">{sanitized-data-model-svg}</figure>

    <details class="mt-6">
      <summary class="text-sm font-semibold text-slate-700 cursor-pointer"><span class="collapsible-arrow">▶</span> Migraciones ({N} migraciones)</summary>
      <table class="w-full text-sm mt-3">
        <thead class="bg-slate-50">
          <tr><th class="text-left p-2">ID</th><th class="text-left p-2">Tabla</th><th class="text-left p-2">Operación</th><th class="text-left p-2">Reversible</th></tr>
        </thead>
        <tbody>
          {for each migration:}
          <tr class="border-b border-slate-100">
            <td class="p-2 font-mono text-xs">{migration-id}</td>
            <td class="p-2 font-mono text-xs">{table}</td>
            <td class="p-2">{operation}</td>
            <td class="p-2">{yes/no}</td>
          </tr>
        </tbody>
      </table>
    </details>
  </div>
</section>
```

---

## Section 14: Verificación

Test results, lens findings, entropy metrics, final verdict. Same as before but with entropy metrics added.

```html
<section id="verification">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-6">14 · Verificación</h2>

    <!-- Test pyramid -->
    <div class="grid grid-cols-4 gap-4 mb-8">
      <div class="text-center p-4 bg-slate-50 rounded-xl">
        <div class="text-2xl font-bold text-emerald-600">{unit-passed}</div>
        <div class="text-sm text-slate-500">Unit</div>
        <div class="text-xs text-slate-400">{unit-total} total</div>
      </div>
      <div class="text-center p-4 bg-slate-50 rounded-xl">
        <div class="text-2xl font-bold text-emerald-600">{integration-passed}</div>
        <div class="text-sm text-slate-500">Integration</div>
        <div class="text-xs text-slate-400">{integration-total} total</div>
      </div>
      <div class="text-center p-4 bg-slate-50 rounded-xl">
        <div class="text-2xl font-bold text-emerald-600">{e2e-passed}</div>
        <div class="text-sm text-slate-500">E2E</div>
        <div class="text-xs text-slate-400">{e2e-total} total</div>
      </div>
      <div class="text-center p-4 bg-slate-50 rounded-xl">
        <div class="text-2xl font-bold text-red-600">{failed}</div>
        <div class="text-sm text-slate-500">Fallaron</div>
      </div>
    </div>

    <!-- Entropy metrics -->
    <h3 class="text-lg font-semibold text-slate-700 mb-4">Métricas de diseño (entropy-sdd)</h3>
    <div class="grid grid-cols-4 gap-4 mb-6">
      <div class="metric-card">
        <div class="metric-value">{dqs-score}</div>
        <div class="metric-label">Design Quality Score</div>
      </div>
      <div class="metric-card">
        <div class="metric-value {connascence-color}">{connascence-count}</div>
        <div class="metric-label">Connascence crítica</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">{solid-violations}</div>
        <div class="metric-label">SOLID violations</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">{cohesion-score}</div>
        <div class="metric-label">Cohesion (avg)</div>
      </div>
    </div>

    <!-- Lenses -->
    <h3 class="text-lg font-semibold text-slate-700 mb-4">Lentes arquitectónicas aplicadas</h3>
    <div class="space-y-4">
      {for each lens:}
      <div class="border border-slate-200 rounded-xl p-4">
        <div class="flex items-center justify-between">
          <h4 class="font-semibold text-slate-800">{lens-name}</h4>
          <span class="tag badge-{pass|warn|fail} text-white text-xs">{verdict}</span>
        </div>
        <p class="text-sm text-slate-600 mt-2">{lens-finding}</p>
      </div>
    </div>

    <div class="mt-8 p-6 rounded-xl {verdict-bg}">
      <h3 class="text-xl font-bold {verdict-color}">Veredicto: {PASS / PASS WITH WARNINGS / FAIL}</h3>
      <p class="text-sm text-slate-600 mt-2">{verdict-detail}</p>
    </div>
  </div>
</section>
```

---

## Section 15: Git & Release

Same as before but with diff stats per area.

```html
<section id="git">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-6">15 · Git & Release</h2>

    <div class="grid grid-cols-2 gap-6 mb-6">
      <div class="space-y-4">
        <div class="flex items-center gap-4">
          <span class="text-sm font-semibold text-slate-400 w-24">Rama</span>
          <code class="font-mono text-sm bg-slate-100 px-3 py-1 rounded">{branch-name}</code>
        </div>
        <div class="flex items-center gap-4">
          <span class="text-sm font-semibold text-slate-400 w-24">PR</span>
          <a href="{pr-url}" class="text-blue-600 hover:underline font-mono text-sm">#{pr-number}</a>
        </div>
        <div class="flex items-center gap-4">
          <span class="text-sm font-semibold text-slate-400 w-24">Tag</span>
          <code class="font-mono text-sm bg-emerald-50 text-emerald-700 px-3 py-1 rounded">{tag}</code>
        </div>
      </div>
      <div class="space-y-4">
        <div class="flex items-center gap-4">
          <span class="text-sm font-semibold text-slate-400 w-24">Commits</span>
          <span class="text-slate-700">{commit-count}</span>
        </div>
        <div class="flex items-center gap-4">
          <span class="text-sm font-semibold text-slate-400 w-24">Archivos</span>
          <span class="text-slate-700">{file-count} ({additions} + / {deletions} -)</span>
        </div>
        <div class="flex items-center gap-4">
          <span class="text-sm font-semibold text-slate-400 w-24">Merge</span>
          <code class="font-mono text-xs bg-slate-100 px-3 py-1 rounded">git merge --no-ff {branch}</code>
        </div>
      </div>
    </div>

    <details>
      <summary class="text-sm font-semibold text-slate-700 cursor-pointer"><span class="collapsible-arrow">▶</span> Commits ({commit-count})</summary>
      <table class="w-full text-sm mt-3">
        <tbody>
          {for each commit:}
          <tr class="border-b border-slate-100">
            <td class="py-1 font-mono text-xs text-slate-400">{short-sha}</td>
            <td class="py-1"><span class="tag badge-{type} text-white text-xs">{type}</span></td>
            <td class="py-1 text-slate-700">{message}</td>
            <td class="py-1 text-xs text-slate-500">{files} files</td>
          </tr>
        </tbody>
      </table>
    </details>
  </div>
</section>
```

---

## Section 16: Impacto en conocimiento

Same as before.

```html
<section id="knowledge">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-6">16 · Impacto en conocimiento</h2>

    <div class="grid grid-cols-2 gap-6">
      <div>
        <h3 class="text-sm font-semibold text-emerald-600 uppercase tracking-wider mb-3">✓ Confirmado</h3>
        <ul class="space-y-2">{for each confirmed claim:}<li class="text-sm text-slate-700 flex items-start gap-2"><span class="text-emerald-500 mt-0.5">✓</span>{claim}</li>{endfor}</ul>
      </div>
      <div>
        <h3 class="text-sm font-semibold text-red-600 uppercase tracking-wider mb-3">✗ Contradicho</h3>
        <ul class="space-y-2">{for each contradicted claim:}<li class="text-sm text-slate-700 flex items-start gap-2"><span class="text-red-500 mt-0.5">✗</span>{claim}</li>{endfor}</ul>
      </div>
    </div>

    <div class="mt-6 grid grid-cols-2 gap-6">
      <div>
        <h3 class="text-sm font-semibold text-amber-600 uppercase tracking-wider mb-3">⬆ Promovido a durable</h3>
        <ul class="space-y-2">{for each promoted item:}<li class="text-sm text-slate-700 flex items-start gap-2"><span class="text-amber-500 mt-0.5">⬆</span>{item} → {target-artifact}</li>{endfor}</ul>
      </div>
      <div>
        <h3 class="text-sm font-semibold text-purple-600 uppercase tracking-wider mb-3">🧠 Solo memoria (Engram)</h3>
        <ul class="space-y-2">{for each memory-only item:}<li class="text-sm text-slate-700 flex items-start gap-2"><span class="text-purple-500 mt-0.5">🧠</span>{item}</li>{endfor}</ul>
      </div>
    </div>
  </div>
</section>
```

---

## Section 17: Deuda técnica

Tech debt created or discovered, with plan to address it. This is critical for an architect's view.

```html
<section id="tech-debt">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">17 · Deuda técnica (sddk-debt-verify)</h2>
    <p class="text-sm text-slate-500 mb-6">Auditoría post-verify sobre <code>{base-commit}...{head-commit}</code>. Veredicto: <strong class="text-{verdict-color}">{debt-verdict}</strong> · Cobertura: <strong>{clusters-completed}/{clusters-required}</strong> · JSON: <code>{debt-json-sha256}</code></p>

    {if no debt:}
    <div class="p-4 bg-emerald-50 border border-emerald-200 rounded-xl text-emerald-700">
      No se introdujo deuda técnica bloqueante. Clusters completos: {clusters-completed}/{clusters-required}.
    </div>
    {else:}
    <div class="mb-6 overflow-x-auto">
      <table class="w-full text-sm">
        <thead><tr class="border-b-2 border-slate-200">
          <th class="text-left py-2">Cluster</th>
          <th class="text-left py-2">Estado</th>
          <th class="text-right py-2">Critical</th>
          <th class="text-right py-2">High</th>
          <th class="text-right py-2">Medium</th>
          <th class="text-right py-2">Low</th>
          <th class="text-left py-2">Errores</th>
        </tr></thead>
        <tbody>
          {for each cluster verdict:}
          <tr class="border-b border-slate-100">
            <td class="py-2 font-semibold">{cluster-name}</td>
            <td class="py-2"><span class="risk-{cluster-status}">{cluster-status}</span></td>
            <td class="py-2 text-right">{cluster-critical}</td>
            <td class="py-2 text-right">{cluster-high}</td>
            <td class="py-2 text-right">{cluster-medium}</td>
            <td class="py-2 text-right">{cluster-low}</td>
            <td class="py-2 text-xs text-slate-600">{cluster-errors}</td>
          </tr>
          {endfor}
          <tr class="bg-slate-50 font-bold">
            <td class="py-2">TOTAL</td>
            <td class="py-2"><span class="risk-{debt-verdict}">{debt-verdict}</span></td>
            <td class="py-2 text-right">{total-critical}</td>
            <td class="py-2 text-right">{total-high}</td>
            <td class="py-2 text-right">{total-medium}</td>
            <td class="py-2 text-right">{total-low}</td>
            <td class="py-2 text-xs">Introduced: {total-introduced} · Pre-existing: {total-pre-existing} · Unknown: {total-unknown}</td>
          </tr>
        </tbody>
      </table>
    </div>

    {if re-iterate-from=apply:}
    <div class="p-4 bg-amber-50 border border-amber-200 rounded-xl text-amber-800 mb-4">
      <strong>Remediación en curso:</strong> misma rama del ciclo, <code class="font-mono">remediation_round={remediation_round}</code> (máximo 3). Aplica fixes, re-verifica y vuelve a ejecutar debt-verify.
    </div>
    {endif}

    {if debt-verdict=INCONCLUSIVE:}
    <div class="p-4 bg-red-50 border border-red-200 rounded-xl text-red-800 mb-4">
      <strong>Auditoría inconclusa.</strong> Hay clusters requeridos incompletos o evidencia inválida. Release permanece bloqueado hasta reintento o revisión humana.
    </div>
    {endif}

    {if pre-existing-debt:}
    <div class="p-4 bg-orange-50 border border-orange-200 rounded-xl text-orange-800 mb-4">
      <strong>Deuda preexistente detectada.</strong> Permanece visible y requiere incidencias con owner y prioridad, pero no cuenta como deuda introducida por este cambio.
    </div>
    {endif}

    <div class="space-y-3">
      {for each debt item:}
      <div class="risk-row">
        <span class="risk-{severity}">{severity}</span>
        <div>
          <strong class="text-slate-800">{debt-title}</strong>
          <p class="text-xs text-slate-600 mt-1">{debt-description}</p>
          <p class="text-xs text-slate-500 mt-1 font-mono">{location} · {fingerprint}</p>
          <p class="text-xs text-slate-600 mt-1">Confianza: {confidence} · Atribución: {attribution}</p>
          {if solid-violation:}
          <p class="text-xs text-purple-700 mt-1">SOLID: <strong>{solid-principle}</strong> · cluster: {cluster-name}</p>
          {endif}
        </div>
        <div class="text-xs text-right">
          <div class="text-slate-500">Remediación</div>
          <div>{remediation-link-or-id}</div>
        </div>
      </div>
    </div>
    {endif}

    <details class="mt-6">
      <summary class="text-sm font-semibold text-slate-700 cursor-pointer"><span class="collapsible-arrow">▶</span> Áreas de mejora detectadas (no bloqueantes)</summary>
      <ul class="list-disc list-inside text-sm text-slate-700 mt-3 space-y-1">
        {for each improvement:}
        <li>{improvement-description}</li>
      </ul>
    </details>

    <details class="mt-3">
      <summary class="text-sm font-semibold text-slate-700 cursor-pointer"><span class="collapsible-arrow">▶</span> Skill coverage por cluster (qué skills se cargaron)</summary>
      <ul class="list-disc list-inside text-xs text-slate-700 mt-3 space-y-1">
        {for each cluster:}
        <li><strong>{cluster-name}</strong>: {skills-list}</li>
        {endfor}
      </ul>
    </details>
  </div>
</section>
```

---

## Section 18: Características operacionales

Operational profile: observability, failure modes, recovery, scaling.

```html
<section id="ops">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">18 · Características operacionales</h2>
    <p class="text-sm text-slate-500 mb-6">Cómo se observa, recupera y escala este cambio.</p>

    <div class="grid grid-cols-2 gap-6">
      <div>
        <h3 class="text-sm font-semibold text-slate-500 uppercase tracking-wider mb-3">Observabilidad</h3>
        <table class="w-full text-sm">
          <tbody>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Logs estructurados</td><td class="py-2">{yes/no — formato}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Métricas (RED/USE)</td><td class="py-2">{metrics-emitted}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Distributed tracing</td><td class="py-2">{tracing — OTel/Honeycomb/etc}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Health checks</td><td class="py-2">{endpoints}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Alertas</td><td class="py-2">{alert-rules}</td></tr>
          </tbody>
        </table>
      </div>

      <div>
        <h3 class="text-sm font-semibold text-slate-500 uppercase tracking-wider mb-3">Resiliencia</h3>
        <table class="w-full text-sm">
          <tbody>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Failure modes</td><td class="py-2">{list}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Retry policy</td><td class="py-2">{retry-config}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Circuit breakers</td><td class="py-2">{configured-for}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Timeouts</td><td class="py-2">{timeouts}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Rollback plan</td><td class="py-2">{rollback-steps}</td></tr>
          </tbody>
        </table>
      </div>

      <div>
        <h3 class="text-sm font-semibold text-slate-500 uppercase tracking-wider mb-3">Escalado</h3>
        <table class="w-full text-sm">
          <tbody>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Horizontal scaling</td><td class="py-2">{strategy}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Vertical scaling</td><td class="py-2">{limits}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Bottlenecks</td><td class="py-2">{bottlenecks}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Capacity planning</td><td class="py-2">{capacity-numbers}</td></tr>
          </tbody>
        </table>
      </div>

      <div>
        <h3 class="text-sm font-semibold text-slate-500 uppercase tracking-wider mb-3">Seguridad</h3>
        <table class="w-full text-sm">
          <tbody>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Auth</td><td class="py-2">{auth-method}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Authz</td><td class="py-2">{authz-model}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Secrets</td><td class="py-2">{secrets-storage}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">PII handling</td><td class="py-2">{pii-policy}</td></tr>
            <tr class="border-b border-slate-100"><td class="py-2 text-slate-600">Threat model</td><td class="py-2">{threats}</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</section>
```

---

## Section 19: Métricas & coste

Pipeline cost & efficiency metrics. Comes from telemetry.

```html
<section id="metrics">
  <div class="card p-8">
    <h2 class="text-2xl font-bold text-slate-800 mb-2">19 · Métricas & coste del pipeline</h2>
    <p class="text-sm text-slate-500 mb-6">Datos agregados de `~/.local/share/opencode/telemetry/events.log`.</p>

    <div class="grid grid-cols-5 gap-4 mb-8">
      <div class="metric-card">
        <div class="metric-value">{total-duration-min}m</div>
        <div class="metric-label">Duración total</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">${cost-usd}</div>
        <div class="metric-label">Coste LLM total</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">{tokens-in}M</div>
        <div class="metric-label">Tokens entrada</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">{tokens-out}M</div>
        <div class="metric-label">Tokens salida</div>
      </div>
      <div class="metric-card">
        <div class="metric-value">{coherence-final}</div>
        <div class="metric-label">Coherence final</div>
      </div>
    </div>

    <!-- Per-phase table -->
    <h3 class="text-lg font-semibold text-slate-700 mb-4">Por fase</h3>
    <div class="overflow-x-auto">
      <table class="w-full text-sm">
        <thead class="bg-slate-50">
          <tr><th class="text-left p-2">Fase</th><th class="text-left p-2">Duración</th><th class="text-left p-2">Coste</th><th class="text-left p-2">Tokens in</th><th class="text-left p-2">Tokens out</th><th class="text-left p-2">Modelo</th><th class="text-left p-2">Coherence</th></tr>
        </thead>
        <tbody>
          {for each phase:}
          <tr class="border-b border-slate-100">
            <td class="p-2 font-mono text-xs">{phase}</td>
            <td class="p-2">{duration}</td>
            <td class="p-2">${cost}</td>
            <td class="p-2">{tokens-in}</td>
            <td class="p-2">{tokens-out}</td>
            <td class="p-2 text-xs">{model}</td>
            <td class="p-2">{coherence}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</section>
```

---

## Section 20: Próximos pasos

Same as before, but with roadmap position.

```html
<section id="next-steps">
  <div class="section-dark rounded-2xl p-10 text-center space-y-4">
    <h2 class="text-2xl font-bold">20 · Próximos pasos</h2>
    <p class="text-slate-300">{next-milestone-description}</p>
    <div class="flex justify-center gap-4 pt-2 flex-wrap">
      <a href="{roadmap-ref}" class="inline-block bg-white/10 hover:bg-white/20 text-white px-6 py-2 rounded-xl text-sm font-medium transition">📋 ROADMAP</a>
      <a href="{adr-dir}" class="inline-block bg-white/10 hover:bg-white/20 text-white px-6 py-2 rounded-xl text-sm font-medium transition">📐 ADRs</a>
      <a href="{related-issues}" class="inline-block bg-white/10 hover:bg-white/20 text-white px-6 py-2 rounded-xl text-sm font-medium transition">🔗 Related issues</a>
      <a href="telemetry-dashboard.html" class="inline-block bg-white/10 hover:bg-white/20 text-white px-6 py-2 rounded-xl text-sm font-medium transition">📊 Dashboard</a>
    </div>
  </div>
</section>
```

---

## Data Collection Pipeline

The archive agent must collect this data before rendering. Order matters:

```
1. Read artifact-registry for change metadata
   → resolve `{cycle-artifacts-dir}` from the launch context
   → read each required XDG artifact by its declared path

2. Read `{cycle-artifacts-dir}` for full artifacts
   → explore-report, proposal, spec, design, tasks
   → apply-progress (file lists, commit mapping)
   → verify-report (lens verdicts)
   → archive-report (final stats)

3. Run git analysis
   → git log --oneline {base}..HEAD
   → git diff --stat {base}..HEAD
   → git show --format=fuller {commits}

4. Static analysis (if tools available)
   → cognicode_get_hot_paths
   → cognicode_check_architecture
   → entropy_sdd connascence + SOLID metrics

5. Parse telemetry
   → ~/.local/share/opencode/telemetry/events.log
   → filter by change_name, group by phase
   → sum duration_ms, cost_usd, tokens_in, tokens_out

6. Consume the evidence-bound architecture manifest when C4 was selected
   → Use validated rich output or the table fallback from `sddk-c4-likec4`

7. Render HTML with pre-rendered diagrams/fallback tables + structured data

8. Save; open only on explicit request
```

---

## Color Reference

| Element | Gradient |
|---------|----------|
| feat badge | `#059669 → #047857` |
| fix badge | `#dc2626 → #b91c1c` |
| refactor badge | `#2563eb → #1d4ed8` |
| chore badge | `#6b7280 → #4b5563` |
| PASS verdict | `#059669 → #047857` |
| WARN verdict | `#d97706 → #b45309` |
| FAIL verdict | `#dc2626 → #b91c1c` |
| Dark section bg | `#1e293b → #0f172a` |
| Blueprint section bg | `#f8fafc → #f1f5f9` |
| Domain layer fill | `#d1fae5 → #065f46` |
| Application layer fill | `#e0e7ff → #3730a3` |
| Adapter layer fill | `#fce7f3 → #9d174d` |
| Infrastructure fill | `#fef3c7 → #92400e` |

---

## Browser Opening

Open only after an explicit user request:
- Linux: `xdg-open <path>`
- macOS: `open <path>`
- Windows: `start <path>`

Always tell the user the absolute path without opening it automatically.

---

## Notes for the archive agent

1. **Missing evidence stays visible**: omit only evidence-backed `N/A`; render missing required sources with impact and recovery.
2. **Architecture is consumed, not invented**: C4 sections use validated output from `skills/sddk-c4-likec4/SKILL.md`. Embed pre-rendered SVG or the skill's table fallback; generic examples never create relationships.
3. **Tables are preferred for compactness**: details/summary for long content.
4. **The hero section is mandatory** — it's the only section that always renders.
5. **The TOC sidebar updates on scroll** via vanilla JS (already in scaffold).
6. **Self-contained**: inline every required asset. The final file must work offline with no prior network load.
7. **Semantic/render separation**: rich rendering failure selects the fallback and never changes semantic status or verification verdict.
8. **Stable across locales**: IDs, verdicts, counts, hashes, paths, commands, and evidence are byte-identical across localized projections.
