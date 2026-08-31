/* Renderers (vanilla): matrix table, traceability rollup, status pills,
 * and the v2 wizard primitives (context bar, pre-flight, typed step,
 * typed evidence, failure protocol, teardown).
 */

const UatRender = (() => {
  function pill(status) {
    const cls = String(status || "PENDING").toLowerCase();
    return `<span class="status-pill ${cls}">${status || "PENDING"}</span>`;
  }

  function escapeHtml(s) {
    return String(s == null ? "" : s)
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
  }

  function statusOf(session, scenarioId) {
    if (!session) return "PENDING";
    if (Array.isArray(session.results)) {
      const r = session.results.find(r => r && r.scenario_id === scenarioId);
      if (r && r.status) return r.status;
    }
    if (session.scenario_results && session.scenario_results[scenarioId]) {
      return session.scenario_results[scenarioId].status || "PENDING";
    }
    return "PENDING";
  }

  function matrix(plan, session) {
    const rows = [];
    let i = 0;
    for (const feature of plan.features || []) {
      for (const sc of feature.scenarios || []) {
        rows.push(
          `<tr style="--i:${i++}">` +
            `<td class="id">${escapeHtml(sc.id)}</td>` +
            `<td>${escapeHtml(feature.name)}</td>` +
            `<td>${escapeHtml(sc.title)}</td>` +
            `<td>${(sc.priority || "").toUpperCase()}</td>` +
            `<td>${escapeHtml(sc.assignee || "developer")}</td>` +
            `<td>${pill(statusOf(session, sc.id))}</td>` +
          `</tr>`
        );
      }
    }
    const table = document.createElement("table");
    table.className = "data-table";
    table.innerHTML =
      "<thead><tr><th class=\"id\">ID</th><th>Feature</th><th>Escenario</th><th>Prioridad</th><th>Assignee</th><th>Estado</th></tr></thead>" +
      "<tbody>" + rows.join("") + "</tbody>";
    return table;
  }

  function traceability(plan, session) {
    const blocks = [];
    let featureIdx = 0;
    for (const feature of plan.features || []) {
      const total = (feature.scenarios || []).length;
      let covered = 0;
      const scRows = (feature.scenarios || []).map(sc => {
        const st = statusOf(session, sc.id);
        if (st !== "PENDING") covered++;
        return `<tr><td class="id">${escapeHtml(sc.id)}</td><td>${escapeHtml(sc.title)}</td><td>${pill(st)}</td></tr>`;
      }).join("");
      const pct = total ? Math.round((100 * covered) / total) : 0;
      blocks.push(
        `<section class="trace-feature" style="--i:${featureIdx++}">` +
          `<header style="display:flex;align-items:baseline;justify-content:space-between;gap:var(--space-3);margin-bottom:var(--space-3)">` +
            `<h3 style="font-size:var(--text-md)">${escapeHtml(feature.id)} — ${escapeHtml(feature.name)}` +
              (feature.requirement_ref ? ` <span style="color:var(--text-dim);font-family:var(--font-mono);font-weight:400;font-size:var(--text-sm)">(${escapeHtml(feature.requirement_ref)})</span>` : "") +
            `</h3>` +
            `<span class="progress-label">coverage ${pct}% <span style="color:var(--text-dim)">(${covered}/${total})</span></span>` +
          `</header>` +
          `<table class="data-table"><thead><tr><th class="id\">ID</th><th>Escenario</th><th>Estado</th></tr></thead><tbody>${scRows}</tbody></table>` +
        `</section>`
      );
    }
    if (blocks.length === 0) return `<p class="page-sub">Plan vacío: no hay features para mostrar.</p>`;
    return `<div style="display:grid;gap:var(--space-5)">${blocks.join("")}</div>`;
  }

  function kpis(plan) {
    let scenarios = 0, p0 = 0;
    for (const feature of plan.features || []) {
      for (const sc of feature.scenarios || []) {
        scenarios++;
        if (sc.priority === "P0") p0++;
      }
    }
    return [
      { v: scenarios, l: "escenarios" },
      { v: p0, l: "P0" },
      { v: (plan.features || []).length, l: "features" },
    ].map(k => `<div class="kpi"><div class="v">${k.v}</div><div class="l">${k.l}</div></div>`).join("");
  }

  function userStoryBanner(scenario) {
    const story = scenario.context && scenario.context.user_story;
    if (!story || !story.trim()) return "";
    return `<div class="user-story-banner" style="--i:0"><span class="user-story-label">Intención</span><p>${escapeHtml(story)}</p></div>`;
  }

  function preflightChecklist(scenario) {
    const items = (scenario.context && scenario.context.preconditions) || [];
    if (!items.length) return "";
    const rows = items.map((p, i) =>
      `<li class="preflight-item" style="--i:${i}"><label class="preflight-check"><input type="checkbox" class="preflight-cb" data-preflight="${escapeHtml(p)}"><span>${escapeHtml(p)}</span></label><button class="copy-btn" data-copy="${escapeHtml(p)}">copiar</button></li>`
    ).join("");
    return `<section class="preflight" style="--i:1"><header class="preflight-head"><h3 class="preflight-title">Pre-flight</h3><span class="preflight-hint">Marca cada requisito antes de empezar</span></header><ul class="preflight-list">${rows}</ul></section>`;
  }

  function contextBar(scenario, feature) {
    const timing = scenario.context && scenario.context.timing;
    const risk = scenario.risk;
    const help = scenario.context && scenario.context.help;
    const window = (timing && timing.window) || (scenario.flags && scenario.flags.includes("smoke") ? "smoke" : "regression");
    const est = scenario.est_minutes || 0;
    const ceiling = (timing && timing.timeout_min) || Math.max(est * 2, 5);
    const riskCls = risk ? `risk-${(risk.classification || "medium").toLowerCase()}` : "";
    const riskTxt = risk ? (risk.classification || "medium") : "—";
    const helpParts = [];
    if (help) {
      if (help.slack && help.slack.length) helpParts.push(`<span class="ctx-help-item">${escapeHtml(help.slack.join(" · "))}</span>`);
      if (help.contacts && help.contacts.length) helpParts.push(`<span class="ctx-help-item">${escapeHtml(help.contacts.join(" · "))}</span>`);
      if (help.related_adrs && help.related_adrs.length) helpParts.push(`<span class="ctx-help-item">${escapeHtml(help.related_adrs.join(" · "))}</span>`);
      if (help.docs && help.docs.length) helpParts.push(`<a class="ctx-help-item ctx-help-link" href="#">${escapeHtml(help.docs[0])}</a>`);
    }
    if (feature && feature.requirement_ref) {
      helpParts.push(`<span class="ctx-help-item"><code>${escapeHtml(feature.requirement_ref)}</code></span>`);
    }
    return `<div class="context-bar ${riskCls}" style="--i:0"><div class="ctx-pair"><span class="ctx-label">window</span><span class="ctx-val">${escapeHtml(window)}</span></div><div class="ctx-pair"><span class="ctx-label">est / ceiling</span><span class="ctx-val">${est}m / ${ceiling}m</span></div><div class="ctx-pair"><span class="ctx-label">risk</span><span class="ctx-val">${escapeHtml(riskTxt)}</span></div><div class="ctx-pair ctx-help">${helpParts.join("") || "<span class='ctx-help-item'>—</span>"}</div></div>`;
  }

  function stepBlock(step, i) {
    const kind = (step.kind || "shell").toLowerCase();
    const action = escapeHtml(step.action || "");
    const expected = escapeHtml(step.expected || "");
    const number = step.step || (i + 1);
    const copyHint = step.copy_hint !== false && (kind === "shell" || kind === "api" || kind === "file");
    const header = `<div class="step-head"><span class="step-num">${number}</span><span class="step-kind step-kind-${kind}">${escapeHtml(kind)}</span>${copyHint ? `<button class="copy-btn" data-copy="${escapeHtml(step.action || "")}">copiar</button>` : ""}</div>`;
    let body;
    if (kind === "shell" || kind === "api") {
      body = `<pre class="step-code">${escapeHtml(step.action || "")}</pre>`;
    } else {
      body = `<p class="step-prose">${escapeHtml(step.action || "")}</p>`;
    }
    return `<li class="scenario-step step-${kind}" style="--i:${i}">${header}${body}<p class="step-expected"><strong>Esperado</strong>${expected}</p></li>`;
  }

  // --- Form DSL (ADR-015 / REQ-RF-025) ---
  // Render determinista de items del formulario: checks (incl. blind checks
  // con expected oculto), informativos y flujo. Los agentes generan la spec;
  // este compilador dibuja. Nunca HTML arbitrario de agentes.

  function checkControl(check) {
    const name = "check-" + Math.random().toString(36).slice(2, 8);
    const opts = (check.options || []).map((o, i) =>
      `<label class="form-option"><input type="radio" name="${name}" value="${escapeHtml(o)}" data-blind="${check.visibility === "blind" ? "1" : ""}"><span>${escapeHtml(o)}</span></label>`
    ).join("");
    const rating = (check.kind === "rating" && !opts)
      ? `<div class="form-rating">${[1,2,3,4,5].map(n => `<label class="form-rating-star"><input type="radio" name="${name}" value="${n}"><span>${n}</span></label>`).join("")}</div>`
      : "";
    const text = (check.kind === "text" || check.kind === "textarea")
      ? `<textarea class="form-text" name="${name}" rows="${check.kind === "textarea" ? 3 : 1}" placeholder="Tu respuesta…"></textarea>`
      : "";
    const num = (check.kind === "number")
      ? `<input class="form-text" type="number" name="${name}">`
      : "";
    const yesNo = (check.kind === "yes_no" || check.kind === "confirm" || check.kind === "pass_fail")
      ? `<div class="form-yesno">${["Sí", "No"].map(v => `<label class="form-option"><input type="radio" name="${name}" value="${v}"><span>${v}</span></label>`).join("")}</div>`
      : "";
    const blindNote = (check.visibility === "blind")
      ? `<p class="form-blind-note">🔒 Respuesta blind: el expected está oculto hasta validar</p>`
      : "";
    const expected = (check.expected && check.visibility !== "blind")
      ? `<p class="step-expected"><strong>Esperado</strong>${escapeHtml(check.expected)}</p>`
      : "";
    const required = check.required ? `<span class="form-required" title="Obligatorio">*</span>` : "";
    return `${opts}${rating}${text}${num}${yesNo}${blindNote}${expected}${required}`;
  }

  function formItemBlock(item, i) {
    const kind = (item.kind || "info").toLowerCase();
    if (kind === "info") {
      return `<li class="scenario-step step-info form-item form-info" style="--i:${i}"><p class="step-prose form-info-text">${escapeHtml(item.text || "")}</p></li>`;
    }
    if (kind === "flow") {
      const flow = (item.flow || "next").toLowerCase();
      return `<li class="scenario-step step-flow form-item" style="--i:${i}"><span class="badge badge-flag">flow: ${escapeHtml(flow)}</span>${item.target ? `<code class="form-flow-target">→ ${escapeHtml(item.target)}</code>` : ""}</li>`;
    }
    const check = item.check || {};
    const prompt = escapeHtml(check.prompt || "");
    const oracle = check.oracle ? `<span class="badge badge-flag">oracle: ${escapeHtml(check.oracle)}</span>` : "";
    return `<li class="scenario-step step-check form-item" style="--i:${i}">
      <div class="step-head"><span class="step-num">${i + 1}</span><span class="step-kind step-kind-check">check · ${escapeHtml(check.kind || "confirm")}</span>${oracle}</div>
      <p class="step-prose form-prompt">${prompt}</p>
      <div class="form-controls">${checkControl(check)}</div>
    </li>`;
  }

  function formList(form, startIndex) {
    if (!form || !form.items || !form.items.length) return "";
    return form.items.map((item, i) => formItemBlock(item, startIndex + i)).join("");
  }

  function evidenceChips(evidence) {
    if (!evidence || !evidence.length) return "";
    return evidence.map((e, i) => {
      const kind = (e.kind || "note").toLowerCase();
      const ref = e.ref ? escapeHtml(e.ref.slice(0, 24)) + (e.ref.length > 24 ? "…" : "") : "—";
      const size = e.size_bytes ? ` · ${e.size_bytes}B` : "";
      return `<li class="evidence-chip evidence-chip-${kind}" data-evidence-index="${i}"><button class="evidence-remove" data-remove="${i}" title="Eliminar">×</button><span class="evidence-kind">${escapeHtml(kind)}</span><code class="evidence-ref">${ref}</code>${size ? `<span class="evidence-size">${size}</span>` : ""}${e.observed_value != null ? `<span class="evidence-obs">obs=<code>${escapeHtml(e.observed_value)}</code></span>` : ""}${e.note ? `<span class="evidence-note">${escapeHtml(e.note)}</span>` : ""}</li>`;
    }).join("");
  }

  function evidenceCaptureUI(scenario, current) {
    const kinds = (scenario.evidence && scenario.evidence.kinds) || [];
    const fallback = scenario.evidence_prompt
      ? [{ kind: "note", note: scenario.evidence_prompt }]
      : [{ kind: "screenshot" }, { kind: "note" }];
    const list = kinds.length ? kinds : fallback;
    const prompt = scenario.evidence_prompt
      ? escapeHtml(scenario.evidence_prompt)
      : "Captura evidencia para soportar tu verdict";
    const inputs = list.map((k, i) => {
      const kind = (k.kind || "note").toLowerCase();
      if (kind === "screenshot") {
        return `<div class="evidence-input evidence-input-screenshot" style="--i:${i}"><label class="evidence-input-label">📷 Screenshot</label><div class="evidence-drop" id="drop">Pega screenshot aquí (Ctrl+V)</div><button class="btn btn-tiny" data-attach-file="screenshot">Adjuntar archivo</button></div>`;
      }
      if (kind === "file") {
        return `<div class="evidence-input evidence-input-file" style="--i:${i}"><label class="evidence-input-label">📄 Fichero${k.ref ? ` <span class="evidence-input-ref">ref esperado: <code>${escapeHtml(k.ref)}</code></span>` : ""}</label><button class="btn btn-tiny" data-attach-file="file">Adjuntar archivo</button></div>`;
      }
      if (kind === "command_output") {
        return `<div class="evidence-input evidence-input-command" style="--i:${i}"><label class="evidence-input-label">⌨️ Command output${k.ref ? ` <span class="evidence-input-ref">ref: <code>${escapeHtml(k.ref)}</code></span>` : ""}</label><textarea class="evidence-text" rows="3" data-evidence-kind="command_output" placeholder="Pega aquí la salida del comando (stdout+stderr)"></textarea><button class="btn btn-tiny" data-attach-text="command_output">Capturar</button></div>`;
      }
      if (kind === "assertion") {
        return `<div class="evidence-input evidence-input-assertion" style="--i:${i}"><label class="evidence-input-label">✅ Assertion${k.expected_value != null ? ` <span class="evidence-input-ref">expected: <code>${escapeHtml(k.expected_value)}</code> (${escapeHtml(k.match_mode || "exact_match")})</span>` : ""}</label><input class="evidence-text evidence-text-assertion" type="text" data-evidence-kind="assertion" placeholder="observed value"><button class="btn btn-tiny" data-attach-text="assertion">Comparar</button></div>`;
      }
      if (kind === "metric") {
        return `<div class="evidence-input evidence-input-metric" style="--i:${i}"><label class="evidence-input-label">📊 Metric${k.expected_value != null ? ` <span class="evidence-input-ref">expected: <code>${escapeHtml(k.expected_value)}</code></span>` : ""}</label><input class="evidence-text evidence-text-metric" type="text" data-evidence-kind="metric" placeholder="valor"><button class="btn btn-tiny" data-attach-text="metric">Capturar</button></div>`;
      }
      if (kind === "video") {
        return `<div class="evidence-input evidence-input-video" style="--i:${i}"><label class="evidence-input-label">🎥 Video</label><div class="video-controls"><button class="btn btn-tiny video-start" data-evidence-kind="video">Grabar</button><button class="btn btn-tiny video-stop" data-evidence-kind="video" disabled>Parar</button><span class="video-timer"></span></div><video class="video-preview" controls style="display:none"></video></div>`;
      }
      if (kind === "annotation") {
        return `<div class="evidence-input evidence-input-annotation" style="--i:${i}"><label class="evidence-input-label">✏️ Anotación</label><div class="annotation-base" id="annotation-base"></div><div class="annotation-tools"><button class="btn btn-tiny annotation-tool-active" data-tool="arrow">→</button><button class="btn btn-tiny" data-tool="rect">▢</button><button class="btn btn-tiny" data-tool="text">T</button><button class="btn btn-tiny" data-tool="clear">✕</button></div><canvas class="annotation-canvas" width="800" height="500" style="display:none"></canvas><button class="btn btn-tiny" data-attach-annotation="annotation">Adjuntar anotación</button></div>`;
      }
      return `<div class="evidence-input evidence-input-note" style="--i:${i}"><label class="evidence-input-label">📝 Nota</label><textarea class="evidence-text" rows="2" data-evidence-kind="note" placeholder="${escapeHtml(k.note || "Observación libre")}"></textarea><button class="btn btn-tiny" data-attach-text="note">Capturar</button></div>`;
    }).join("");
    return `<div class="evidence-section" style="--i:6"><p class="evidence-prompt"><strong>Evidencia</strong>${prompt}</p><div class="evidence-inputs">${inputs}</div><ul class="evidence-list">${evidenceChips(current.evidence)}</ul></div>`;
  }

  function failureProtocolPanel(scenario, observed) {
    const protocol = scenario.context && scenario.context.failure_protocol;
    if (!protocol) return "";
    const onFail = protocol.on_fail || [];
    const checklist = onFail.map((s, i) =>
      `<li class="failure-item" style="--i:${i}"><label class="failure-check"><input type="checkbox" data-failure-check="${escapeHtml(s)}"><span>${escapeHtml(s)}</span></label></li>`
    ).join("");
    return `<section class="failure-panel" id="failure-panel" style="--i:10"><header class="failure-head"><h3 class="failure-title">Failure protocol</h3><span class="failure-hint">Sigue el checklist antes de reportar</span></header><ul class="failure-list">${checklist}</ul><div class="failure-actions"><textarea class="evidence-text failure-observed" rows="2" placeholder="Observado (pegar aquí)">${escapeHtml(observed || "")}</textarea><button class="btn btn-failure" id="copy-defect-report">📋 Copiar defect report</button><input class="evidence-text failure-defect-id" type="text" placeholder="DEF-123 (issue tracker id)" id="defect-id"></div></section>`;
  }

  function teardownChecklist(scenario) {
    const items = (scenario.context && scenario.context.postconditions) || [];
    if (!items.length) return "";
    const rows = items.map((p, i) =>
      `<li class="teardown-item" style="--i:${i}"><label class="teardown-check"><input type="checkbox" class="teardown-cb" data-teardown="${escapeHtml(p)}"><span>${escapeHtml(p)}</span></label></li>`
    ).join("");
    return `<section class="teardown" style="--i:0"><header class="teardown-head"><h3 class="teardown-title">Teardown</h3><span class="teardown-hint">Cleanup tras el scenario</span></header><ul class="teardown-list">${rows}</ul></section>`;
  }

  // ─── F13: Blind Match + Normalization (REQ-RF-026) ─────────────────────────
  // spec: trim → NBSP → space → lowercase → collapse whitespace
  function normalizeForMatch(s) {
    if (s == null) return "";
    return String(s)
      .replace(/\u00A0/g, " ")   // NBSP → space
      .replace(/\u200B/g, "")     // zero-width space
      .replace(/[ \t]+/g, " ")   // collapse interior whitespace
      .trim()
      .toLowerCase();
  }

  function blindMatch(observed, expected) {
    return normalizeForMatch(observed || "") === normalizeForMatch(expected || "");
  }

  // ─── F13: Branching Runtime (REQ-RF-025) ───────────────────────────────────
  // Returns nextItemId or null (stop / continue-manual / target-not-found)
  function interpretBranching(items, currentItemId, status) {
    if (!items || !currentItemId || !status) return null;
    const current = items.find((i) => i.id === currentItemId);
    if (!current || !current.flow) return null;
    const key = { PASS: "on_pass", FAIL: "on_fail", BLOCKED: "on_blocked" }[status];
    if (!key) return null;
    const target = current.flow[key];
    if (!target || target === "stop" || target === "continue") return null;
    // If target doesn't exist in items, runtime should stop
    if (!items.find((i) => i.id === target)) return null;
    return target;
  }

  // ─── F13: Evidence Gate (REQ-RF-026) ───────────────────────────────────────
  // Returns true if all evidence_requirement[] kinds are satisfied
  function evidenceGateEnforce(scenario, evidence) {
    const reqs = (scenario.check && scenario.check.evidence_requirement) || {};
    if (!reqs.required) return true;
    const accepted = reqs.accepted || ["screenshot"];
    const evidenceKinds = (evidence || []).map((e) => (e.kind || "note").toLowerCase());
    return accepted.every((k) => evidenceKinds.includes(k.toLowerCase()));
  }

  // ─── F13: Rating Gate (REQ-RF-026) ─────────────────────────────────────────
  // Rating 1-5; if rating < require_comment_below, comment is mandatory
  function ratingGate(rating, comment, requireCommentBelow) {
    if (!requireCommentBelow) return true;
    if (rating >= requireCommentBelow) return true;
    return (comment || "").trim().length > 0;
  }

  // ─── F13: Inbox View (REQ-RF-024) ──────────────────────────────────────────
  function inboxView(plan, session) {
    const features = plan.features || [];
    const scenarioStatus = {};
    let attention = 0, inProgress = 0, blocked = 0, total = 0;
    let totalEstMinutes = 0, totalPreflight = 0, totalHuman = 0, totalCoverage = 0;

    for (const feature of features) {
      for (const sc of feature.scenarios || []) {
        total++;
        const st = statusOf(session, sc.id);
        scenarioStatus[sc.id] = { status: st, scenario: sc, feature };
        if (st === "PENDING") attention++;
        else if (st === "IN_PROGRESS") inProgress++;
        else if (st === "BLOCKED") blocked++;

        // E14.1: metadata enrichment
        const estMinutes = sc.context?.estimated_duration
          || Math.max(1, ((sc.steps && sc.steps.length) || (sc.form && sc.form.items && sc.form.items.length) || 3) * 2);
        const preflightCount = (sc.context?.preconditions?.length || 0);
        const humanCount = (sc.form?.items || []).filter(item =>
          item.check && ["blind_observation", "human_confirmation", "human_rating", "human_observation"].includes(item.check.kind)
        ).length;
        const hasResult = st && st !== "PENDING";
        scenarioStatus[sc.id]._meta = { estMinutes, preflightCount, humanCount, hasResult };
        totalEstMinutes += estMinutes;
        totalPreflight += preflightCount;
        totalHuman += humanCount;
        if (hasResult) totalCoverage++;
      }
    }

    const rows = [];
    for (const feature of features) {
      for (const sc of feature.scenarios || []) {
        const { status: st, _meta } = scenarioStatus[sc.id];
        const priority = (sc.priority || "P2").toUpperCase();
        const priorityCls = priority === "P0" ? "p0" : priority === "P1" ? "p1" : "p2";
        const statusCls = st === "PASS" ? "passed" : st === "FAIL" ? "failed" : st === "BLOCKED" ? "blocked" : st === "IN_PROGRESS" ? "in-progress" : "pending";
        const statusLabel = st === "IN_PROGRESS" ? `${inProgress}/${total}` : st;
        const { estMinutes, preflightCount, humanCount } = _meta;
        rows.push(
          `<tr class="inbox-row inbox-row-${statusCls}" data-scenario="${escapeHtml(sc.id)}">` +
            `<td class="id">${escapeHtml(sc.id)}</td>` +
            `<td>${escapeHtml(sc.title)}</td>` +
            `<td><span class="badge badge-${priorityCls.toLowerCase()}">${priority}</span></td>` +
            `<td class="inbox-meta">` +
              `<span class="inbox-meta-item inbox-time">⏱${estMinutes}m</span>` +
              `<span class="inbox-meta-item inbox-preflight">✓${preflightCount}</span>` +
              `<span class="inbox-meta-item inbox-human">⚠${humanCount}</span>` +
            `</td>` +
            `<td class="inbox-status"><span class="status-pill ${statusCls}">${statusLabel}</span></td>` +
          `</tr>`
        );
      }
    }

    const coveragePct = total > 0 ? Math.round((totalCoverage / total) * 100) : 0;
    const summary = `<div class="inbox-summary">
      <span class="inbox-stat inbox-attention">⚠ ${attention}</span>
      <span class="inbox-stat inbox-progress">⏳ ${inProgress}/${total}</span>
      <span class="inbox-stat inbox-blocked">⏸ ${blocked}</span>
      <span class="inbox-stat inbox-coverage">☑ ${totalCoverage}/${total} (${coveragePct}%)</span>
      <span class="inbox-stat inbox-est">⏱ ${totalEstMinutes}m total</span>
    </div>`;

    return `<section class="inbox-view" style="--i:0">
      <header class="inbox-head"><h2 class="inbox-title">My Validations</h2></header>
      ${summary}
      <table class="data-table inbox-table"><thead><tr><th class="id">ID</th><th>Escenario</th><th>Prioridad</th><th class="inbox-meta-col">Meta</th><th>Estado</th></tr></thead><tbody>${rows.join("")}</tbody></table>
    </section>`;
  }

  // ─── F13: Checkpoint Block (REQ-RF-027) ────────────────────────────────────
  function checkpointBlock(checkpoint, summary) {
    const title = escapeHtml(checkpoint.title || "Checkpoint");
    const machineSummary = summary.machine || {};
    const machinePassed = machineSummary.passed || 0;
    const machineTotal = machineSummary.total || 0;
    const machinePct = machineTotal ? Math.round((100 * machinePassed) / machineTotal) : 0;
    const faraSummary = summary.fara || {};
    const faraConf = faraSummary.confidence ? ` (${Math.round(faraSummary.confidence * 100)}%)` : "";
    const anomalies = (summary.anomalies || []).length;
    const anomalyNote = anomalies > 0 ? `<span class="checkpoint-anomaly">⚠ ${anomalies} anomaly(ies)</span>` : "";

    return `<section class="checkpoint-block" style="--i:10">
      <header class="checkpoint-head">
        <h3 class="checkpoint-title">${title}</h3>
        <span class="checkpoint-badge">Checkpoint</span>
      </header>
      <div class="checkpoint-evidence-summary">
        <div class="checkpoint-machine">
          <span class="checkpoint-label">Machine</span>
          <span class="checkpoint-value">${machinePassed}/${machineTotal} (${machinePct}%)</span>
          <div class="checkpoint-bar"><div class="checkpoint-fill" style="width:${machinePct}%"></div></div>
        </div>
        ${faraSummary.assessment ? `<div class="checkpoint-fara"><span class="checkpoint-label">AI Assessment</span><span class="checkpoint-value checkpoint-ai">◉ ${escapeHtml(faraSummary.assessment)}${faraConf}</span></div>` : ""}
        ${anomalyNote}
      </div>
      <div class="checkpoint-actions">
        <button class="btn btn-checkpoint-approve" data-action="approve">✅ Approve</button>
        <button class="btn btn-checkpoint-reject" data-action="reject">❌ Reject</button>
      </div>
    </section>`;
  }

  // ─── F13: AI Diagnostics Panel (REQ-RF-027) ────────────────────────────────
  // 6 evidence kinds: screenshot, trace, console, network, dom, trajectory
  function diagnosticsPanel(failure, driverData) {
    const collected = driverData || {};
    const kinds = [];
    if (collected.screenshot) kinds.push("screenshot");
    if (collected.trace || collected.trace_zip) kinds.push("trace");
    if (collected.console_messages > 0) kinds.push("console");
    if (collected.network_failures > 0) kinds.push("network");
    if (collected.dom) kinds.push("DOM");
    if (collected.trajectory) kinds.push("trajectory");

    const icons = kinds.map((k) => {
      const icon = k === "screenshot" ? "📷" : k === "trace" ? "🔍" : k === "console" ? "⌨️" : k === "network" ? "🌐" : k === "DOM" ? "📄" : "🛤️";
      return `<span class="diag-kind">${icon} ${k}</span>`;
    }).join("");

    const cause = escapeHtml(failure.cause || "Unknown — inspect evidence above");
    const category = escapeHtml(failure.category || "Uncategorized");
    const defectTitle = escapeHtml(failure.suggested_defect || "Untitled defect");
    const observed = escapeHtml(failure.observed || "");
    const expected = escapeHtml(failure.expected || "");

    return `<section class="diagnostics-panel" style="--i:9">
      <header class="diagnostics-head"><h3 class="diagnostics-title">Failure detected — evidence already collected</h3></header>
      <div class="diagnostics-evidence">${icons}</div>
      <div class="diagnostics-cause">
        <span class="diag-label">Possible cause:</span>
        <p class="diag-value">${cause}</p>
      </div>
      <div class="diagnostics-category">
        <span class="diag-label">Category:</span>
        <span class="badge badge-flag">${category}</span>
      </div>
      <div class="diagnostics-defect">
        <span class="diag-label">Suggested defect:</span>
        <p class="diag-defect-title">${defectTitle}</p>
      </div>
      ${observed || expected ? `<div class="diagnostics-compare"><span class="diag-label">Observed:</span><code>${observed}</code><span class="diag-label">Expected:</span><code>${expected}</code></div>` : ""}
      <div class="diagnostics-actual-result">
        <label class="diag-label">Actual Result (machine-observable):</label>
        <div class="actual-result-confirm">
          <span class="actual-correct">[✓ Correct]</span>
          <button class="btn btn-tiny btn-edit-actual">Edit</button>
        </div>
      </div>
    </section>`;
  }

  // ─── F13: Sign-Off Wizard (REQ-RF-028) ─────────────────────────────────────
  function signOffWizard(acceptance, counts) {
    const decision = acceptance.decision || "pending";
    const actor = escapeHtml(acceptance.actor || "");
    const justification = escapeHtml(acceptance.justification || "");
    const timestamp = acceptance.timestamp || "";
    const decisionIcon = { accepted: "✅", accepted_conditional: "⚠", rejected: "❌", pending: "⏳" }[decision] || "⏳";
    const decisionLabel = { accepted: "Accepted", accepted_conditional: "Accepted conditionally", rejected: "Rejected", pending: "Pending" }[decision] || "Pending";

    return `<section class="signoff-wizard" style="--i:0">
      <header class="signoff-head"><h2 class="signoff-title">Sign-off Decision</h2></header>
      <div class="signoff-decision">
        <span class="signoff-icon">${decisionIcon}</span>
        <span class="signoff-label">${decisionLabel}</span>
      </div>
      <div class="signoff-meta">
        <span class="signoff-actor">Actor: ${actor}</span>
        ${timestamp ? `<span class="signoff-time">${timestamp}</span>` : ""}
      </div>
      ${justification ? `<div class="signoff-justification"><p>${justification}</p></div>` : ""}
      <div class="signoff-counts">
        <span class="count-badge count-pass">✓ ${counts.passed || 0}</span>
        <span class="count-badge count-conditional">⚠ ${counts.conditional || 0}</span>
        <span class="count-badge count-rejected">✕ ${counts.rejected || 0}</span>
      </div>
    </section>`;
  }

  // ─── F13: Release Acceptance View (REQ-RF-028) ───────────────────────────────
  function releaseAcceptanceView(report) {
    const counts = report.counts || {};
    const criticalReqs = report.critical_requirements || [];
    const openDefects = report.open_defects || [];
    const aiAssessment = report.ai_assessment || {};
    const aiConf = aiAssessment.confidence ? ` (${Math.round(aiAssessment.confidence * 100)}%)` : "";

    const criticalBadges = criticalReqs.map((r) => {
      const cls = r.status === "pass" ? "pass" : r.status === "fail" ? "fail" : "conditional";
      return `<span class="badge badge-${cls}">${escapeHtml(r.req || r)}</span>`;
    }).join("");

    const defectRows = openDefects.map((d) =>
      `<tr><td class="id">${escapeHtml(d.id || "")}</td><td>${escapeHtml(d.title || "")}</td><td>${escapeHtml(d.priority || "P2")}</td></tr>`
    ).join("");

    return `<section class="release-acceptance-view" style="--i:0">
      <header class="ra-head"><h2 class="ra-title">RELEASE ACCEPTANCE — ${escapeHtml(report.release || "N/A")}</h2></header>
      <div class="ra-counts">
        <div class="ra-count-item ra-pass"><span class="ra-count-num">${counts.machine_passed || 0}</span><span class="ra-count-label">machine ✓</span></div>
        <div class="ra-count-item ra-human"><span class="ra-count-num">${counts.human_passed || 0}</span><span class="ra-count-label">human ✓</span></div>
        <div class="ra-count-item ra-conditional"><span class="ra-count-num">${counts.conditional || 0}</span><span class="ra-count-label">conditional ⚠</span></div>
        <div class="ra-count-item ra-rejected"><span class="ra-count-num">${counts.rejected || 0}</span><span class="ra-count-label">rejected ✕</span></div>
      </div>
      <div class="ra-critical"><h3>Critical requirements</h3><div class="ra-badges">${criticalBadges || "<span class='text-dim'>none</span>"}</div></div>
      <div class="ra-defects">
        <h3>Open defects</h3>
        <p class="ra-defect-summary">P0 ${openDefects.filter(d => d.priority === "P0").length} | P1 ${openDefects.filter(d => d.priority === "P1").length} | P2 ${openDefects.filter(d => d.priority === "P2").length}</p>
        ${defectRows ? `<table class="data-table"><thead><tr><th class="id">ID</th><th>Title</th><th>Priority</th></tr></thead><tbody>${defectRows}</tbody></table>` : ""}
      </div>
      ${aiAssessment.decision ? `<div class="ra-ai"><h3>AI Assessment</h3><p>◉ ${escapeHtml(aiAssessment.decision)}${aiConf}</p></div>` : ""}
    </section>`;
  }

  // ─── F13: Staleness Banner (REQ-RF-024) ────────────────────────────────────
  function stalenessBanner(report) {
    const affected = report.affected_scenarios || [];
    if (!affected.length) return "";
    const count = affected.length;
    return `<div class="staleness-banner" style="--i:0">
      <span class="staleness-icon">⚠</span>
      <span class="staleness-text">${count} scenario${count > 1 ? "s" : ""} may be stale</span>
      <button class="btn btn-small" onclick="location.reload()">Review proposed updates</button>
    </div>`;
  }

  return {
    pill, matrix, traceability, kpis,
    userStoryBanner, preflightChecklist, contextBar, stepBlock,
    formList, formItemBlock,
    evidenceChips, evidenceCaptureUI, failureProtocolPanel, teardownChecklist,
    escapeHtml,
    // F13 exports
    normalizeForMatch, blindMatch, interpretBranching,
    evidenceGateEnforce, ratingGate,
    inboxView, checkpointBlock, diagnosticsPanel,
    signOffWizard, releaseAcceptanceView, stalenessBanner,
  };
})();