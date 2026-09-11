# AX-S5 — Agent asset static scanner

**Status:** COMPLETED 2026-09-11 (v1.168.18)
**Question (SPIKES.md):** can we detect deprecated commands, raw store/table references, authority language, duplicated prompt fragments and unregistered examples in Markdown/YAML/Rust literals — with a false-positive rate low enough to keep advisory?
**Harness:** `crates/sddk-cli/src/spike_axs5.rs` (6 pinned tests, pure scanners over in-memory content).

## Method

Five deterministic detectors, each returning file/line/snippet findings:

1. `deprecated_name` — legacy namespace spellings (`SDD-kernel` etc., AGENTS.md §2.0).
2. `raw_store_reference` — SQL/store markers in Markdown prose (vault/runtime separation).
3. `authority_language` — capability-bypassing phrases (SPEC-013..018), case-insensitive.
4. `duplicated_fragment` — identical 4+ normalized-line blocks across files.
5. `unregistered_example` — `sddk <cmd>` prose mentions not in the canonical command list (mirrors `command_spec`, which is pinned against clap by the AX-S1 guard).

## Measurements on the real corpus (91 files: `agents/*.md` + `prompts/sddk/*.md`)

| Rule | Hits | Verdict |
|---|---|---|
| deprecated_name | **0** | clean — normalization complete |
| raw_store_reference | **0** | clean |
| authority_language | **0** | clean |
| unregistered_example | **0 false** | all 17 distinct `sddk <cmd>` tokens resolve to spec'd commands (measured via `command_spec`) |
| duplicated_fragment | **20 windows / ~14 pairs** | **real signal, low severity** — all are shared boilerplate: the Common Finding Contract envelope duplicated verbatim across the 5 `debt-*-cluster` agents (4 shared windows each), and 5 windows shared by `prompts/sddk/mcw.md` ↔ `metrics-schema.md`; plus 3-window echo between `sddk-explore`/`sddk-tasks` |

**False-positive rate: 0/5 rules on this corpus.** All five detectors are advisory-clean today.

## Findings

1. **Scanner is viable and immediately clean** — the corpus has zero hits for rules 1–3 and 5. Their value is *regression prevention*, not current cleanup. Promoting them to `sddk dev lint` as advisory checks is cheap and useful now.
2. **The only true positive is structural duplication**: the receipt/Common-Finding envelope is copy-pasted across the 5 debt clusters and the metrics schema. That is a known M-pack consolidation target, not a scanner problem — the correct fix is extracting shared envelope prose to a single included source, not suppressing the detector.
3. **Command-truth indirection works**: rule 5 delegates canonicality to `command_spec` (already clap-pinned since AX-S1), so the scanner needs no independent command registry. This is the single-source pattern of AGENTS.md §2.8 applied.
4. **Recommended disposition**: keep the harness as a spike; promote rules 1–3+5 to a `deprecated-patterns`-adjacent advisory lint in a small follow-up cycle, and treat rule 4 as a pointing device for the debt-cluster envelope consolidation (feed its output into a debt ledger entry rather than enforcing).

## Revisit triggers

- ~~Promoting the scanner into `sddk dev lint`~~ **DONE v1.168.20**: rules 1-3+5 promoted as advisory lints `asset_deprecated_namespace`, `asset_raw_store_reference`, `asset_authority_language`, `asset_unregistered_cli_example` in `docs/architecture/lints/deprecated_patterns.toml` (regex engine has no lookahead: rule 5 uses a first-letter sieve, confidence medium; full-surface validation stays in the command_spec clap guard). Guard test `live_registry_asset_lints_are_advisory_and_clean` pins advisory + zero hits. Rule 4 (duplicated fragments) remains a spike-only pointing device; the envelope half was consolidated in v1.168.19.
- Debt-cluster envelope consolidation cycle (rule 4 output is the worklist).
