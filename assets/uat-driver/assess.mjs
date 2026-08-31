#!/usr/bin/env node
// SDDK UAT semantic oracle harness (ADR-014, F9) — visual_ai / llm_rubric.
//
// Juez SEMÁNTICO preliminar: evalúa evidencia (screenshot o texto) contra
// una rúbrica con un VLM/LLM local (Fara / llama.cpp). Emite
// `assessment.json` con verdict (pass|fail|uncertain) + confidence + detalle.
// NUNCA es la autoridad final: confidence < 1.0 y la decisión de aceptación
// pertenece al humano (REQ-RF-023).
//
// Uso:
//   node assess.mjs --kind visual_ai --screenshot <png> \
//     --rubric <json> --output <dir> [--fara-url http://127.0.0.1:8082]
//   node assess.mjs --kind llm_rubric --text <file> \
//     --rubric <json> --output <dir> [--fara-url ...]

import { createRequire } from "node:module";
import { execSync } from "node:child_process";
const require = createRequire(import.meta.url);

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
  console.error(`assess: ${message}`);
  process.exit(1);
}

const args = parseArgs(process.argv);
const kind = args.kind || fail("--kind visual_ai|llm_rubric is required");
const output = args.output || fail("--output is required");
const rubricFile = args.rubric || fail("--rubric <json> is required");
const faraUrl = args["fara-url"] || process.env.FARA_URL || "http://127.0.0.1:8082";
const timeout = Number(args.timeout || 90000);

fs.mkdirSync(output, { recursive: true });
let rubric;
try {
  rubric = JSON.parse(fs.readFileSync(rubricFile, "utf8"));
} catch (err) {
  fail(`cannot read rubric: ${err.message}`);
}
// Rubric: array of strings or object {criteria: [...]}.
const criteria = Array.isArray(rubric)
  ? rubric
  : rubric.criteria ?? [JSON.stringify(rubric)];

let content;
if (kind === "visual_ai") {
  const shot = args.screenshot || fail("--screenshot <png> is required for visual_ai");
  const b64 = fs.readFileSync(shot).toString("base64");
  content = [
    { type: "text", text: "Screenshot to assess:" },
    { type: "image_url", image_url: { url: `data:image/png;base64,${b64}` } },
  ];
} else if (kind === "llm_rubric") {
  const textFile = args.text || fail("--text <file> is required for llm_rubric");
  content = fs.readFileSync(textFile, "utf8").slice(0, 16_000);
} else {
  fail(`unknown kind ${kind}`);
}

const system = `You are a UAT assessor. Evaluate the evidence against the criteria and reply with EXACTLY TWO LINES:
LINE1: VERDICT:pass|fail|uncertain CONFIDENCE:0.0-1.0
LINE2: one sentence explaining the verdict
Criteria:
${criteria.map((c, i) => `${i + 1}. ${c}`).join("\n")}
Rules: pass = evidence satisfies all criteria; fail = a criterion is clearly violated; uncertain = cannot determine from the evidence. Confidence reflects how sure you are (0.0-1.0). Never output anything but the two lines.`;

const body = {
  model: "llama-cpp",
  messages: [
    { role: "system", content: system },
    { role: "user", content: Array.isArray(content) ? content : content },
  ],
  max_tokens: 256,
  temperature: 0.2,
};

const response = await fetch(`${faraUrl}/v1/chat/completions`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(body),
  signal: AbortSignal.timeout(timeout),
});
if (!response.ok) {
  fail(`Fara returned ${response.status}: ${await response.text()}`);
}
const data = await response.json();
const raw = data.choices?.[0]?.message?.content ?? "";
const reasoning = data.choices?.[0]?.message?.reasoning_content ?? "";
const full = raw || reasoning;

// Parse the two-line contract from anywhere in the output.
const verdictMatch = full.match(/VERDICT:\s*(pass|fail|uncertain)/i);
const confMatch = full.match(/CONFIDENCE:\s*([0-9]*\.?[0-9]+)/i);
const verdict = verdictMatch ? verdictMatch[1].toLowerCase() : "uncertain";
const confidence = confMatch
  ? Math.max(0, Math.min(1, Number(confMatch[1])))
  : 0.0;
const details =
  full
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && !/^VERDICT:/i.test(l) && !/^CONFIDENCE:/i.test(l))
    .join(" ") || "no explanation";

fs.writeFileSync(
  path.join(output, "assessment.json"),
  JSON.stringify({ verdict, confidence, details, model: "fara-9b", raw: full.slice(0, 2000) }, null, 2),
);
console.error(`assess: ${verdict} conf=${confidence.toFixed(2)}`);
process.exit(0);
