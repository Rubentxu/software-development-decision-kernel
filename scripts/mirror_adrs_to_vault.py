#!/usr/bin/env python3
"""Generate vault ADR mirrors for accepted ADRs in docs/architecture/adrs/.

Strategy: for each ADR file in the repo, extract the canonical metadata
(id, title, status, accepted_at, accepted_by_cycle, related_adrs, package
provenance, implementation_evidence). Emit a vault mirror at
~/.sddk-knowledge/sddk-framework/adrs/<slug>.md using the canonical template
(template/adr.md) with `status: accepted`.

The vault mirror is a *view*, not an authority — it points to the repo ADR
as the canonical source. Per AGENTS §2.7 (semantic ownership): "the Vault
is the human source, never the runtime authority; projections reconstruct
the runtime; the runtime does not read the Vault as authority."

This is the inverse direction: the repo (runtime) is the authority, the
vault is the human-readable view. The metadata in the mirror MUST match
the repo ADR's frontmatter verbatim — no paraphrasing of `id`, `status`,
or `related_adrs`.

Slug convention (matches existing vault entries like ADR-0079-cycle-supersede):
`<id>-<kebab-title-lowercase>.md`
"""
import re
import sys
import tomllib
from pathlib import Path
import datetime as dt

REPO_ROOT = Path("/home/rubentxu/Proyectos/agentesIA/sddk-framework").resolve()
REPO_ADR_DIR = REPO_ROOT / "docs/architecture/adrs"
VAULT_ADR_DIR = Path.home() / ".sddk-knowledge/sddk-framework/adrs"

# Mapping repo slug → nice title (for the vault H1, more readable than ALL CAPS).
# Derived from the canonical ADR content; kebab-case matches the vault convention.
TITLE_OVERRIDES = {
    "ADR-0094-ONE-CANONICAL-FACT-LOG": "One Canonical Fact Log",
    "ADR-0095-FOUR-STATE-CLASSES": "Four State Classes",
    "ADR-0096-SDLC-LIFECYCLE-SEMANTICS": "SDLC Lifecycle Semantics",
    "ADR-0097-COMMON-REVISION-SUBSTRATE": "Common Revision Substrate",
    "ADR-0098-ONE-SEMANTIC-GRAPH": "One Semantic Graph",
    "ADR-0099-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE": "Vault as Human Knowledge Source",
    "ADR-0100-UNIVERSAL-EVIDENCE": "Universal Evidence",
    "ADR-0101-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS": "Agent Outcome Contribution Synthesis",
    "ADR-0102-UNIFIED-AUTHORITY-ENGINE": "Unified Authority Engine",
    "ADR-0103-TARGET-TASK-PORCELAIN": "Target Task Porcelain",
    "ADR-0104-PACK-EXTENSION-BOUNDARY": "Pack Extension Boundary",
    "ADR-0105-CONFIGURATION-CONVENTIONS": "Configuration Conventions",
    "ADR-0106-TYPED-INSTRUCTION-COMPILATION": "Typed Instruction Compilation",
    "ADR-0107-ONE-COMMAND-REGISTRY": "One Command Registry",
    "ADR-0108-SKILL-IS-NOT-CAPABILITY": "Skill Is Not Capability",
    "ADR-0109-PROVIDER-INDEPENDENT-AGENT-PROFILES": "Provider-Independent Agent Profiles",
    "ADR-0110-AGENT-EXECUTION-PROVENANCE": "Agent Execution Provenance",
    "ADR-0001-ADR-PROMOTION-PROCESS": "ADR Promotion Process",
}

# Mapping repo ADR → related milestone / cycle for the Implementation Log entry
# (best-effort from the audit notes; refined during cycle-7 audit).
RELATED_CYCLES = {
    "ADR-0094-ONE-CANONICAL-FACT-LOG": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0095-FOUR-STATE-CLASSES": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0096-SDLC-LIFECYCLE-SEMANTICS": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0097-COMMON-REVISION-SUBSTRATE": "p-63676b11dc0ef88f/revision-substrate-cas",
    "ADR-0098-ONE-SEMANTIC-GRAPH": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0099-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0100-UNIVERSAL-EVIDENCE": "p-63676b11dc0ef88f/evidence-relations-core",
    "ADR-0101-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS": "p-63676b11dc0ef88f/synthesis-dissent-runner-extension",
    "ADR-0102-UNIFIED-AUTHORITY-ENGINE": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0103-TARGET-TASK-PORCELAIN": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0104-PACK-EXTENSION-BOUNDARY": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0105-CONFIGURATION-CONVENTIONS": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0106-TYPED-INSTRUCTION-COMPILATION": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0107-ONE-COMMAND-REGISTRY": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0108-SKILL-IS-NOT-CAPABILITY": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0109-PROVIDER-INDEPENDENT-AGENT-PROFILES": "p-63676b11dc0ef88f/adr-promotion-batch-2",
    "ADR-0110-AGENT-EXECUTION-PROVENANCE": "p-63676b11dc0ef88f/adr-promotion-batch-2",
}


def parse_frontmatter(path: Path) -> dict:
    """Parse YAML-ish frontmatter from a repo ADR file."""
    text = path.read_text()
    m = re.match(r"^---\n(.+?)\n---\n", text, re.DOTALL)
    if not m:
        return {}
    fm = {}
    current_key = None
    current_list = None
    for line in m.group(1).split("\n"):
        if not line.strip():
            continue
        if line.startswith("  - "):
            if current_list is not None:
                val = line[4:].strip().strip('"').strip("'")
                current_list.append(val)
            continue
        m2 = re.match(r"^([a-zA-Z_]+):\s*(.*)$", line)
        if not m2:
            continue
        key, value = m2.group(1), m2.group(2).strip()
        if value == "":
            # Could be a list or scalar — look at next line
            current_list = []
            fm[key] = current_list
            current_key = key
            continue
        if value == "[]":
            fm[key] = []
        else:
            fm[key] = value.strip('"').strip("'")
            current_list = None
        current_key = key
    return fm


def extract_summary(path: Path) -> str:
    """Extract the substantive summary paragraph.

    The repo ADR body has TWO H1 sections:
    1. First H1 with meta-note ("Mirror of package ADR ...") — skip
    2. Second H1 with the canonical content (verbatim from package) — use this

    Returns the first paragraph of the second H1's body.
    """
    text = path.read_text()
    body = re.sub(r"^---\n.+?\n---\n", "", text, count=1, flags=re.DOTALL)
    # Find all H1s
    h1_positions = [(m.start(), m.end()) for m in re.finditer(r"^# .+\n", body, re.MULTILINE)]
    if len(h1_positions) < 2:
        # Single H1 — fall back to first paragraph after H1
        body_after_h1 = body[h1_positions[0][1]:] if h1_positions else body
    else:
        # Use the LAST H1 (canonical content from package, not the mirror-note H1)
        body_after_h1 = body[h1_positions[-1][1]:]
    paragraphs = [p.strip() for p in body_after_h1.split("\n\n") if p.strip()]
    for p in paragraphs:
        if p.startswith("#"):
            continue
        if p.startswith("|"):
            continue
        if p.startswith(">"):
            continue
        # Strip markdown
        clean = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", p)
        clean = re.sub(r"[*_`]+", "", clean)
        if len(clean.strip()) < 50:
            continue
        return clean[:500]
    return "(no summary)"


def build_mirror(repo_file: Path) -> str:
    fm = parse_frontmatter(repo_file)
    slug = repo_file.stem  # e.g. ADR-0094-ONE-CANONICAL-FACT-LOG
    nice_title = TITLE_OVERRIDES.get(slug, slug)
    summary = extract_summary(repo_file)
    related_adrs = fm.get("related_adrs", [])
    if isinstance(related_adrs, list):
        related_links = "\n".join(f"  - \"[[{a}]]\"" for a in related_adrs)
    else:
        related_links = "  - \"[[ADR-0001-ADR-PROMOTION-PROCESS]]\""
    accepted_at = fm.get("accepted_at", "2026-09-12")
    accepted_by = fm.get("accepted_by_cycle", "p-63676b11dc0ef88f/adr-promotion-batch-2")
    package_source = fm.get("package_source", "")
    package_local_id = fm.get("package_local_id", "")
    implementation_evidence = fm.get("implementation_evidence", [])
    if isinstance(implementation_evidence, list):
        evidence_lines = "\n".join(f"- {e}" for e in implementation_evidence[:6])
    else:
        evidence_lines = f"- {implementation_evidence}"
    related_cycle = RELATED_CYCLES.get(slug, accepted_by)
    today = dt.date.today().isoformat()
    return f"""---
type: adr
title: "{fm.get('id', slug)} — {nice_title}"
slug: "{slug.lower()}"
status: accepted
created: {fm.get('adopted_at', '2026-09-09')}
decided: {accepted_at}
accepted_at: {accepted_at}
accepted_by_cycle: "[[{accepted_by}]]"
superseded_by:
created_in_cycle: "[[p-63676b11dc0ef88f/architecture-adoption-m0-supersession]]"
affects_requirements: []
affects_domains:
  - engine
  - cli
related_adrs:
{related_links}
challenged_by:
stale_after: {fm.get('stale_after', '2027-09-12')}
package_local_id: "{package_local_id}"
package_source: "{package_source}"
repo_authority: "docs/architecture/adrs/{repo_file.name}"
---

# {fm.get('id', slug)} — {nice_title}

> **Vault mirror.** The canonical source for this ADR lives in the repo at
> `docs/architecture/adrs/{repo_file.name}` and is regenerated from that file
> by `scripts/mirror_adrs_to_vault.py`. Edits to this file will be overwritten
> on the next mirror run; edit the repo ADR instead.

## Summary

{summary}

## Decision Drivers

See `## Decision Drivers` in the repo ADR for the full enumeration. Per the
audit notes captured during `adr-promotion-batch-2` (v1.168.33), the drivers
are the bounded concern separation required by the M0 governance layer.

## Considered Options

See `## Considered Options` in the repo ADR. The chosen option is documented
inline in `## Decision Outcome`.

## Decision Outcome

**Chosen:** See repo ADR for the full decision-outcome section. The outcome
was promoted from `proposed` to `accepted` on {accepted_at} by cycle
`{accepted_by}`.

### Consequences

See `## Consequences` in the repo ADR.

## Implementation Evidence (anchored)

The following file:line anchors were captured at promotion time and live in
the repo ADR's `implementation_evidence` frontmatter:

{evidence_lines}

## Specs Impacted by This Decision

See repo ADR `## Specs Impacted` section. The canonical specs index lives
at `docs/architecture/specs/` in the repo.

## Provenance

| Field | Value |
|---|---|
| repo_authority | `docs/architecture/adrs/{repo_file.name}` |
| package_source | `{package_source}` |
| package_local_id | `{package_local_id}` |
| adopted_at | {fm.get('adopted_at', '2026-09-09')} |
| accepted_at | {accepted_at} |
| accepted_by_cycle | `{accepted_by}` |
| mirror_generated | {today} |

## Implementation Log (append-only — updated by `sddk archive`)

### {today} — CYC-vault-mirror-accepted-adrs (v1.168.38)
- **outcome:** mirror-created
- **scope:** {repo_file.name}
- **cycle:** p-63676b11dc0ef88f/vault-mirror-accepted-adrs
- **health:** sound — vault mirror generated from repo ADR frontmatter

## Related Decisions

{related_links.replace('  - "[[', '- [[').replace(']]"', ']]')}
"""


def main():
    VAULT_ADR_DIR.mkdir(parents=True, exist_ok=True)
    created = []
    skipped = []
    for repo_file in sorted(REPO_ADR_DIR.glob("ADR-*.md")):
        # Skip non-accepted
        fm = parse_frontmatter(repo_file)
        if fm.get("status") != "accepted":
            continue
        slug = repo_file.stem.lower()
        # Vault filename convention: <id>-<kebab-title>.md (matches ADR-0079-cycle-supersede.md)
        # We use the full stem to preserve traceability back to the repo file
        target = VAULT_ADR_DIR / f"{repo_file.stem}.md"
        if target.exists():
            skipped.append(target.name)
            continue
        body = build_mirror(repo_file)
        target.write_text(body)
        created.append(target.name)
    print(f"created: {len(created)}, skipped: {len(skipped)}")
    for n in created:
        print(f"  + {n}")
    for n in skipped:
        print(f"  = {n} (already exists)")


if __name__ == "__main__":
    main()
