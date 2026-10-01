# Impeccable — integration overview

Reference for [`agents/impeccable-primary.md`](../../agents/impeccable-primary.md), the
SDDK agent that wraps the **upstream** `impeccable` design skill.

## Read this first: the skill is not in this repository

`impeccable` is an **external dependency**, not a bundled SDDK surface. The agent
expects it installed at a path the user chooses (`<your-impeccable-skill-path>/`),
typically `.opencode/skills/impeccable/`. Nothing in this repo ships it, and
nothing here should be treated as a copy of it.

That is why these are *reference notes* and not vendored source: they describe
how to drive the upstream tool, and they go stale when upstream does. The
upstream is the authority; this file is an index into it.

## What the upstream is, with sources

| claim | value | source |
|---|---|---|
| description | "The design language that makes your AI harness better at design." | [repo README](https://github.com/pbakaus/impeccable) |
| author / license | Paul Bakaus · Apache 2.0 | [repo README](https://github.com/pbakaus/impeccable) |
| detector size | **61 deterministic issues** | [repo README](https://github.com/pbakaus/impeccable) |
| docs | <https://impeccable.style/docs/detector> | — |

### The number the agent gets wrong

`impeccable-primary.md` states **"23 commands, 46-rule detector"** and promises
these files contain *"46 rules distilled"*. The upstream README currently says
the detector catches **61** deterministic issues. The `46` is stale or was never
right; either way it is a number this repo repeated without a source.

**Do not restore `46` from memory.** If you need a count, read the upstream
README and the release notes, and put the date next to it. A count with no date
is a count that rots — the same failure as `prompts_count = 0` and the "21
bundled skills" line that turned out to be 22.

The `23 commands` figure was **not verifiable** from the sources consulted, so
it is not repeated here. Treat it as unconfirmed.

## Driving the detector

```bash
npx impeccable detect .                          # scan a directory
npx impeccable detect src/components/Card.tsx    # one file
npx impeccable detect https://example.com        # rendered page (needs Chrome/Chromium/Edge)
npx impeccable detect --json .                   # CI-friendly, on stdout
```

Findings go to **stderr**; redirect with `2> findings.txt`. `--json` writes
machine-readable results to stdout.

### Exit codes — these are the contract

| code | meaning |
|---|---|
| `0` | scan completed, no primary findings (advisories may still exist) |
| `2` | scan completed **with** primary findings |
| `1` | at least one target could not be scanned — **treat as a scan failure**, even if other targets produced findings |

Operational failure takes precedence over findings on a partial multi-target
scan. A pipeline that only checks "exit == 0" will read a broken scan as a
clean one, which is the shape of the defect that
[INC-DEBT-055](../../docs/debt/INC-DEBT-055-UAT-VERDICT-READY-OVER-AN-EMPTY-SESSION.md)
is about.

## What a clean run does and does not prove

From the upstream docs, verbatim in substance: **a clean detector run is
evidence, not proof.** It means these specific checks found nothing reportable.
It does not replace inspecting the rendered experience across viewports, and it
is not an accessibility audit. Report it as what it is.

## Keeping an intentional choice

Global, per file, or per value:

```bash
npx impeccable ignores list
npx impeccable ignores add-file "src/legacy/**"
npx impeccable ignores add-value overused-font Inter --reason "Brand font"
```

Prefer `add-value` over `add-rule` when a specific value is the problem: the
rule stays active for everything else.

Config lives in `.impeccable/config.json` and `.impeccable/config.local.json`,
with `detector.ignoreRules`, `detector.ignoreFiles`, `detector.ignoreValues`,
`detector.designSystem.enabled` and `hook.enabled`. `hook.*` only affects
automatic hook execution, not manual scans.

A waiver that should travel with **one file** rather than the repo config:

```html
<!-- impeccable-disable overused-font: exported brand doc -->
<!-- impeccable-disable-line ... -->
<!-- impeccable-disable-next-line ... -->
```

Any comment syntax works; the marker scopes to the whole file (or the line,
with the two line-scoped variants). `--no-inline-ignores` and `--no-config`
bypass them.

## Supported input types

Folders are searched for supported UI and style files. HTML scans include
linked local CSS. JSX, TSX, Vue, Svelte, Astro and stylesheet files get source
checks. Extra server-side template extensions (Blade, ERB, …) are declared
under `detector.extensions`. URL scans inspect the rendered DOM, computed
layout and accessible linked stylesheets — browser security still blocks
reading cross-origin CSS without CORS.

## Where to go next

- [`impeccable-antipatterns.md`](./impeccable-antipatterns.md) — the categories
  the detector looks for, and which ones are cheap to grep for by hand.
- <https://impeccable.style/docs/detector> — authoritative detector docs.
- <https://impeccable.style/slop> — visual specimens of AI slop.
- <https://impeccable.style/detector> — detector playground.
