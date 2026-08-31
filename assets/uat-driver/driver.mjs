#!/usr/bin/env node
// SDDK UAT driver — sensor + actuador de browser, NUNCA juez (ADR-014).
//
// Invoqué desde `sddk_gateway::playwright::run_playwright` vía `node`.
// Navega, ejecuta acciones opcionales, captura evidencia (screenshot,
// console, network, dom, aria, geometry, trace) y escribe `summary.json`
// + artefactos en el directorio de salida. Nunca emite PASS/FAIL.
//
// Uso:
//   node driver.mjs --url <url> --output <dir> [--viewport WxH]
//     [--actions actions.json] [--screenshot] [--trace] [--console]
//     [--network] [--dom] [--geometry selectors.json] [--timeout ms]
//   node driver.mjs --inspect --url <url> --output <dir> [--viewport WxH]

// Resolve `playwright` manually: ESM import ignores NODE_PATH and Node 25
// CJS also ignores it, so walk known global module roots ourselves.
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
const require = createRequire(import.meta.url);

function globalModuleRoots() {
  const roots = new Set();
  for (const dir of (process.env.NODE_PATH ?? "").split(path.delimiter).filter(Boolean)) {
    roots.add(dir);
  }
  // Bun global modules (common on dev machines).
  const home = process.env.HOME ?? "";
  roots.add(path.join(home, ".bun", "install", "global", "node_modules"));
  // npm global root, if npm is available.
  try {
    const npmRoot = execSync("npm root -g", { encoding: "utf8" }).trim();
    if (npmRoot) roots.add(npmRoot);
  } catch {
    // npm unavailable — keep going with the roots we have.
  }
  return [...roots];
}

function resolvePlaywright() {
  for (const dir of globalModuleRoots()) {
    try {
      const resolved = require.resolve("playwright", { paths: [dir] });
      return require(resolved);
    } catch {
      // try next root
    }
  }
  // Fall back to normal resolution (project-local node_modules).
  return require("playwright");
}

const { chromium, firefox, webkit } = resolvePlaywright();

import fs from "node:fs";
import path from "node:path";

function parseArgs(argv) {
  const args = {};
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next !== undefined && !next.startsWith("--")) {
        args[key] = next;
        i++;
      } else {
        args[key] = true;
      }
    }
  }
  return args;
}

function fail(message) {
  console.error(`driver: ${message}`);
  process.exit(1);
}

const args = parseArgs(process.argv);
const url = args.url || fail("--url is required");
const output = args.output || fail("--output is required");
const timeout = Number(args.timeout || 30000);
const browserName = args.browser || "chromium";
const viewport = args.viewport ? args.viewport.split("x").map(Number) : [1366, 768];

fs.mkdirSync(output, { recursive: true });

const browsers = { chromium, firefox, webkit };
const browserType = browsers[browserName] || fail(`unknown browser: ${browserName}`);

const consoleMessages = [];
const networkFailures = [];

async function run() {
  const browser = await browserType.launch({ headless: true });
  const contextOptions = {
    viewport: { width: viewport[0], height: viewport[1] },
  };
  const trace = args.trace === true || args.trace === "true";
  if (trace) contextOptions.recordVideo = undefined; // trace via tracing API below
  const context = await browser.newContext(contextOptions);
  const page = await context.newPage();

  page.on("console", (msg) => {
    if (args.console) consoleMessages.push({ type: msg.type(), text: msg.text() });
  });
  page.on("requestfailed", (req) => {
    if (args.network) {
      networkFailures.push({
        url: req.url(),
        method: req.method(),
        failure: req.failure()?.errorText ?? "unknown",
      });
    }
  });

  if (trace) {
    await context.tracing.start({ screenshots: true, snapshots: true });
  }

  let pageTitle = null;
  let httpStatus = null;
  let httpUrl = null;
  let httpContentType = null;
  try {
    const response = await page.goto(url, { waitUntil: "domcontentloaded", timeout });
    pageTitle = await page.title();
    if (response) {
      httpStatus = response.status();
      httpUrl = response.url();
      httpContentType = response.headers()["content-type"] ?? null;
    }
  } catch (err) {
    await browser.close();
    fail(`navigation failed: ${err.message}`);
  }

  // Actions file: [{ type: "click"|"fill"|"press"|"wait"|"goto", selector?, value?, url? }]
  if (args.actions) {
    let actions;
    try {
      actions = JSON.parse(fs.readFileSync(args.actions, "utf8"));
    } catch (err) {
      await browser.close();
      fail(`cannot read actions: ${err.message}`);
    }
    for (const action of actions) {
      try {
        if (action.type === "click") {
          await page.click(action.selector, { timeout });
        } else if (action.type === "fill") {
          await page.fill(action.selector, action.value);
        } else if (action.type === "press") {
          await page.press(action.selector, action.value);
        } else if (action.type === "wait") {
          await page.waitForTimeout(action.value ?? 500);
        } else if (action.type === "goto") {
          await page.goto(action.url, { waitUntil: "domcontentloaded", timeout });
        }
      } catch (err) {
        await browser.close();
        fail(`action ${action.type} failed: ${err.message}`);
      }
    }
  }

  const finalUrl = page.url();

  // DOM snapshot.
  if (args.dom) {
    fs.writeFileSync(path.join(output, "dom.html"), await page.content());
  }

  // ARIA snapshot (best-effort; not exposed in all channels).
  if (args.aria) {
    try {
      const snapshot = await page.accessibility.snapshot();
      fs.writeFileSync(path.join(output, "aria.json"), JSON.stringify(snapshot, null, 2));
    } catch {
      fs.writeFileSync(path.join(output, "aria.json"), JSON.stringify({ error: "unavailable" }));
    }
  }

  // Geometry: bounding boxes for selectors.
  if (args.geometry) {
    let selectors;
    try {
      selectors = JSON.parse(fs.readFileSync(args.geometry, "utf8"));
    } catch (err) {
      await browser.close();
      fail(`cannot read geometry selectors: ${err.message}`);
    }
    const boxes = {};
    for (const selector of selectors) {
      const locator = page.locator(selector);
      try {
        const box = await locator.boundingBox();
        boxes[selector] = box ?? null;
      } catch {
        boxes[selector] = null;
      }
    }
    fs.writeFileSync(path.join(output, "geometry.json"), JSON.stringify(boxes, null, 2));
  }

  // Screenshot.
  if (args.screenshot) {
    await page.screenshot({ path: path.join(output, "screenshot.png"), fullPage: true });
  }

  // Console + network dumps.
  if (args.console) {
    fs.writeFileSync(path.join(output, "console.json"), JSON.stringify(consoleMessages, null, 2));
  }
  if (args.network) {
    fs.writeFileSync(
      path.join(output, "network.json"),
      JSON.stringify(networkFailures, null, 2),
    );
  }

  // HTTP response snapshot (status oracle).
  if (httpStatus !== null) {
    fs.writeFileSync(
      path.join(output, "http.json"),
      JSON.stringify(
        {
          status: httpStatus,
          url: httpUrl,
          content_type: httpContentType,
        },
        null,
        2,
      ),
    );
  }

  // Trace archive.
  if (trace) {
    await context.tracing.stop({ path: path.join(output, "trace.zip") });
  }

  // Inspect mode: capture fingerprint for staleness diff.
  if (args.inspect) {
    // Collect visible text content.
    const texts = await page.evaluate(() => {
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
        acceptNode: (node) => {
          if (!node.textContent.trim()) return NodeFilter.FILTER_REJECT;
          const style = window.getComputedStyle(node.parentElement);
          if (style.display === "none" || style.visibility === "hidden") return NodeFilter.FILTER_REJECT;
          return NodeFilter.FILTER_ACCEPT;
        }
      });
      const nodes = [];
      let node;
      while ((node = walker.nextNode())) {
        const t = node.textContent.trim();
        if (t.length > 0) nodes.push(t);
      }
      return [...new Set(nodes)];
    });

    // Collect meaningful selectors (ids, data-*, aria labels).
    const selectors = await page.evaluate(() => {
      const found = new Set();
      // IDs
      document.querySelectorAll("[id]").forEach(el => {
        if (el.id) found.add(`#${el.id}`);
      });
      // data-testid / data-* attributes
      document.querySelectorAll("[data-testid]").forEach(el => {
        found.add(`[data-testid="${el.getAttribute("data-testid")}"]`);
      });
      document.querySelectorAll("[data-cy]").forEach(el => {
        found.add(`[data-cy="${el.getAttribute("data-cy")}"]`);
      });
      document.querySelectorAll("[aria-label]").forEach(el => {
        found.add(`[aria-label="${el.getAttribute("aria-label")}"]`);
      });
      // Common semantic selectors
      ["button", "a", "input", "select", "textarea"].forEach(tag => {
        document.querySelectorAll(tag).forEach((el, i) => {
          if (el.name) found.add(`${tag}[name="${el.name}"]`);
          if (el.type && el.type !== "hidden") found.add(`${tag}[type="${el.type}"]`);
        });
      });
      return [...found];
    });

    const fingerprint = {
      url,
      timestamp: new Date().toISOString(),
      viewport: { width: viewport[0], height: viewport[1] },
      texts,
      selectors,
    };

    fs.writeFileSync(path.join(output, "fingerprint.json"), JSON.stringify(fingerprint, null, 2));
  }

  // Summary (the executor parses this).
  fs.writeFileSync(
    path.join(output, "summary.json"),
    JSON.stringify(
      {
        page_title: pageTitle,
        final_url: finalUrl,
        console_messages: consoleMessages.length,
        network_failures: networkFailures.length,
      },
      null,
      2,
    ),
  );

  await browser.close();
  process.exit(0);
}

run().catch((err) => {
  console.error(`driver: ${err.stack ?? err.message}`);
  process.exit(1);
});
