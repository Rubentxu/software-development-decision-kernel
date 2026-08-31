# SDDK Framework

> **Software Development Decision Kernel** — an agentic decision and workflow kernel with a built-in knowledge graph, governed Git effects, and evidence-based verification.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![OKF Compatible](https://img.shields.io/badge/OKF-v0.2-blue.svg)](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
[![Obsidian Compatible](https://img.shields.io/badge/Obsidian-Properties_v1.4+-purple.svg)](https://obsidian.md/)

**[English](README.md)** | [Español](README.es.md)

---

## What is SDDK?

SDDK is a complete agent orchestration framework for AI-assisted software development. It coordinates AI agents through a structured pipeline — from exploration to release — with built-in quality gates, technical debt auditing, and a knowledge graph that tracks every decision, requirement, and incidence across cycles.

### Key differentiators

| Feature | What it does |
|---------|-------------|
| **Decision-governed** | Context and risk select the workflow path; explicit evidence and receipts govern every handoff. Specifications remain one acceptance artifact, not the product identity. |
| **Multi-lens verification** | 6 parallel verification lenses (spec compliance, architecture, test quality, design coherence, 2 adversarial judges) + synthesis. |
| **Technical debt audit** | 5 cluster agents (architecture, smells, duplication, coupling, over-engineering) audit debt before merge to main. |
| **Knowledge graph** | Every milestone, ADR, requirement, cycle, and incidence is a node in an Obsidian-compatible wikilink graph. Full bidirectional traceability. |
| **Trunk-based guarantee** | A cycle cannot declare `success` until changes are merged to `main` + semver tagged + trunk synced. No silent aborts. |
| **Serialization lock** | Only one cycle at a time. The lock survives session crashes. |
| **Editor-agnostic** | Works with ZCode and OpenCode (extensible to any agent runner). |

## Architecture

```
┌─────────────────────────────────────────────────────┐
│   ~/Proyectos/agentesIA/sddk-framework/    │
│              (this repository — framework)           │
│                                                      │
│  ┌──────────┐  ┌─────────┐  ┌────────────────────┐  │
│  │  agents/  │  │ skills/ │  │   prompts/sddk    │  │
│  │ (prompts) │  │ (tools)  │  │ (phase specs, MCW) │  │
│  └────┬─────┘  └────┬────┘  └─────────┬──────────┘  │
│       │              │                  │             │
│  ┌────┴──────────────┴──────────────────┴──────────┐ │
│  │         knowledge-template/ (vault template)     │ │
│  │  milestones · adrs · specs · cycles · incidences │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─────────────┐  ┌──────────────────┐               │
│  │golden-dataset│  │ bootstrap.sh     │               │
│  │(meta-testing)│  │ (installer)      │               │
│  └─────────────┘  └──────────────────┘               │
└─────────────────────────────────────────────────────┘
         │                                    │
    ┌────┴────┐                         ┌─────┴─────┐
    │ ZCode   │                         │ OpenCode  │
    │(symlinks)│                        │(symlinks) │
    └─────────┘                         └───────────┘
         │
    ┌────┴──────────────┐
    │ ~/.sddk-knowledge/{project}/ │  (per-project vault,
    │       (committed to git)    │   created by sddk-adopt)
    └───────────────────┘
```

The current agent and skill paths are tracked in the [generated repository inventory](docs/generated/inventory.md).

## Quick start

### Install (users — one-liner)

Since v1.28.0 SDDK ships pre-compiled binaries on GitHub Releases. Install
with a single command (rustup / mise model):

```bash
curl -fsSL https://raw.githubusercontent.com/Rubentxu/software-development-decision-kernel/main/scripts/install.sh | bash
```

The script auto-detects your platform (Linux x86_64 / Linux aarch64, macOS,
Windows — see [Supported platforms](#supported-platforms)), downloads the
binary + SHA256, verifies integrity, asks which editor to configure
(opencode/zcode/claude/codex), and links the framework bundle into your
`~/.config/<editor>/` directory.

### Install (developers — from source)

If you are modifying the framework itself, build and install from the repo:

```bash
# 1. Clone the framework (single source of truth — never edit the runtime copy directly)
git clone https://github.com/Rubentxu/software-development-decision-kernel.git ~/Proyectos/agentesIA/sddk-framework

# 2. Install the runtime binary with atomic install + receipt
#    (uses the source repo as bundle; verify receipt after)
cd ~/Proyectos/agentesIA/sddk-framework
cargo build --release -p sddk-cli
sddk dev install --prefix ~/.local --source .
sddk dev verify --prefix ~/.local
# Expected: valid: true, version: <x.y.z>

# 3. Link the framework into every detected editor (agents + skills + prompts + workflows)
sddk dev link --editor all

# 4. Diagnose the toolchain + framework layout
sddk dev doctor
# Expected: all_present: true
```

The bootstrap script (`./bootstrap.sh --all`) is an **alternative** to step 3 — it creates the same symlinks but only for content surfaces. Prefer `sddk dev install` + `sddk dev link` because they verify the install with a receipt and a doctor pass.

### Supported platforms

| Platform | Architecture | Status |
|----------|--------------|--------|
| Linux | x86_64 | ✅ musl static (ships in release) |
| Linux | aarch64 | ✅ musl static (ships in release) |
| macOS | x86_64 | ⏳ pending (cargo-zigbuild ready, binaries not yet uploaded) |
| macOS | arm64 | ⏳ pending (cargo-zigbuild ready, binaries not yet uploaded) |
| Windows | x86_64 | ⏳ pending (requires `#[cfg(unix)]` in `dev_cmd.rs`) |

Your project repos stay clean — **zero documentation files in your code repos**. All SDDK state (cycle artifacts, knowledge vault, telemetry) lives under `$XDG_DATA_HOME/sddk/` (`~/.local/share/sddk/` by default).

### Run a cycle

**First time on a project?** Adopt it first:

```bash
cd your-project
/sddk-adopt         # one-time: audit project, plant SDDK artifacts, create knowledge vault
/sddk-init          # one-time: detect stack, testing, TDD mode
/sddk-new add-auth  # start a full SDDK cycle
```

**Subsequent cycles** (project already adopted):

```bash
cd your-project
/sddk-new <change-name>  # the ~/.sddk-knowledge/{project}/ vault is already there; init is skipped
```

The `~/.sddk-knowledge/{project}/` directory is the adoption marker — its existence means the project is adopted. `sddk-init` checks it with a single `test -d`.

The orchestrator will:
1. **Plan** — explore → propose → spec → design → tasks (interactive checkpoints)
2. **Build** — apply (with Strict TDD if enabled) → verify (multi-lens) → path-derived debt-verify
3. **Release and archive** — publish main → semver tag → receipts → archive manifest → sync trunk

No cycle closes until your code is on `main`.

## Workflow paths

| Path | When | Depth |
|------|------|-------|
| **B-direct** | Hotfix, bounded task | Load skill → execute → light verify → release → archive |
| **A-min** | Simple change, C2 context | spec → apply → verify → debt-verify (smoke, 2 clusters) → release → archive |
| **A-lite** | Bounded work, C1 context | propose → spec → apply → verify → debt-verify (standard, 4 clusters) → release → archive |
| **A-full** | Architectural, new domain, C0 | explore → propose → spec ∥ design → tasks → apply → verify (6 lenses) → debt-verify (deep, 5 clusters) → release → archive |

Debt-verify depth is fixed when triage selects the path. Reversibility affects
that initial path decision, not whether the gate runs after implementation.
This trades some analyzer cost for a predictable, non-bypassable local gate;
incomplete required coverage returns `INCONCLUSIVE` and blocks release.

## Knowledge graph

Every cycle populates a knowledge vault at `~/.sddk-knowledge/{project}/` (in user home, outside the repo):

```
my-app/~/.sddk-knowledge/{project}/
├── _index.md              ← MOC with Dataview queries
├── milestones/
│   ├── _active.md         ← serialization lock
│   └── M-001-auth.md      ← [[ADR-003]], [[REQ-Session]]
├── adrs/
│   └── ADR-003-jwt.md     ← [[REQ-Session]], implementation log
├── specs/auth/
│   └── REQ-Session.md     ← [[ADR-003]], tested_by, verified_in_cycle
├── cycles/
│   └── CYC-2026-08-03.md  ← traceability hub (links everything)
├── incidences/
│   └── INC-001-lag.md     ← [[ADR-003]], affects [[REQ-Session]]
└── terms/
    └── TERM-JWT.md
```

Open it in [Obsidian](https://obsidian.md) for graph view, backlinks, and Dataview queries. Based on [Google's OKF spec](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) with bi-temporal changelogs.

## Verification system

### Functional verification (`sddk-verify`)

The **Behavioral Compliance Matrix** maps every spec scenario to a test that passed at runtime. Static analysis alone is never verification.

| Lens | What it checks |
|------|---------------|
| Spec Compliance | Every scenario → covering test → runtime PASS |
| Architecture + Connascence | Design quality, coupling, SOLID |
| Test Quality | Banned assertions, mock ratios, triangulation |
| Design Coherence | Design decisions vs implementation |
| Adversarial Judge A | Blind deficiency detection |
| Adversarial Judge B | Blind deficiency detection |

### Technical debt audit (`sddk-debt-verify`)

Up to 5 cluster agents run in parallel, depending on the selected path. They
write machine-authoritative `debt-report.json` plus a derived
`debt-report.md`; the current CLI handoff remains specification-only.

| Cluster | Dimension |
|---------|-----------|
| Architecture | Connascence, SOLID, Matsumoto + Khononov critiques |
| Smells | 12 Fowler smells with grep-verifiable signals → SOLID mapping |
| Duplication | Structural/literal/semantic + dead code |
| Coupling | Hidden deps, global state, circular imports |
| Over-engineering | YAGNI, ponytail debt ledger, bloat trajectory |

## Project structure

```
sddk-framework/
├── agents/                 # Agent prompts; see docs/generated/inventory.md
├── skills/                 # Skills; see docs/generated/inventory.md
├── prompts/sddk/            # Phase specs, MCW, git-contract, decision-model, ADR/roadmap templates
│   └── workflows/          # Path-specific workflow YAML (A-full/A-lite/A-min/B-direct)
├── workflows/              # Top-level workflow trees (e.g. sddk-b-research)
├── knowledge-template/     # Vault template (6 node types, MOCs, serialization lock)
├── golden-dataset/         # Meta-verification test cases (5 initial cases + runner)
├── assets/                 # Runtime assets (UAT drivers, dashboard kit, MCP overrides)
├── packs/                  # Declarative pack manifests (sddk-pack-uat, etc.)
├── crates/                 # sddk-cli, sddk-domain, sddk-gateway, ...
├── tests/                  # Integration + golden tests
├── docs/                   # ADRs, history, handoff notes, generated inventory
├── bootstrap.sh            # Legacy installer for ZCode/OpenCode content surfaces
├── README.md               # This file
├── README.es.md            # Spanish documentation
├── CONTRIBUTING.md         # Contribution guide (commit conventions, review process)
└── LICENSE                 # MIT
```

## Key concepts

- **MCW (Mandatory Complete Workflow)** — the law. 5 phases, numbered steps, hard gates. Source of truth: `prompts/sddk/mcw.md`.
- **Serialization Lock** — one cycle at a time. Lock file: `milestones/_active.md`. Survives session crashes.
- **Release Completion Guard** — the orchestrator cannot emit `status: success` without `HEAD == origin/main` + semver tag confirmed on remote.
- **Zero docs in repo** — all project knowledge lives in the vault, never in the project's git repo.
- **Bi-temporal changelog** — every node tracks `valid_from` / `valid_to`, enabling time-travel queries.

## Compatibility

- **Editors**: ZCode, OpenCode (extensible to any agent runner that reads markdown prompts)
- **Knowledge format**: [OKF v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md), [Obsidian Properties v1.4+](https://obsidian.md/)
- **MCPs** (optional): CogniCode (architecture analysis), Chronos (time-travel debugging), Engram (cross-session memory)

## Contributing

Contributions are welcome. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) for the workflow (commit conventions, review process, CI policy). The architecture lives in `prompts/sddk/mcw.md` — read it before proposing changes to phase agents or the orchestrator.

## License

[MIT](LICENSE) © 2026 Rubentxu
