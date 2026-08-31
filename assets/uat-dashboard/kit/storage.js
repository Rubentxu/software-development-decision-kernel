/* Storage helpers: localStorage persistence + export/import JSON (zero backend).
 *
 * v2 export shape (consumed by `sddk uat ingest`):
 *   schema_version: 2, session_id, plan_ref, plan_version: 2, release,
 *   executor: human, executed_by, started_at, finished_at, metadata: {
 *     tester, started_at, completed_at, duration_ms, env_fingerprint: {
 *       os, shell, binary, locale, workdir }, build: { commit, branch, tag,
 *       dirty } }, results: [{ scenario_id, status, verdict_at,
 *     verdict_duration_ms, duration_minutes, comment, tester_notes, observed,
 *     failure_reason, linked_defect, repro_command, evidence: [{ kind (typed),
 *     ref, note, captured_at, size_bytes, mime, path, observed_value,
 *     expected_value, match_mode }] }].
 */

const UAT = (() => {
  const KEY = (release) => `sddk-${release}`;

  function nowRfc3339() {
    return new Date().toISOString();
  }

  function uuid() {
    if (typeof crypto !== "undefined" && crypto.randomUUID) return crypto.randomUUID();
    return "uat-" + Math.random().toString(36).slice(2) + "-" + Date.now().toString(36);
  }

  function ensureTesterId() {
    const KEY_ID = "sddk-tester-id";
    let id = localStorage.getItem(KEY_ID);
    if (!id) {
      id = "T-" + Math.random().toString(36).slice(2, 6).toUpperCase();
      try { localStorage.setItem(KEY_ID, id); } catch (e) {}
    }
    return id;
  }

  function loadSession(release) {
    try {
      const raw = localStorage.getItem(KEY(release));
      return raw ? JSON.parse(raw) : null;
    } catch (e) { return null; }
  }

  function saveSession(release, session) {
    try {
      localStorage.setItem(KEY(release), JSON.stringify(session));
      return true;
    } catch (e) { return false; }
  }

  function buildEnvFingerprint() {
    const ua = navigator.userAgent || "";
    const os = (ua.match(/\(([^)]+)\)/) || [, ""])[1].trim() || "unknown";
    return {
      os,
      shell: "browser",
      binary: `sddk-dashboard (browser; ${navigator.platform || "unknown"})`,
      locale: navigator.language || "unknown",
      workdir: location.pathname,
    };
  }

  function fromLegacy(legacy, plan) {
    if (!legacy) return null;
    if (Array.isArray(legacy.results)) return legacy;
    if (legacy.scenario_results && plan) {
      const order = [];
      for (const f of plan.features || []) {
        for (const s of f.scenarios || []) order.push(s.id);
      }
      const results = [];
      for (const id of order) {
        const r = legacy.scenario_results[id];
        if (!r || !r.status) continue;
        results.push({
          scenario_id: id,
          status: r.status,
          comment: r.comment || "",
          evidence: r.evidence || [],
          duration_minutes: r.duration_minutes || 0,
        });
      }
      return {
        schema_version: 1,
        session_id: legacy.session_id || uuid(),
        plan_ref: legacy.plan_ref || plan.release?.candidate || "",
        release: legacy.release || "",
        executor: legacy.executor || "human",
        executed_by: legacy.executed_by || "",
        started_at: legacy.started_at || nowRfc3339(),
        finished_at: legacy.finished_at || null,
        results,
      };
    }
    return null;
  }

  function buildUatSession({
    release, planRef, executedBy, startedAt, verdicts, scenarioOrder, finishedAt,
    planVersion = 2, buildMeta = null,
  }) {
    const results = [];
    for (const id of scenarioOrder) {
      const v = verdicts[id];
      if (!v || !v.status) {
        results.push({
          scenario_id: id,
          status: "NOT_RUN",
          comment: "Scenario not evaluated",
          evidence: [],
          duration_minutes: 0,
        });
        continue;
      }
      const result = {
        scenario_id: id,
        status: v.status,
        comment: v.comment || "",
        evidence: v.evidence || [],
        duration_minutes: v.duration_minutes || 0,
      };
      if (v.verdict_at) result.verdict_at = v.verdict_at;
      if (v.verdict_duration_ms) result.verdict_duration_ms = v.verdict_duration_ms;
      if (v.tester_notes) result.tester_notes = v.tester_notes;
      if (v.observed) result.observed = v.observed;
      if (v.failure_reason) result.failure_reason = v.failure_reason;
      if (v.linked_defect) result.linked_defect = v.linked_defect;
      if (v.repro_command) result.repro_command = v.repro_command;
      results.push(result);
    }
    const finished = finishedAt || nowRfc3339();
    const startedMs = startedAt ? Date.parse(startedAt) : Date.now();
    const finishedMs = Date.parse(finished);
    const duration_ms = Number.isFinite(startedMs) && Number.isFinite(finishedMs)
      ? Math.max(0, finishedMs - startedMs)
      : null;
    const testerId = ensureTesterId();
    const metadata = {
      tester: { id: testerId, display: executedBy || null },
      started_at: startedAt || nowRfc3339(),
      completed_at: finished,
      duration_ms,
      env_fingerprint: buildEnvFingerprint(),
      build: buildMeta || null,
    };
    return {
      schema_version: 2,
      plan_version: planVersion,
      session_id: uuid(),
      plan_ref: planRef || release,
      release,
      executor: "human",
      executed_by: executedBy || testerId,
      started_at: startedAt || nowRfc3339(),
      finished_at: finished,
      metadata,
      results,
    };
  }

  function downloadBlob(filename, json) {
    const blob = new Blob([JSON.stringify(json, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  function finalizeAndExport({ release, planRef, executedBy, startedAt, verdicts, scenarioOrder, planVersion, buildMeta }) {
    const session = buildUatSession({
      release, planRef, executedBy, startedAt, verdicts, scenarioOrder,
      finishedAt: nowRfc3339(), planVersion, buildMeta,
    });
    saveSession(release, session);
    downloadBlob(`uat-session-${release}.json`, session);
    return session;
  }

  function exportSession(release) {
    const session = loadSession(release);
    if (!session) return null;
    downloadBlob(`uat-session-${release}.json`, session);
    return session;
  }

  function importSession(release) {
    return new Promise((resolve, reject) => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = ".json,.yaml,.yml";
      input.onchange = () => {
        const file = input.files && input.files[0];
        if (!file) { resolve(null); return; }
        file.text().then((text) => {
          try {
            const parsed = JSON.parse(text);
            saveSession(release, parsed);
            resolve(parsed);
          } catch (e) { reject(new Error("archivo no es JSON válido")); }
        });
      };
      input.click();
    });
  }

  async function sha256OfBlob(blob) {
    const buf = await blob.arrayBuffer();
    const digest = await crypto.subtle.digest("SHA-256", buf);
    const bytes = new Uint8Array(digest);
    let hex = "sha256:";
    for (let i = 0; i < bytes.length; i++) hex += bytes[i].toString(16).padStart(2, "0");
    return { hex, bytes: buf.byteLength };
  }

  async function sha256OfString(s) {
    const buf = new TextEncoder().encode(s);
    const digest = await crypto.subtle.digest("SHA-256", buf);
    const bytes = new Uint8Array(digest);
    let hex = "sha256:";
    for (let i = 0; i < bytes.length; i++) hex += bytes[i].toString(16).padStart(2, "0");
    return { hex, bytes: buf.byteLength };
  }

  async function addTypedEvidence(release, scenarioId, evidence) {
    const stamp = { captured_at: nowRfc3339() };
    let ref = evidence.ref;
    let size_bytes = evidence.size_bytes;
    let mime = evidence.mime;
    if ((evidence.kind === "screenshot" || evidence.kind === "file" || evidence.kind === "command_output")
        && evidence.blob && !ref) {
      const h = await sha256OfBlob(evidence.blob);
      ref = h.hex;
      size_bytes = size_bytes || h.bytes;
    } else if (evidence.kind === "note" && evidence.text && !ref) {
      const h = await sha256OfString(evidence.text);
      ref = h.hex;
      size_bytes = size_bytes || h.bytes;
    } else if (evidence.kind === "command_output" && evidence.text && !ref) {
      const h = await sha256OfString(evidence.text);
      ref = h.hex;
      size_bytes = size_bytes || h.bytes;
    } else if (evidence.kind === "video" && evidence.blob && !ref) {
      const h = await sha256OfBlob(evidence.blob);
      ref = h.hex;
      size_bytes = size_bytes || h.bytes;
    } else if (evidence.kind === "annotation" && evidence.blob && !ref) {
      const h = await sha256OfBlob(evidence.blob);
      ref = h.hex;
      size_bytes = size_bytes || h.bytes;
    }
    const entry = { kind: evidence.kind, ref: ref || "", note: evidence.note, ...stamp };
    if (size_bytes != null) entry.size_bytes = size_bytes;
    if (mime != null) entry.mime = mime;
    if (evidence.path != null) entry.path = evidence.path;
    if (evidence.observed_value != null) entry.observed_value = evidence.observed_value;
    if (evidence.expected_value != null) entry.expected_value = evidence.expected_value;
    if (evidence.match_mode != null) entry.match_mode = evidence.match_mode;
    if (evidence.duration_ms != null) entry.duration_ms = evidence.duration_ms;
    if (evidence.based_on != null) entry.based_on = evidence.based_on;
    const session = loadSession(release) || { schema_version: 2, release, results: [] };
    let r = (session.results || []).find(r => r.scenario_id === scenarioId);
    if (!r) {
      r = { scenario_id: scenarioId, status: "NOT_RUN", evidence: [] };
      (session.results || (session.results = [])).push(r);
    }
    if (!r.evidence) r.evidence = [];
    r.evidence.push(entry);
    saveSession(release, session);
    return entry;
  }

  function addEvidence(release, scenarioId, evidence) {
    const session = loadSession(release) || { release, results: [] };
    let entry = (session.results || []).find(r => r.scenario_id === scenarioId);
    if (!entry) {
      entry = { scenario_id: scenarioId, status: "NOT_RUN", evidence: [] };
      (session.results || (session.results = [])).push(entry);
    }
    if (!entry.evidence) entry.evidence = [];
    entry.evidence.push(evidence);
    saveSession(release, session);
  }

  // Internal map: `${release}:${scenarioId}` → data URL of most recent screenshot
  // (used by annotation canvas to retrieve base image; not part of the session schema)
  const _screenshotCache = {};

  function _screenshotCacheKey(release, scenarioId) {
    return `${release}::${scenarioId}`;
  }

  async function pasteScreenshot(release, scenarioId, callback) {
    return new Promise((resolve) => {
      const handler = async (ev) => {
        document.removeEventListener("paste", handler);
        const items = ev.clipboardData && ev.clipboardData.items;
        if (!items) { resolve(null); return; }
        for (const item of items) {
          if (item.type && item.type.startsWith("image/")) {
            const blob = item.getAsFile();
            const reader = new FileReader();
            reader.onload = async () => {
              const dataUrl = reader.result;
              const entry = await addTypedEvidence(release, scenarioId, {
                kind: "screenshot", blob, mime: blob.type,
                note: "pegado desde portapapeles",
              });
              // Cache data URL for annotation canvas
              _screenshotCache[_screenshotCacheKey(release, scenarioId)] = dataUrl;
              if (callback) callback(dataUrl, entry);
              resolve(entry);
            };
            reader.readAsDataURL(blob);
            return;
          }
        }
        resolve(null);
      };
      document.addEventListener("paste", handler);
    });
  }

  function buildDefectReport({ scenario, plan, session, observed }) {
    const feature = (plan.features || []).find(f =>
      (f.scenarios || []).some(s => s.id === scenario.id));
    const protocol = scenario.context && scenario.context.failure_protocol;
    const tpl = protocol && protocol.expected_defect_template;
    if (!tpl) return null;
    const testerId = session.metadata && session.metadata.tester
      ? session.metadata.tester.id : (session.executed_by || "unknown");
    const commit = session.metadata && session.metadata.build && session.metadata.build.commit
      ? session.metadata.build.commit : "unknown";
    const repro = (scenario.plain_steps || []).filter(s => s.copy_hint).map(s => s.action).join("\n") || "n/a";
    return tpl
      .replace(/<scenario_id>/g, scenario.id)
      .replace(/<commit>/g, commit)
      .replace(/<repro_command>/g, repro)
      .replace(/<expected>/g, (scenario.plain_steps || []).map(s => s.expected).join("\n") || "n/a")
      .replace(/<observed>/g, observed || "(pegar aquí)")
      .replace(/<os>/g, (session.metadata && session.metadata.env_fingerprint && session.metadata.env_fingerprint.os) || "unknown")
      .replace(/<binary>/g, (session.metadata && session.metadata.env_fingerprint && session.metadata.env_fingerprint.binary) || "unknown")
      + `\n\n— Testado por: ${testerId}\n— Feature: ${feature ? feature.id + " / " + feature.name : "(unknown)"}`;
  }

  // ─── Video evidence helpers ───────────────────────────────────────────────────
  // Returns the most recent screenshot sha256 ref from a scenario's evidence list.
  function getLastScreenshotRef(release, scenarioId) {
    const session = loadSession(release);
    if (!session) return null;
    const r = (session.results || []).find(r => r.scenario_id === scenarioId);
    if (!r || !r.evidence) return null;
    // Find last screenshot evidence entry
    const screenshots = r.evidence.filter(e => e.kind === "screenshot");
    return screenshots.length > 0 ? screenshots[screenshots.length - 1].ref : null;
  }

  // ─── Annotation helpers ────────────────────────────────────────────────────────
  // Caches a screenshot data URL for use by the annotation canvas.
  function cacheScreenshotDataUrl(release, scenarioId, dataUrl) {
    _screenshotCache[_screenshotCacheKey(release, scenarioId)] = dataUrl;
  }

  // Returns the cached screenshot data URL for the given scenario.
  // Returns null if no screenshot has been captured/attached this session.
  function getScreenshotDataUrl(release, scenarioId) {
    return _screenshotCache[_screenshotCacheKey(release, scenarioId)] || null;
  }

  return {
    loadSession, saveSession, exportSession, importSession,
    addEvidence, addTypedEvidence, pasteScreenshot,
    buildUatSession, buildDefectReport, fromLegacy, finalizeAndExport,
    nowRfc3339, uuid, ensureTesterId, sha256OfBlob, sha256OfString,
    getLastScreenshotRef, cacheScreenshotDataUrl, getScreenshotDataUrl,
  };
})();
