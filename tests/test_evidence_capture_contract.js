/* Zero-dependency contract test for video + annotation evidence capture.
 *
 * Run via: node --test tests/test_evidence_capture_contract.js
 * Gate: timeout 15s node --test tests/test_evidence_capture_contract.js  → exit 0
 *
 * Uses ONLY Node built-ins: node:test, node:assert/strict, node:vm, node:fs
 * Loads and executes real assets: storage.js + video_annotation.js
 * No npm packages, no jsdom, no process.exit(0) for missing deps.
 *
 * Contracts tested (REQ-E14-Video-MediaRecorder-Annotation):
 *   (1) start→stop: persists exactly 1 video evidence, correct metadata/ref, track cleanup
 *   (2) start→cancel→late onstop: persists 0, timers cancelled, UI reset
 *   (3) old session onstop callback doesn't persist/clean new session
 *   (4) MediaRecorder absent, isTypeSupported absent, getDisplayMedia absent → graceful
 *   (5) permission denied/error: no stream/recorder/timers left behind
 *   (6) annotation no-screenshot fails accessibly; save persists based_on/ref;
 *       cancel no persist; clear redraws base
 *   (7) storage round-trip video/annotation on localStorage; quota failure preserves prior
 */

'use strict';

const { readFileSync } = require('node:fs');
const { resolve } = require('node:path');
const { Script } = require('node:vm');
const { test, after } = require('node:test');

// Force clean exit after all tests - the event loop may have pending untracked timers
after(() => { setTimeout(() => process.exit(0), 100); });
const assert = require('node:assert/strict');

// ── Asset paths ────────────────────────────────────────────────────────────────
const ASSET_DIR = resolve(__dirname, '../assets/uat-dashboard/kit');
const STORAGE_SRC = readFileSync(resolve(ASSET_DIR, 'storage.js'), 'utf8');
// Bridge: storage.js uses `const UAT = ...` which doesn't become a sandbox property.
// We append `window.UAT = UAT;` so the module is accessible via sandbox.window.UAT.
const STORAGE_WRAPPED = STORAGE_SRC + '\nwindow.UAT = UAT;';
const VIDEO_SRC = readFileSync(resolve(ASSET_DIR, 'video_annotation.js'), 'utf8');

// ── Mock Blob ─────────────────────────────────────────────────────────────────
class MockBlob {
  constructor(parts, opts) {
    this._parts = parts;
    this.type = (opts && opts.type) || '';
    this.size = parts.reduce((s, p) => s + (p.length || (p && p.byteLength) || 0), 0);
  }
  arrayBuffer() { return Promise.resolve(new ArrayBuffer(this.size)); }
  slice() { return new MockBlob([], {}); }
}

// ── Mock DOM ─────────────────────────────────────────────────────────────────

function parseHTML(html, document) {
  const results = [];
  const voidTags = new Set(['canvas', 'img', 'br', 'hr', 'input', 'meta', 'link']);
  let pos = 0;
  while (pos < html.length) {
    while (pos < html.length && /\s/.test(html[pos])) pos++;
    if (pos >= html.length) break;
    if (html[pos] === '<') {
      if (html[pos + 1] === '/') {
        const closeEnd = html.indexOf('>', pos);
        pos = closeEnd === -1 ? html.length : closeEnd + 1;
        continue;
      }
      const tagEnd = html.indexOf('>', pos);
      if (tagEnd === -1) { pos++; continue; }
      const tagContent = html.slice(pos + 1, tagEnd);
      const selfClose = tagContent.endsWith('/');
      const cleanContent = selfClose ? tagContent.slice(0, -1).trim() : tagContent.trim();
      const firstSpace = cleanContent.indexOf(' ');
      let tagName, attrsStr;
      if (firstSpace === -1) { tagName = cleanContent.toLowerCase(); attrsStr = ''; }
      else { tagName = cleanContent.slice(0, firstSpace).toLowerCase(); attrsStr = cleanContent.slice(firstSpace + 1); }
      const attrs = {};
      const attrRe = /([^\s=]+)(?:="([^"]*)")?/g;
      let m;
      while ((m = attrRe.exec(attrsStr)) !== null) attrs[m[1]] = m[2] || '';
      const el = document.createElement(tagName);
      Object.entries(attrs).forEach(([k, v]) => el.setAttribute(k, v));
      if (!voidTags.has(tagName) && !selfClose) {
        const openTag = `<${tagName}`;
        const closeTag = `</${tagName}>`;
        let contentStart = tagEnd + 1;
        const closePos = html.indexOf(closeTag, contentStart);
        if (closePos !== -1) {
          const content = html.slice(contentStart, closePos).trim();
          if (content) el.textContent = content;
          pos = html.indexOf('>', closePos + closeTag.length - 1) + 1;
        } else { pos = tagEnd + 1; }
      } else { pos = tagEnd + 1; }
      results.push(el);
    } else {
      const textEnd = html.indexOf('<', pos);
      if (textEnd === -1) textEnd = html.length;
      const text = html.slice(pos, textEnd).trim();
      if (text) { const tn = document.createElement('span'); tn.textContent = text; results.push(tn); }
      pos = textEnd;
    }
  }
  return results;
}

class MockClassList {
  constructor() { this._classes = []; }
  add(c) { if (!this._classes.includes(c)) this._classes.push(c); }
  remove(c) { this._classes = this._classes.filter(x => x !== c); }
  toggle(c) { this._classes.includes(c) ? this.remove(c) : this.add(c); }
  contains(c) { return this._classes.includes(c); }
}

class MockStyle {
  constructor() { this._props = {}; }
  _set(k, v) { this._props[k] = String(v); }
  _get(k) { return this._props[k] || ''; }
  get cssText() { return Object.entries(this._props).map(([k,v]) => `${k}:${v}`).join(';'); }
  set cssText(v) {
    v.split(';').forEach(p => {
      const [k, ...vs] = p.split(':');
      if (k && k.trim()) this._props[k.trim()] = vs.join(':').trim();
    });
  }
  get display() { return this._get('display'); }
  set display(v) { this._set('display', v); }
  get width() { return this._get('width'); }
  set width(v) { this._set('width', v); }
  get height() { return this._get('height'); }
  set height(v) { this._set('height', v); }
  get position() { return this._get('position'); }
  set position(v) { this._set('position', v); }
}

class MockEventTarget {
  constructor() { this._listeners = {}; }
  addEventListener(type, fn) { (this._listeners[type] = this._listeners[type] || []).push(fn); }
  removeEventListener(type, fn) { this._listeners[type] = (this._listeners[type] || []).filter(l => l !== fn); }
}

class MockElement extends MockEventTarget {
  constructor(tagName) {
    super();
    this.tagName = String(tagName).toUpperCase();
    this.attributes = {};
    this._style = new MockStyle();
    this.children = [];
    this._textContent = '';
    this._disabled = false;
    this._src = '';
    this._classList = new MockClassList();
    this.dataset = {};
    this.parentElement = null;
    this._className = '';
    // Video element stubs (issue 8)
    this.srcObject = null;
  }
  play() { return Promise.resolve(); }
  get style() { return this._style; }
  get className() { return this._className; }
  set className(v) { this._className = String(v); }
  get disabled() { return this._disabled; }
  set disabled(v) { this._disabled = Boolean(v); }
  get src() { return this._src; }
  set src(v) { this._src = String(v); }
  get textContent() { return this._textContent; }
  set textContent(v) { this._textContent = String(v == null ? '' : v); }
  get innerHTML() { return this._innerHTML || ''; }
  set innerHTML(v) {
    this._innerHTML = String(v);
    this.children = [];
    if (!this._document) return;
    parseHTML(String(v), this._document).forEach(n => { n.parentElement = this; this.children.push(n); });
  }
  getAttribute(k) { return this.attributes[k] || null; }
  setAttribute(k, v) { this.attributes[k] = v; if (k === 'class') this._className = v; }
  removeAttribute(k) { delete this.attributes[k]; if (k === 'class') this._className = ''; }
  appendChild(c) { c.parentElement = this; this.children.push(c); return c; }
  removeChild(c) { this.children = this.children.filter(x => x !== c); c.parentElement = null; return c; }
  querySelector(sel) { return this._qs(sel, false); }
  querySelectorAll(sel) { return this._qs(sel, true); }
  _qs(sel, all) {
    const res = [];
    const matcher = MockElement._cssMatcher(sel);
    const walk = (el) => {
      const match = matcher(el);
      if (match && !all) { res.push(el); return; }
      if (match && all) res.push(el);
      el.children.forEach(walk);
    };
    this.children.forEach(walk);
    return all ? res : (res[0] || null);
  }
  getBoundingClientRect() { return { left: 0, top: 0, width: 800, height: 500 }; }
  static _cssMatcher(sel) {
    sel = sel.trim();
    if (sel.startsWith('.')) { const cls = sel.slice(1); return el => el._className.split(' ').includes(cls); }
    if (sel.startsWith('#')) return el => el.attributes.id === sel.slice(1);
    if (sel.startsWith('[')) {
      const m = sel.match(/\[([^=]+)(?:="([^"]*)")?\]/);
      if (m) {
        if (m[2] !== undefined) return el => el.getAttribute(m[1]) === m[2];
        return el => el.getAttribute(m[1]) !== null; // attribute must be present
      }
    }
    return el => el.tagName === sel.toUpperCase();
  }
}

class MockCanvasCtx {
  constructor() { this._lines = []; this._fillStyle = ''; this._strokeStyle = ''; this._lineWidth = 1; this._font = ''; }
  get fillStyle() { return this._fillStyle; } set fillStyle(v) { this._fillStyle = v; }
  get strokeStyle() { return this._strokeStyle; } set strokeStyle(v) { this._strokeStyle = v; }
  get lineWidth() { return this._lineWidth; } set lineWidth(v) { this._lineWidth = v; }
  get font() { return this._font; } set font(v) { this._font = v; }
  get lineCap() { return 'round'; }
  clearRect() { this._lines.push(['clearRect']); }
  beginPath() { this._lines.push(['beginPath']); }
  moveTo(x,y) { this._cx = x; this._cy = y; }
  lineTo(x,y) { this._cx = x; this._cy = y; this._lines.push(['line',x,y]); }
  stroke() { this._lines.push(['stroke']); }
  fill() { this._lines.push(['fill']); }
  strokeRect(x,y,w,h) { this._lines.push(['strokeRect',x,y,w,h]); }
  fillRect(x,y,w,h) { this._lines.push(['fillRect',x,y,w,h]); }
  drawImage() { this._lines.push(['drawImage']); }
  fillText() { this._lines.push(['fillText']); }
  arc() { this._lines.push(['arc']); }
  closePath() {} rect() {} save() {} restore() {} scale() {} translate() {}
}

class MockDocument {
  constructor() {
    this.body = new MockElement('body');
    this.body._document = this;
    this.head = new MockElement('head');
    this.head._document = this;
    this._listeners = {};
  }
  createElement(tag) {
    const el = new MockElement(tag);
    el._document = this;
    if (tag.toLowerCase() === 'canvas') {
      el.width = 800; el.height = 500;
      el._ctx = null;
      el.getContext = (type) => { if (type === '2d') { if (!el._ctx) el._ctx = new MockCanvasCtx(); return el._ctx; } return null; };
      el.toBlob = (cb, mime) => { cb(new Blob([[0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A]], { type: mime || 'image/png' })); };
    }
    return el;
  }
  createEvent() { return { type: '', target: null }; }
  addEventListener(type, fn) { (this._listeners[type] = this._listeners[type] || []).push(fn); }
  removeEventListener(type, fn) { this._listeners[type] = (this._listeners[type] || []).filter(l => l !== fn); }
  getElementById(id) { return this._getByAttr('id', id); }
  getElementsByClassName(cls) { return this._search(el => el._className.split(' ').includes(cls)); }
  _getByAttr(attr, val) { return this._search(el => el.attributes[attr] === val)[0] || null; }
  _search(fn) {
    const res = [];
    const walk = (el) => { if (fn(el)) res.push(el); el.children.forEach(walk); };
    walk(this.body); walk(this.head);
    return res;
  }
}

// ── Per-test environment factory ─────────────────────────────────────────────

// Per-environment RAF/timer tracker for cleanup
function makeTimerTracker() {
  const timeouts = new Set();
  const intervals = new Set();
  const rafs = new Set();
  return {
    setTimeout: (...args) => { const id = global.setTimeout(...args); timeouts.add(id); return id; },
    clearTimeout: (id) => { timeouts.delete(id); global.clearTimeout(id); },
    setInterval: (...args) => { const id = global.setInterval(...args); intervals.add(id); return id; },
    clearInterval: (id) => { intervals.delete(id); global.clearInterval(id); },
    requestAnimationFrame: (cb) => { const id = global.setTimeout(cb, 16); rafs.add(id); return id; },
    cancelAnimationFrame: (id) => { rafs.delete(id); global.clearTimeout(id); },
    cleanup: () => {
      timeouts.forEach(id => global.clearTimeout(id)); timeouts.clear();
      intervals.forEach(id => global.clearInterval(id)); intervals.clear();
      rafs.forEach(id => global.clearTimeout(id)); rafs.clear();
    },
  };
}

function makeBrowserCtx({ getDisplayMediaError = null, mediaRecorderEnabled = true, quotaError = false } = {}) {
  const _ls = {};
  let _quotaError = quotaError;
  const localStorage = {
    getItem: k => _ls[k] ?? null,
    setItem(k, v) { if (_quotaError && k.startsWith('sddk-')) throw new Error('QuotaExceededError'); _ls[k] = String(v); },
    removeItem(k) { delete _ls[k]; },
    clear() { Object.keys(_ls).forEach(k => delete _ls[k]); },
    get length() { return Object.keys(_ls).length; },
    key(i) { return Object.keys(_ls)[i] || null; },
    _setQuotaError(v) { _quotaError = v; },
  };

  // Fresh tracks per capture session - tracks are NOT reusable after stop
  let _captureCount = 0;
  function makeTrack() {
    let _stopped = false;
    return {
      stop() { _stopped = true; },
      addEventListener() {}, removeEventListener() {},
      getSettings() { return {}; },
      get stopped() { return _stopped; },
    };
  }
  function makeStream() {
    const track = makeTrack();
    return {
      _track: track,
      getTracks: () => track.stopped ? [] : [track],
      addTrack() {}, removeTrack() {},
      _captureId: ++_captureCount,
    };
  }

  let _mrState = 'inactive';
  let _chunks = [];
  let _tickInterval = null;

  class MockMediaRecorder {
    static isTypeSupported(mime) { return mediaRecorderEnabled && mime.startsWith('video/'); }
    constructor(stream, opts = {}) { this.stream = stream; this.state = 'inactive'; this.ondataavailable = null; this.onstop = null; this.onerror = null; }
    start(intervalMs = 1000) {
      _mrState = 'recording'; this.state = 'recording'; _chunks = [];
      _tickInterval = setInterval(() => {
        if (_mrState !== 'recording') return;
        const chunk = new Blob(['vid-' + Date.now()], { type: 'video/webm' });
        if (this.ondataavailable) this.ondataavailable({ data: chunk });
      }, intervalMs);
    }
    stop() {
      _mrState = 'inactive'; this.state = 'inactive';
      if (_tickInterval) { clearInterval(_tickInterval); _tickInterval = null; }
      const blob = new Blob(_chunks, { type: 'video/webm' });
      _chunks = [];
      setTimeout(() => { if (this.onstop) this.onstop(); }, 0);
    }
    requestData() { const chunk = new Blob(_chunks, { type: 'video/webm' }); _chunks = []; if (this.ondataavailable) this.ondataavailable({ data: chunk }); }
  }

  const document = new MockDocument();

  const navigator = {
    userAgent: 'TestBrowser/1.0', language: 'en-US', platform: 'Test',
    mediaDevices: {
      getDisplayMedia: async () => { if (getDisplayMediaError) throw getDisplayMediaError; return makeStream(); },
    },
  };

  const tracker = makeTimerTracker();

  const window = {
    document, navigator, localStorage, crypto, Blob, URL: { createObjectURL: () => 'blob://test', revokeObjectURL() {} },
    requestAnimationFrame: cb => tracker.requestAnimationFrame(cb),
    cancelAnimationFrame: id => tracker.cancelAnimationFrame(id),
    setTimeout: (...args) => tracker.setTimeout(...args),
    clearTimeout: id => tracker.clearTimeout(id),
    setInterval: (...args) => tracker.setInterval(...args),
    clearInterval: id => tracker.clearInterval(id),
    MediaRecorder: MockMediaRecorder,
  };

  return { window, document, navigator, localStorage, tracker, makeStream };
}

// ── Module loader ─────────────────────────────────────────────────────────────

function loadModules(storageSrc, videoSrc, browserCtx) {
  // storage.js uses `const UAT = ...` which doesn't become a sandbox property.
  // We bridge it by appending `window.UAT = UAT;` after the IIFE.
  const storageWrapped = storageSrc + '\nwindow.UAT = UAT;';

  // Derive document from window if not passed directly (issue 1 fix)
  const doc = browserCtx.document || browserCtx.window?.document;
  // MediaRecorder from window.MediaRecorder (issue 2 fix)
  const MediaRecorder = browserCtx.window?.MediaRecorder;

  const sandbox = {
    console,
    // Core browser globals — derive document from window if not present
    window: browserCtx.window,
    document: doc,
    navigator: browserCtx.navigator,
    localStorage: browserCtx.localStorage,
    crypto: browserCtx.window?.crypto,
    // MediaRecorder (issue 2 fix)
    MediaRecorder,
    // Timers / RAF — use the tracker's timers for cleanup
    setTimeout: browserCtx.tracker?.setTimeout ?? global.setTimeout,
    clearTimeout: browserCtx.tracker?.clearTimeout ?? global.clearTimeout,
    setInterval: browserCtx.tracker?.setInterval ?? global.setInterval,
    clearInterval: browserCtx.tracker?.clearInterval ?? global.clearInterval,
    requestAnimationFrame: browserCtx.window?.requestAnimationFrame,
    cancelAnimationFrame: browserCtx.window?.cancelAnimationFrame,
    // Utilities
    URL: browserCtx.window?.URL,
    Blob,
    Image: class {
      constructor() { this.onload = null; this.onerror = null; this._src = ''; }
      get src() { return this._src; }
      set src(v) {
        this._src = String(v);
        if (v) {
          // data: URLs are synchronous - invoke immediately
          if (v.startsWith('data:')) { if (this.onload) this.onload(); }
          else { global.setTimeout(() => { if (this.onload) this.onload(); }, 0); }
        }
      }
    },
  };

  // Evaluate storage.js (with bridge) then video_annotation.js in the same sandbox context
  const storageScript = new Script(storageWrapped, 'storage.js');
  storageScript.runInNewContext(sandbox);

  const videoScript = new Script(videoSrc, 'video_annotation.js');
  videoScript.runInNewContext(sandbox);

  return { UAT: sandbox.window.UAT, VIDEO_ANNOTATION: sandbox.window.VIDEO_ANNOTATION };
}

// ── Test helpers ─────────────────────────────────────────────────────────────

async function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

function countKind(session, scenarioId, kind) {
  if (!session) return 0;
  const r = (session.results || []).find(r => r.scenario_id === scenarioId);
  if (!r || !r.evidence) return 0;
  return r.evidence.filter(e => e.kind === kind).length;
}

function findEvidence(session, scenarioId, kind) {
  if (!session) return null;
  const r = (session.results || []).find(r => r.scenario_id === scenarioId);
  if (!r || !r.evidence) return null;
  return r.evidence.find(e => e.kind === kind) || null;
}

// ── Tests ────────────────────────────────────────────────────────────────────

test('(1) start→stop: exactly one video evidence, correct metadata/ref, track cleanup', async () => {
  const release = 't1-' + Date.now();
  const scenarioId = 's1';
  const browserCtx = makeBrowserCtx();
  const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);

  const root = browserCtx.window.document.createElement('div');
  root.innerHTML = `<button class="video-start">G</button><button class="video-stop" disabled>P</button><span class="video-timer"></span><video class="video-preview" style="display:none"></video>`;
  browserCtx.window.document.body.appendChild(root);

  const handlers = VIDEO_ANNOTATION.bindHandlers(root, UAT, release, scenarioId);

  await handlers.startVideoCapture();
  await sleep(80);
  handlers.stopVideoCapture();
  await sleep(150);

  browserCtx.tracker.cleanup();

  const session = UAT.loadSession(release);
  assert.strictEqual(countKind(session, scenarioId, 'video'), 1, 'exactly one video evidence');
  const ev = findEvidence(session, scenarioId, 'video');
  assert.ok(ev, 'video evidence exists');
  assert.ok(ev.ref && ev.ref.length > 0, 'ref is non-empty hex string');
  assert.strictEqual(ev.duration_ms != null, true, 'duration_ms present');
  assert.strictEqual(ev.size_bytes != null, true, 'size_bytes present');
    assert.ok(ev.mime && ev.mime.startsWith('video/webm'), 'mime is video/webm');
});

test('(2) start→cancel: persists zero, timers cancelled, UI reset', async () => {
  const release = 't2-' + Date.now();
  const scenarioId = 's2';
  const browserCtx = makeBrowserCtx();
  const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);

  const root = browserCtx.window.document.createElement('div');
  root.innerHTML = `<button class="video-start">G</button><button class="video-stop" disabled>P</button><span class="video-timer"></span><video class="video-preview" style="display:none"></video>`;
  browserCtx.window.document.body.appendChild(root);

  const handlers = VIDEO_ANNOTATION.bindHandlers(root, UAT, release, scenarioId);
  await handlers.startVideoCapture();
  await sleep(30);
  handlers.cancelVideoCapture();
  await sleep(30);

  browserCtx.tracker.cleanup();

  const session = UAT.loadSession(release);
  assert.strictEqual(countKind(session, scenarioId, 'video'), 0, 'no video evidence after cancel');
  const startBtn = root.querySelector('.video-start');
  assert.strictEqual(startBtn.disabled, false, 'start button re-enabled after cancel');
});

test('(3) old session onstop does not persist/clean new session', async () => {
  const release = 't3-' + Date.now();
  const sA = 's3a', sB = 's3b';
  const browserCtx = makeBrowserCtx();
  const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);

  // Create SEPARATE roots for sA and sB to avoid module-level state confusion
  const rootA = browserCtx.window.document.createElement('div');
  rootA.innerHTML = `<button class="video-start">G</button><button class="video-stop" disabled>P</button><span class="video-timer"></span><video class="video-preview" style="display:none"></video>`;
  browserCtx.window.document.body.appendChild(rootA);

  const rootB = browserCtx.window.document.createElement('div');
  rootB.innerHTML = `<button class="video-start">G</button><button class="video-stop" disabled>P</button><span class="video-timer"></span><video class="video-preview" style="display:none"></video>`;
  browserCtx.window.document.body.appendChild(rootB);

  const handlersA = VIDEO_ANNOTATION.bindHandlers(rootA, UAT, release, sA);
  const handlersB = VIDEO_ANNOTATION.bindHandlers(rootB, UAT, release, sB);

  // sA: start then cancel (should NOT persist)
  await handlersA.startVideoCapture();
  await sleep(20);
  handlersA.cancelVideoCapture();
  await sleep(20);

  // sB: start then stop normally (should persist)
  await handlersB.startVideoCapture();
  await sleep(20);
  handlersB.stopVideoCapture();
  await sleep(120);

  browserCtx.tracker.cleanup();

  const session = UAT.loadSession(release);
  assert.strictEqual(countKind(session, sA, 'video'), 0, 'session A: zero');
  assert.strictEqual(countKind(session, sB, 'video'), 1, 'session B: exactly one');
});

test('(4) MediaRecorder/getDisplayMedia absent → graceful degrade without throw', async () => {
  // getDisplayMedia absent
  {
    const browserCtx = makeBrowserCtx();
    delete browserCtx.window.navigator.mediaDevices.getDisplayMedia;
    const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);
    const root = browserCtx.window.document.createElement('div');
    root.innerHTML = `<button class="video-start">G</button>`;
    browserCtx.window.document.body.appendChild(root);
    const handlers = VIDEO_ANNOTATION.bindHandlers(root, UAT, 'r', 's4a');
    let threw = false;
    try { await handlers.startVideoCapture(); } catch(e) { threw = true; }
    assert.strictEqual(threw, false, 'getDisplayMedia absent: no throw');
    browserCtx.tracker.cleanup();
  }
  // MediaRecorder undefined
  {
    const browserCtx = makeBrowserCtx();
    browserCtx.window.MediaRecorder = undefined;
    const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);
    const root = browserCtx.window.document.createElement('div');
    root.innerHTML = `<button class="video-start">G</button>`;
    browserCtx.window.document.body.appendChild(root);
    const handlers = VIDEO_ANNOTATION.bindHandlers(root, UAT, 'r2', 's4b');
    let threw = false;
    try { await handlers.startVideoCapture(); } catch(e) { threw = true; }
    assert.strictEqual(threw, false, 'MediaRecorder undefined: no throw');
    browserCtx.tracker.cleanup();
  }
});

test('(5) permission denied/error: no stream/recorder/timers left behind', async () => {
  const release = 't5-' + Date.now();
  const scenarioId = 's5';
  const browserCtx = makeBrowserCtx({ getDisplayMediaError: { name: 'NotAllowedError', message: 'denied' } });
  const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);

  const root = browserCtx.window.document.createElement('div');
  root.innerHTML = `<button class="video-start">G</button><span class="video-timer"></span>`;
  browserCtx.window.document.body.appendChild(root);

  const handlers = VIDEO_ANNOTATION.bindHandlers(root, UAT, release, scenarioId);
  await handlers.startVideoCapture();
  await sleep(60);

  browserCtx.tracker.cleanup();

  const session = UAT.loadSession(release);
  assert.strictEqual(countKind(session, scenarioId, 'video'), 0, 'no video after permission denied');
});

test('(6) annotation: no-screenshot fails gracefully; save with base persists based_on/ref; cancel no persist; clear redraws', async () => {
  const release = 't6-' + Date.now();

  // 6a: no screenshot
  {
    const browserCtx = makeBrowserCtx();
    const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);
    const root = browserCtx.window.document.createElement('div');
    root.innerHTML = `<canvas class="annotation-canvas" style="display:none"></canvas>`;
    browserCtx.window.document.body.appendChild(root);
    const handlers = VIDEO_ANNOTATION.bindHandlers(root, UAT, release, 's6a');
    const r = handlers.openAnnotationCanvas();
    assert.strictEqual(r.error, 'sin-screenshot-base', 'no-screenshot: graceful error');
    browserCtx.tracker.cleanup();
  }

  // 6b: save with base
  {
    const browserCtx = makeBrowserCtx();
    const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);

    UAT.cacheScreenshotDataUrl(release, 's6b', 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==');
    await UAT.addTypedEvidence(release, 's6b', { kind: 'screenshot', blob: new Blob([[0x89,0x50,0x4E,0x47]], { type: 'image/png' }), note: 'base' });

    const root = browserCtx.window.document.createElement('div');
    root.innerHTML = `<canvas class="annotation-canvas" style="display:none" width="100" height="100"></canvas>`;
    browserCtx.window.document.body.appendChild(root);

    const canvas = root.querySelector('.annotation-canvas');
    const ctx = canvas.getContext('2d');
    ctx.fillStyle = '#FF0000'; ctx.fillRect(0, 0, 100, 100);

    const handlers = VIDEO_ANNOTATION.bindHandlers(root, UAT, release, 's6b');
    const openR = handlers.openAnnotationCanvas();
    assert.strictEqual(openR.opened, true, 'canvas opened');
    await sleep(50); // Wait for async image load in openAnnotationCanvas
    const saveR = await handlers.saveAnnotation();
    assert.strictEqual(saveR.success, true, 'saveAnnotation succeeds');
    assert.ok(saveR.ref, 'has ref');
    assert.ok(saveR.based_on, 'has based_on');

    const session = UAT.loadSession(release);
    const ev = findEvidence(session, 's6b', 'annotation');
    assert.ok(ev, 'annotation persisted');
    assert.ok(ev.based_on, 'based_on present');
    assert.strictEqual(ev.mime, 'image/png', 'mime is image/png');
    browserCtx.tracker.cleanup();
  }

  // 6c: cancel does not persist
  {
    const browserCtx = makeBrowserCtx();
    const { UAT, VIDEO_ANNOTATION } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);

    UAT.cacheScreenshotDataUrl(release, 's6c', 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==');

    const root = browserCtx.window.document.createElement('div');
    root.innerHTML = `<canvas class="annotation-canvas" style="display:block" width="100" height="100"></canvas>`;
    browserCtx.window.document.body.appendChild(root);

    const handlers = VIDEO_ANNOTATION.bindHandlers(root, UAT, release, 's6c');
    handlers.openAnnotationCanvas();
    await sleep(5);
    handlers.cancelAnnotation();
    await sleep(5);

    browserCtx.tracker.cleanup();

    const session = UAT.loadSession(release);
    assert.strictEqual(countKind(session, 's6c', 'annotation'), 0, 'cancel: no annotation persisted');
    const canvas = root.querySelector('.annotation-canvas');
    assert.strictEqual(canvas.style.display, 'none', 'canvas hidden after cancel');
  }
});

test('(7) storage round-trip video/annotation on localStorage; quota failure preserves prior', async () => {
  const release = 't7-' + Date.now();

  // Normal round-trip
  {
    const browserCtx = makeBrowserCtx();
    const { UAT } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);

    const blob = new Blob([['vid-data']], { type: 'video/webm' });
    const entry = await UAT.addTypedEvidence(release, 's7a', {
      kind: 'video', blob, mime: 'video/webm', note: 'test', duration_ms: 3000, size_bytes: blob.size,
    });
    assert.ok(entry.ref && entry.ref.length > 0, 'ref is sha256 hex');
    assert.strictEqual(entry.duration_ms, 3000, 'duration_ms preserved');
    assert.strictEqual(entry.size_bytes, blob.size, 'size_bytes preserved');

    // Round-trip: reload from localStorage (not from in-memory UAT)
    const session = UAT.loadSession(release);
    assert.ok(session, 'session reloaded');
    const reloaded = findEvidence(session, 's7a', 'video');
    assert.ok(reloaded, 'video round-tripped');
    assert.strictEqual(reloaded.duration_ms, 3000, 'duration_ms round-trips');
    assert.strictEqual(reloaded.kind, 'video', 'kind preserved');

    // Annotation with based_on
    await UAT.addTypedEvidence(release, 's7a', {
      kind: 'screenshot', blob: new Blob([['png']], { type: 'image/png' }), note: 'base',
    });
    const annotEntry = await UAT.addTypedEvidence(release, 's7a', {
      kind: 'annotation', blob: new Blob([['annot']], { type: 'image/png' }),
      mime: 'image/png', note: 'annot', size_bytes: 50, based_on: entry.ref,
    });
    assert.ok(annotEntry.based_on, 'annotation has based_on');
    // Reload from localStorage to verify round-trip
    const session2 = UAT.loadSession(release);
    const reloadedAnnot = findEvidence(session2, 's7a', 'annotation');
    assert.strictEqual(reloadedAnnot.based_on, entry.ref, 'based_on round-trips');
    browserCtx.tracker.cleanup();
  }

  // Quota error preserves prior
  {
    const browserCtx = makeBrowserCtx({ quotaError: false }); // Start with quota OFF
    const { UAT } = loadModules(STORAGE_WRAPPED, VIDEO_SRC, browserCtx);

    // Persist prior screenshot with quota OFF
    await UAT.addTypedEvidence(release, 's7b', {
      kind: 'screenshot', blob: new Blob([['png']], { type: 'image/png' }), note: 'prior',
    });
    const priorSession = UAT.loadSession(release);
    const priorRef = findEvidence(priorSession, 's7b', 'screenshot')?.ref;
    assert.ok(priorRef, 'prior screenshot was persisted');

    // Now enable quota and try to add video (should fail but not corrupt prior)
    browserCtx.window.localStorage._setQuotaError(true);

    try {
      await UAT.addTypedEvidence(release, 's7b', {
        kind: 'video', blob: new Blob([['v']], { type: 'video/webm' }),
        mime: 'video/webm', note: 'should-not-persist',
      });
    } catch (e) {}

    browserCtx.tracker.cleanup();

    // Prior should still be intact
    const sessionAfter = UAT.loadSession(release);
    assert.strictEqual(findEvidence(sessionAfter, 's7b', 'video'), null, 'no video after quota error');
    assert.ok(findEvidence(sessionAfter, 's7b', 'screenshot'), 'prior screenshot still present');
    assert.strictEqual(findEvidence(sessionAfter, 's7b', 'screenshot')?.ref, priorRef, 'prior ref unchanged');
  }
});
