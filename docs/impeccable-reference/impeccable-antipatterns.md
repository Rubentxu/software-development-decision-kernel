# Impeccable antipatterns

What the upstream detector looks for, in the two families it names. Every
category below is quoted from the
[upstream README](https://github.com/pbakaus/impeccable); the count it gives is
**61 deterministic issues**, not the 46 that
[`agents/impeccable-primary.md`](../../agents/impeccable-primary.md) states.

**This is not the rule list.** The detector has 61; the upstream enumerates them
in its own docs and they change with releases. What follows is the *taxonomy* —
which families exist and what each family looks like — because that is the part
that survives an upstream bump and the part you need to recognise a finding you
are reading rather than one the tool produced.

Upstream's own framing: the detector catches issues "across AI slop … and
general design quality".

## Family 1 — AI slop

The tells that a page was generated rather than designed. Upstream names four
explicitly:

| antipattern | what it looks like | why it reads as generated |
|---|---|---|
| **side-tab borders** | vertical rules down the side of cards, sections or list items, used as a decorative device | carries no information; a real hierarchy uses spacing, weight or indentation |
| **purple gradients** | the default `from-purple-500 to-purple-700` fill on a hero or CTA | it is the most common output of a model asked for "a modern gradient" — its presence is evidence of no decision having been made |
| **bounce easing** | `cubic-bezier` curves that overshoot and settle (`easeOutBack`, spring with bounce) applied to ordinary UI transitions | motion with elasticity on a list row or a fade reads as a demo, not a product |
| **dark glows** | blurred coloured shadows (`drop-shadow` with a saturated hue, radial glows behind text or icons) | glow implies emission; nothing on screen emits |

The common thread: each is a **default**. A generated page gets them because the
model's prior for "polished UI" includes them. That is exactly why they are
detectable — a rule with a strong prior is cheap to check.

## Family 2 — general design quality

Not slop-specific; these fail on hand-built pages too. Upstream names four and
appends "and more":

| antipattern | what it looks like | the actual defect |
|---|---|---|
| **line length** | body copy running to 120+ characters | measure drops past the comfortable range and the reader starts losing the return sweep |
| **cramped padding** | text sitting a few pixels from its container edge | the element has no internal rhythm; content touches the boundary it is supposed to be inset from |
| **small touch targets** | interactive elements under ~44×44 CSS px | the target is smaller than the finger that has to hit it — a hit-area defect, not an aesthetic one |
| **skipped headings** | `<h2>` before any `<h1>`, or a heading level jumped by two | the document outline no longer describes the structure, so heading-based navigation and screen-reader heading lists both lie |

The last two are the ones that survive into production most often, because
neither throws and neither looks wrong in a screenshot.

## The one that is not in either family

Upstream flags a category of its own: **drift from the design system** —
`detector.designSystem.enabled` compares a surface against the tokens declared
in config. A page can pass every rule in both families above and still be
inconsistent with the rest of the product, because the rules are about
craft, not about *this* system.

That is the reason a clean scan is evidence and not proof.

## How to use this list

- **Read a finding**: match it against the families above to know whether it is
  a slop tell (someone should have decided differently) or a quality defect
  (someone should have measured it). They get different fixes.
- **Scan by hand**: family 1 is cheap to grep for. `easeOutBack`, `bounce`, a
  saturated `drop-shadow` and a purple-to-purple gradient are all literal
  strings. This list exists so a surface can be screened before the tool runs.
- **Do not treat it as exhaustive.** 61 rules, two families, four named
  examples each. Anything not here is not thereby absent.
- **Do not add rules to this file from memory.** If the detector has a rule you
  want documented, take it from the upstream docs and say which release it came
  from. This file's value is that everything in it is sourced; one invented
  entry spends that.

Related: [`README.md`](./README.md) for how to run the detector and what its exit
codes mean.
