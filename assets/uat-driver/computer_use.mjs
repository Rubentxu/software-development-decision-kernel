#!/usr/bin/env node
// SDDK UAT computer-use harness (ADR-014, F8) — observe→think→act con Fara.
//
// Sensor + actuador, NUNCA juez: ejecuta un goal de forma autónoma con un
// budget de pasos, registrando la trayectoria completa (screenshot + decisión
// + acción + resultado) en `trajectory.json` + `screenshot-<step>.png`.
// La evaluación (PASS/FAIL) pertenece a los oracles y al humano.
//
// Uso:
//   node computer_use.mjs --url <url> --goal "<goal>" --output <dir>
//     [--fara-url http://127.0.0.1:8082] [--max-steps 10] [--timeout ms]

import { createRequire } from "node:module";
import { execSync } from "node:child_process";
const require = createRequire(import.meta.url);

function resolvePlaywright() {
  const roots = new Set();
  for (const dir of (process.env.NODE_PATH ?? "").split(path.delimiter).filter(Boolean)) {
    roots.add(dir);
  }
  const home = process.env.HOME ?? "";
  roots.add(path.join(home, ".bun", "install", "global", "node_modules"));
  try {
    const npmRoot = execSync("npm root -g", { encoding: "utf8" }).trim();
    if (npmRoot) roots.add(npmRoot);
  } catch {}
  for (const dir of roots) {
    try {
      return require(require.resolve("playwright", { paths: [dir] }));
    } catch {}
  }
  return require("playwright");
}

const { chromium } = resolvePlaywright();

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
  console.error(`computer_use: ${message}`);
  process.exit(1);
}

const args = parseArgs(process.argv);
const url = args.url || fail("--url is required");
const goal = args.goal || fail("--goal is required");
const output = args.output || fail("--output is required");
const faraUrl = args["fara-url"] || process.env.FARA_URL || "http://127.0.0.1:8082";
const maxSteps = Number(args["max-steps"] || 10);
const timeout = Number(args.timeout || 60000);

fs.mkdirSync(output, { recursive: true });

const SYSTEM_PROMPT = `You are a browser automation agent (sensor + actuator). You observe a screenshot and decide ONE next action toward the goal.
Reply with EXACTLY ONE line, nothing else, one of:
ACTION:click x y        (x,y are CSS pixel coordinates on the screenshot)
ACTION:type TEXT        (type text into the focused input; add ENTER to press Enter)
ACTION:done             (goal achieved or no further action helps)
Never evaluate pass/fail. Never output anything but the single ACTION line.`;

async function think(browser, page, step, trajectory) {
  const screenshot = await page.screenshot({ type: "png" });
  const b64 = screenshot.toString("base64");
  const shotPath = path.join(output, `screenshot-${String(step).padStart(2, "0")}.png`);
  fs.writeFileSync(shotPath, screenshot);

  const body = {
    model: "llama-cpp",
    messages: [
      { role: "system", content: SYSTEM_PROMPT },
      {
        role: "user",
        content: [
          { type: "text", text: `Goal: ${goal}\nStep ${step}/${maxSteps}. Next ACTION line:` },
          { type: "image_url", image_url: { url: `data:image/png;base64,${b64}` } },
        ],
      },
    ],
    max_tokens: 128,
    temperature: 0.2,
  };
  const response = await fetch(`${faraUrl}/v1/chat/completions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(timeout),
  });
  if (!response.ok) {
    throw new Error(`Fara returned ${response.status}: ${await response.text()}`);
  }
  const data = await response.json();
  const raw = data.choices?.[0]?.message?.content ?? "";
  const reasoning = data.choices?.[0]?.message?.reasoning_content ?? "";
  trajectory.push({
    step,
    screenshot: shotPath,
    raw_decision: raw || reasoning,
    action: null,
    result: null,
  });
  return { raw: raw || reasoning, trajectory_entry: trajectory[trajectory.length - 1] };
}

function parseDecision(raw) {
  if (!raw) return null;
  // Find the FIRST ACTION: line anywhere in the output (content or
  // reasoning); the model may prefix with <think> blocks.
  const match = raw.match(/ACTION:\s*(\w+)\s*([^\n]*)/i);
  if (!match) return null;
  const action = match[1].toLowerCase();
  const rest = match[2].trim();
  if (action === "done") return { action: "done" };
  if (action === "click") {
    const coords = rest.split(/[\s,]+/).map(Number).filter((n) => Number.isFinite(n));
    if (coords.length >= 2) return { action: "click", x: coords[0], y: coords[1] };
    return null;
  }
  if (action === "type") {
    const pressEnter = /\benter\b/i.test(rest);
    const text = rest.replace(/\benter\b/i, "").trim();
    return { action: "type", text, press_enter: pressEnter };
  }
  return null;
}

async function act(page, decision) {
  if (!decision) return { result: "unparseable", done: false };
  switch (decision.action) {
    case "click":
      if (typeof decision.x !== "number" || typeof decision.y !== "number") {
        return { result: "missing coordinates", done: false };
      }
      await page.mouse.click(decision.x, decision.y);
      return { result: `clicked (${decision.x},${decision.y})`, done: false };
    case "type":
      if (typeof decision.text !== "string") {
        return { result: "missing text", done: false };
      }
      await page.keyboard.type(decision.text, { delay: 10 });
      if (decision.press_enter) {
        await page.keyboard.press("Enter");
      }
      return { result: `typed ${JSON.stringify(decision.text)}`, done: false };
    case "done":
      return { result: "goal declared done by agent", done: true };
    default:
      return { result: `unknown action ${decision.action}`, done: false };
  }
}

async function run() {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1366, height: 768 },
  });
  const page = await context.newPage();
  const trajectory = [];

  try {
    await page.goto(url, { waitUntil: "domcontentloaded", timeout });
  } catch (err) {
    await browser.close();
    fail(`navigation failed: ${err.message}`);
  }

  let done = false;
  let steps = 0;
  let lastActionKey = null;
  let repeatCount = 0;
  let stopReason = null;
  while (!done && steps < maxSteps) {
    steps += 1;
    const { raw, trajectory_entry } = await think(browser, page, steps, trajectory);
    const decision = parseDecision(raw);
    let outcome;
    try {
      outcome = await act(page, decision);
    } catch (err) {
      outcome = { result: `action error: ${err.message}`, done: false };
    }
    trajectory_entry.action = decision ?? { action: "unparseable" };
    trajectory_entry.result = outcome.result;
    if (outcome.done) {
      done = true;
      stopReason = "agent_done";
      break;
    }
    // No-progress guard: 3 identical actions in a row => stop burning budget.
    const key = decision ? JSON.stringify(decision) : "unparseable";
    if (key === lastActionKey) {
      repeatCount += 1;
      if (repeatCount >= 3) {
        stopReason = "no_progress";
        break;
      }
    } else {
      lastActionKey = key;
      repeatCount = 1;
    }
    await page.waitForTimeout(300);
  }
  if (!stopReason) stopReason = done ? "agent_done" : "max_steps";

  // Final screenshot.
  const finalShot = path.join(output, "screenshot.png");
  await page.screenshot({ path: finalShot, fullPage: true });

  fs.writeFileSync(path.join(output, "trajectory.json"), JSON.stringify(trajectory, null, 2));
  fs.writeFileSync(
    path.join(output, "summary.json"),
    JSON.stringify(
      {
        goal,
        url,
        steps_taken: steps,
        max_steps: maxSteps,
        done: stopReason === "agent_done",
        stop_reason: stopReason,
        final_url: page.url(),
        page_title: await page.title().catch(() => null),
        fara_url: faraUrl,
      },
      null,
      2,
    ),
  );

  await browser.close();
  console.error(`computer_use: ${steps} step(s), done=${done}`);
  process.exit(0);
}

run().catch((err) => {
  console.error(`computer_use: ${err.stack ?? err.message}`);
  process.exit(1);
});
