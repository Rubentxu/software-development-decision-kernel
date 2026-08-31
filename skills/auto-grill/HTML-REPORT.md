# HTML Report Format

**IDIOMA: El reporte debe ser siempre en español.**

The auto-grill report is a self-contained HTML file with 4 sections. Tailwind and Mermaid from CDNs. Visual, not corporate. Uses the same editorial style as improve-codebase-architecture's reports.

## Scaffold

```html
<!doctype html>
<html lang="es">
  <head>
    <meta charset="utf-8" />
    <title>Auto-Grill — {input name}</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <script src="https://cdn.tailwindcss.com"></script>
    <script type="module">
      import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
      mermaid.initialize({ startOnLoad: true, theme: "neutral", securityLevel: "loose" });
    </script>
    <style>
      * { font-family: 'Inter', system-ui, sans-serif; }
      .font-mono { font-family: 'JetBrains Mono', monospace; }
      .section-dark { background: linear-gradient(135deg, #1e293b, #0f172a); }
      .card { background: white; border-radius: 16px; border: 1px solid #e2e8f0; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.05); }
      .os-green { background: linear-gradient(135deg, #059669, #047857); }
      .os-yellow { background: linear-gradient(135deg, #d97706, #b45309); }
      .os-orange { background: linear-gradient(135deg, #ea580c, #c2410c); }
      .os-red { background: linear-gradient(135deg, #dc2626, #b91c1c); }
      .auto-badge { background: linear-gradient(135deg, #2563eb, #1d4ed8); }
      .escalated-badge { background: linear-gradient(135deg, #9333ea, #7e22ce); }
      .opportunity-funcional { border-left: 4px solid #059669; }
      .opportunity-tecnico { border-left: 4px solid #2563eb; }
      .opportunity-negocio { border-left: 4px solid #d97706; }
    </style>
  </head>
  <body class="bg-gradient-to-br from-slate-100 to-stone-100 min-h-screen">
    <main class="max-w-6xl mx-auto px-6 py-12 space-y-10">

      <!-- SECTION 1: Executive Summary -->
      <!-- SECTION 2: Auto-Resolved Decisions -->
      <!-- SECTION 3: Escalated Decisions (with OS) -->
      <!-- SECTION 4: Pending Validation Checklist -->

    </main>
  </body>
</html>
```

## Section 1: Executive Summary

One card with stats and a visual breakdown.

```html
<section id="summary">
  <div class="card p-8 text-center">
    <h2 class="text-3xl font-bold text-slate-800 mb-6">Auto-Grill: {input name}</h2>
    
    <!-- Stats row -->
    <div class="flex justify-center gap-8 mb-6">
      <div class="text-center">
        <div class="text-4xl font-bold text-blue-600">{N}</div>
        <div class="text-sm text-slate-500">Preguntas</div>
      </div>
      <div class="text-center">
        <div class="text-4xl font-bold text-emerald-600">{M}</div>
        <div class="text-sm text-slate-500">Auto-resueltas</div>
      </div>
      <div class="text-center">
        <div class="text-4xl font-bold text-purple-600">{K}</div>
        <div class="text-sm text-slate-500">Escaladas</div>
      </div>
    </div>

    <!-- Progress bar -->
    <div class="w-full bg-slate-200 rounded-full h-4 mb-2">
      <div class="bg-gradient-to-r from-emerald-500 to-blue-500 h-4 rounded-full" 
           style="width: {rate}%"></div>
    </div>
    <p class="text-sm text-slate-500">{rate}% resuelto automáticamente</p>
  </div>
</section>
```

## Section 2: Auto-Resolved Decisions

Expandable cards, one per decision. Collapsed by default — user expands to see evidence.

```html
<section id="auto-resolved" class="space-y-4">
  <h2 class="text-2xl font-bold text-slate-800 mb-4">Decisiones Automáticas</h2>

  <!-- Each decision -->
  <details class="card overflow-hidden group">
    <summary class="px-6 py-4 cursor-pointer hover:bg-slate-50 flex items-center gap-3">
      <span class="auto-badge text-white text-xs font-bold px-2 py-1 rounded-full">Auto</span>
      <span class="text-sm font-medium text-slate-700 flex-1">{pregunta}</span>
      <span class="text-xs text-emerald-600 font-semibold">Confianza: {0.95}</span>
      <span class="text-xs text-slate-400">▶ expandir</span>
    </summary>
    <div class="px-6 pb-4 border-t border-slate-100">
      <div class="mt-3 grid md:grid-cols-2 gap-4">
        <div>
          <div class="text-xs uppercase tracking-wider text-slate-400 mb-1">Resolución</div>
          <p class="text-sm text-slate-700">{resolución}</p>
        </div>
        <div>
          <div class="text-xs uppercase tracking-wider text-slate-400 mb-1">Evidencia</div>
          <p class="text-xs font-mono text-slate-600">{evidencia}: línea X</p>
        </div>
      </div>
      <div class="mt-3">
        <div class="text-xs uppercase tracking-wider text-slate-400 mb-1">Método</div>
        <p class="text-xs text-slate-500">{CogniCode / Heuristic} — {confianza}</p>
      </div>
    </div>
  </details>

  <!-- More decisions... -->
</section>
```

## Section 3: Escalated Decisions with Opportunity Score

This is the centrepiece. Each escalated decision gets a full card with:
- Options table with OS scores (color-coded)
- Opportunity breakdown by type (Funcional/Técnico/Negocio)
- Agent recommendation
- Comparison visual

```html
<section id="escalated" class="space-y-8">
  <h2 class="text-2xl font-bold text-slate-800 mb-4">Decisiones que Requieren tu Validación</h2>

  <!-- Each escalated decision -->
  <article class="card overflow-hidden">
    <!-- Header -->
    <div class="bg-gradient-to-r from-purple-800 to-purple-700 px-6 py-4">
      <div class="flex items-center gap-3">
        <span class="escalated-badge text-white text-xs font-bold px-2 py-1 rounded-full">Escalada #{N}</span>
        <h3 class="text-lg font-semibold text-white">{pregunta}</h3>
      </div>
      <p class="text-purple-200 text-sm mt-2">{contexto de la decisión}</p>
    </div>

    <div class="p-6">
      <!-- Options comparison table -->
      <div class="overflow-x-auto mb-6">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-slate-400 uppercase tracking-wider text-xs">
              <th class="text-left pb-3">Opción</th>
              <th class="text-center pb-3">ΔI(bits)</th>
              <th class="text-center pb-3">ΔF</th>
              <th class="text-center pb-3">Apertura</th>
              <th class="text-center pb-3">Flexibilidad</th>
              <th class="text-center pb-3">Depth</th>
              <th class="text-center pb-3">Revers.</th>
              <th class="text-center pb-3">OS</th>
            </tr>
          </thead>
          <tbody>
            <!-- Best option (highlighted) -->
            <tr class="bg-emerald-50 border-l-4 border-emerald-500">
              <td class="py-3 font-semibold text-emerald-700">A: {nombre}</td>
              <td class="py-3 text-center">{+0.8}</td>
              <td class="py-3 text-center">{-0.3}</td>
              <td class="py-3 text-center">{0.9}</td>
              <td class="py-3 text-center">{5 esc.}</td>
              <td class="py-3 text-center">{0.7}</td>
              <td class="py-3 text-center">{0.9}</td>
              <td class="py-3 text-center">
                <span class="os-green text-white text-xs font-bold px-2 py-1 rounded-full">0.82</span>
              </td>
            </tr>
            <!-- Other options -->
            <tr class="border-t border-slate-100">
              <td class="py-3 text-slate-600">B: {nombre}</td>
              <td class="py-3 text-center text-slate-500">{+1.5}</td>
              <td class="py-3 text-center text-slate-500">{-0.1}</td>
              <td class="py-3 text-center text-slate-500">{0.7}</td>
              <td class="py-3 text-center text-slate-500">{3 esc.}</td>
              <td class="py-3 text-center text-slate-500">{0.5}</td>
              <td class="py-3 text-center text-slate-500">{0.7}</td>
              <td class="py-3 text-center">
                <span class="os-yellow text-white text-xs font-bold px-2 py-1 rounded-full">0.65</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Opportunities for RECOMMENDED option -->
      <div class="bg-emerald-50 border border-emerald-200 rounded-xl p-4 mb-4">
        <div class="flex items-center gap-2 mb-3">
          <span class="os-green text-white text-xs font-bold px-2 py-1 rounded-full">RECOMENDADA</span>
          <span class="text-sm font-semibold text-emerald-700">Opción A — OS: 0.82</span>
        </div>
        <div class="text-xs uppercase tracking-wider text-emerald-600 font-semibold mb-2">Oportunidades habilitadas</div>
        <div class="space-y-2">
          <!-- Funcional -->
          <div class="opportunity-funcional bg-white rounded-lg px-3 py-2 pl-4">
            <div class="flex items-center gap-2">
              <span class="text-xs">🎯</span>
              <span class="text-xs font-semibold text-emerald-700">Funcional</span>
              <span class="text-xs text-slate-600">{descripción de la oportunidad}</span>
            </div>
          </div>
          <!-- Técnico -->
          <div class="opportunity-tecnico bg-white rounded-lg px-3 py-2 pl-4">
            <div class="flex items-center gap-2">
              <span class="text-xs">🔧</span>
              <span class="text-xs font-semibold text-blue-700">Técnico</span>
              <span class="text-xs text-slate-600">{descripción de la oportunidad}</span>
            </div>
          </div>
          <!-- Negocio -->
          <div class="opportunity-negocio bg-white rounded-lg px-3 py-2 pl-4">
            <div class="flex items-center gap-2">
              <span class="text-xs">💼</span>
              <span class="text-xs font-semibold text-amber-700">Negocio</span>
              <span class="text-xs text-slate-600">{descripción de la oportunidad}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Agent recommendation -->
      <div class="bg-slate-50 rounded-xl p-4">
        <div class="text-xs uppercase tracking-wider text-slate-400 font-semibold mb-1">Recomendación del agente</div>
        <p class="text-sm text-slate-700">{recomendación con rationale}</p>
      </div>
    </div>
  </article>
</section>
```

## Section 4: Pending Validation Checklist

Interactive section where the user can approve or reject each decision.

```html
<section id="validation" class="section-dark rounded-2xl p-8 shadow-xl">
  <h2 class="text-2xl font-bold text-white mb-6">Validación Pendiente</h2>
  <p class="text-slate-400 text-sm mb-6">Revisa cada decisión. Las auto-resueltas están pre-aprobadas. Las escaladas necesitan tu confirmación.</p>

  <div class="space-y-4">
    <!-- Auto-resolved (pre-approved) -->
    <div class="flex items-center gap-3 bg-slate-700/50 rounded-lg px-4 py-3">
      <input type="checkbox" checked disabled class="w-5 h-5 rounded accent-emerald-500">
      <span class="text-xs auto-badge text-white px-2 py-0.5 rounded">Auto</span>
      <span class="text-sm text-slate-300 flex-1">{pregunta resumida}</span>
      <span class="text-xs text-emerald-400">✓ Pre-aprobada</span>
    </div>

    <!-- Escalated (needs confirmation) -->
    <div class="flex items-center gap-3 bg-purple-900/30 border border-purple-500/30 rounded-lg px-4 py-3">
      <input type="checkbox" class="w-5 h-5 rounded accent-purple-500">
      <span class="text-xs escalated-badge text-white px-2 py-0.5 rounded">#{N}</span>
      <span class="text-sm text-white flex-1">{pregunta escalada}</span>
      <span class="text-xs text-purple-300">⬜ Pendiente</span>
    </div>
  </div>

  <!-- Summary action -->
  <div class="mt-6 p-4 bg-slate-700/50 rounded-xl">
    <div class="flex items-center justify-between">
      <div>
        <span class="text-sm text-slate-300">Auto-aprobadas: <strong class="text-emerald-400">{M}</strong></span>
        <span class="text-slate-600 mx-2">|</span>
        <span class="text-sm text-slate-300">Pendientes: <strong class="text-purple-400">{K}</strong></span>
      </div>
      <div class="text-sm text-slate-400">
        Confirma las pendientes para continuar
      </div>
    </div>
  </div>
</section>
```

## Style Guidance

- Same editorial style as improve-codebase-architecture reports
- Generous whitespace, serif optional for headings
- Colour: emerald accent + purple for escalated + amber for warnings
- Opportunity types have consistent left-border colours:
  - 🎯 Funcional = emerald (border-left: 4px solid #059669)
  - 🔧 Técnico = blue (border-left: 4px solid #2563eb)
  - 💼 Negocio = amber (border-left: 4px solid #d97706)
- OS badges colour-coded: green (>0.7), yellow (0.4-0.7), orange (0.2-0.4), red (<0.2)
- All text in Spanish
- `lang="es"` on html element
