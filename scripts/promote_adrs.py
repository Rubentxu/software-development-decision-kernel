#!/usr/bin/env python3
"""Promote 13 ADRs to `accepted` per ADR-0001 §3.2; mark ADR-0097 with
`deferred_until` honest disclosure.

Idempotent — running twice has the same effect as once.

Frontmatter discipline: all YAML frontmatter fields MUST be between the
two `---` markers. This script inserts the footer (superseded_by,
related_adrs, stale_after) at the end of the frontmatter block, not at
the end of the file.
"""
import re
import sys
from pathlib import Path

CYCLE_ID = "p-63676b11dc0ef88f/adr-promotion-batch-2"
ACCEPT_DATE = "2026-09-12"
ADR_DIR = Path("docs/architecture/adrs")

PROMOTIONS = [
    ("ADR-0094-ONE-CANONICAL-FACT-LOG.md", [
        "crates/sddk-engine/src/canonical_event_log.rs:222 — pub trait CanonicalEventLog + InMemoryCanonicalEventLog (line 232)",
        "docs/architecture/specs/arch-spec-001-canonical-authority.md",
    ]),
    ("ADR-0095-FOUR-STATE-CLASSES.md", [
        "crates/sddk-engine/src/state_class_lint.rs:16 — pub enum StateClass { Fact, Object, Projection, Ephemeral }",
        "crates/sddk-domain/src/lib.rs — StateClass types",
    ]),
    ("ADR-0098-ONE-SEMANTIC-GRAPH.md", [
        "crates/sddk-engine/src/semantic_graph.rs:29 — pub trait SemanticGraphProjection + InMemorySemanticGraph impl (line 69)",
        "docs/architecture/specs/arch-spec-005-semantic-graph-and-why.md",
    ]),
    ("ADR-0099-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE.md", [
        "crates/sddk-engine/src/vault_boundary.rs — T-04 boundary tests reject authority call-sites (line 181)",
        "crates/sddk-cli/src/context_compiler.rs:65 — vault_knowledge as ContextAdapter (read-only)",
        "crates/sddk-vault/src/parser.rs:33 — parse_vault",
    ]),
    ("ADR-0102-UNIFIED-AUTHORITY-ENGINE.md", [
        "crates/sddk-engine/src/authority_engine.rs:216 — ActionProposal, :264 — AdmissionDecision, runner.rs:32 — AuthorityEngineRunner",
        "docs/architecture/specs/arch-spec-008-authority-and-side-effects.md",
    ]),
    ("ADR-0103-TARGET-TASK-PORCELAIN.md", [
        "crates/sddk-engine/src/target_task/mod.rs:142 — pub struct Task, :173 — pub struct Target",
        "crates/sddk-engine/src/target_task/executor.rs — DagExecutor",
    ]),
    ("ADR-0104-PACK-EXTENSION-BOUNDARY.md", [
        "crates/sddk-engine/src/pack_registry.rs:95 — PackRegistry, generic_pack_contracts.rs:111 — PackManifest",
        "tests enforce packs cannot write storage tables (per registry)",
    ]),
    ("ADR-0105-CONFIGURATION-CONVENTIONS.md", [
        "crates/sddk-cli/src/config_cmd.rs — sddk config explain (M6.1)",
        "crates/sddk-cli/src/dev/arch_lint.rs — precedence enforcement",
    ]),
    ("ADR-0106-TYPED-INSTRUCTION-COMPILATION.md", [
        "crates/sddk-cli/src/instruction_compiler.rs:301 — EffectiveInstructions, :348 — InstructionCompiler",
        "AX-S2 conflict algebra pinned by tests",
    ]),
    ("ADR-0107-ONE-COMMAND-REGISTRY.md", [
        "crates/sddk-cli/src/command_spec.rs:254 — pub struct CommandSpec",
        "AX-S1 drift guard test (clap_surface_and_command_specs_are_in_sync)",
    ]),
    ("ADR-0108-SKILL-IS-NOT-CAPABILITY.md", [
        "crates/sddk-cli/src/skill_definition.rs:55 — SkillDefinition, :116 — CapabilityRequirement",
        "tests enforce Skill != Capability (no grant method)",
    ]),
    ("ADR-0109-PROVIDER-INDEPENDENT-AGENT-PROFILES.md", [
        "crates/sddk-cli/src/agent_profile.rs:45 — AgentProfile",
        "AX-S4 spike confirms profile carries no provider transport data",
    ]),
    ("ADR-0110-AGENT-EXECUTION-PROVENANCE.md", [
        "crates/sddk-cli/src/execution_receipt.rs:156 — AgentExecutionReceipt, :333 — AgentExecutionReceiptBuilder",
        "M7.6 spec pins required provenance fields",
    ]),
]

DEFERRALS = [
    ("ADR-0097-COMMON-REVISION-SUBSTRATE.md",
     "Generic Revision<T> type and CAS-Ref abstraction implemented as cross-domain substrate",
     "CAS primitive (crates/sddk-storage/src/cas_object_store.rs) exists. However, the spec calls for a common Revision<T> = { oid, parents[], payload_ref, provenance, metadata } with Ref = { namespace, name, expected_old?, new_oid } and compare-and-swap updates, implemented as a cross-domain substrate. The shipped revisions are domain-specialized: GraphRevision is u64-only (crates/sddk-engine/src/semantic_graph.rs), PlanRevisionV1 (crates/sddk-domain/src/plan_revision.rs:298) and ExecutionGraphRevision (crates/sddk-domain/src/graph.rs:1241) each carry their own lineage without sharing a generic envelope. Promotion blocked on construction, not on audit. Per ADR-0001 §3.2 criterion 1."),
]

# Pattern: split file into frontmatter + body
FRONTMATTER_RE = re.compile(r'^---\n(.*?)\n---\n(.*)$', re.DOTALL)


def split_frontmatter(text: str) -> tuple[str, str]:
    """Return (frontmatter_content, body). Frontmatter is the YAML between --- markers."""
    m = FRONTMATTER_RE.match(text)
    if not m:
        raise ValueError("file does not have frontmatter delimited by --- markers")
    return m.group(1), m.group(2)


def join_frontmatter(fm: str, body: str) -> str:
    return f"---\n{fm}\n---\n{body}"


def promote(path: Path, evidence: list[str]) -> None:
    text = path.read_text()
    fm, body = split_frontmatter(text)

    # Already promoted?
    if re.search(r'^accepted_at:', fm, re.M):
        print(f"  {path.name}: already promoted, skipped")
        return

    # 1. status: proposed → accepted
    fm = re.sub(r'^status:\s*\S+', 'status: accepted', fm, count=1, flags=re.M)

    # 2. Insert accepted_at + accepted_by_cycle after adoption_cycle.
    # The adoption_cycle line may or may not end with \n depending on whether
    # it is the last field before the closing --- marker; the regex handles both.
    fm = re.sub(
        r'^(adoption_cycle:[^\n]*)\n?',
        rf'\1\naccepted_at: {ACCEPT_DATE}\naccepted_by_cycle: "{CYCLE_ID}"\n',
        fm, count=1, flags=re.M,
    )

    # 3. Insert implementation_evidence before superseded_by (or before next blank)
    impl_block = 'implementation_evidence:\n' + ''.join(f'  - "{line}"\n' for line in evidence) + '\n'
    if 'superseded_by:' in fm:
        fm = re.sub(
            r'^(\s*superseded_by:)',
            impl_block + r'\1',
            fm, count=1, flags=re.M,
        )
    else:
        # Insert before related_adrs, OR at end of frontmatter
        if 'related_adrs:' in fm:
            fm = re.sub(
                r'^(\s*related_adrs:)',
                impl_block + r'\1',
                fm, count=1, flags=re.M,
            )
        else:
            fm += '\n' + impl_block

    # 4. Append footer (superseded_by + related_adrs + stale_after) at end of frontmatter
    footer = (
        '\nsuperseded_by: []\n'
        'related_adrs:\n'
        '  - "ADR-0001-ADR-PROMOTION-PROCESS"\n'
        '  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"\n'
        'stale_after: 2027-09-12\n'
    )
    if 'stale_after:' not in fm:
        fm += footer

    # 5. Body: add status note right after the package note
    body_note = (
        '\n> **Status:** accepted (2026-09-12) per cycle '
        f'`{CYCLE_ID}`. Lifecycle defined by ADR-0001 §3.1.\n'
    )
    if 'ADR-0001 §3.1' not in body:
        # Insert after the first "Mirror of package" quote block
        body = re.sub(
            r"(> \*\*Mirror of package ADR `ADR-\d+`\*\*\.[^\n]*\n)",
            r"\1" + body_note,
            body, count=1,
        )

    path.write_text(join_frontmatter(fm, body))
    print(f"  {path.name}: promoted")


def defer(path: Path, short: str, long_reason: str) -> None:
    text = path.read_text()
    fm, body = split_frontmatter(text)

    if 'deferred_until:' in fm:
        print(f"  {path.name}: already deferred, skipped")
        return

    # Insert deferred_until + deferred_reason after adoption_cycle.
    long_indented = long_reason.replace('\n', '\n  ')
    fm = re.sub(
        r'^(adoption_cycle:[^\n]*)\n?',
        rf'\1\ndeferred_until: "{short}"\ndeferred_reason: |\n  {long_indented}\n',
        fm, count=1, flags=re.M,
    )

    path.write_text(join_frontmatter(fm, body))
    print(f"  {path.name}: deferred")


def main() -> int:
    print(f"=== Promotion batch 2 — cycle {CYCLE_ID} ===\n")
    print("Promotions:")
    for filename, evidence in PROMOTIONS:
        promote(ADR_DIR / filename, evidence)

    print("\nDeferrals (honest disclosure):")
    for filename, short, long_reason in DEFERRALS:
        defer(ADR_DIR / filename, short, long_reason)

    return 0


if __name__ == "__main__":
    sys.exit(main())
