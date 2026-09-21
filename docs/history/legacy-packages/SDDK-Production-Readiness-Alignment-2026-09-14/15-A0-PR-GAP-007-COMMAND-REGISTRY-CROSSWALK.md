# A0 — PR-GAP-007 Command Registry crosswalk

**Audit basis:** `main@3b8c7b6`.
**Requirement:** eliminate authoritative handwritten duplication between the CLI command model and `CommandSpec`; help, examples, agent surface and allowed-command sets derive from one typed substrate.

## One typed substrate: `CommandSpec`

`crates/sddk-cli/src/command_spec.rs::all_command_specs()` is the typed substrate. Every agent-facing derived surface consumes it:

| Derived surface | Derivation | Evidence |
|---|---|---|
| Agent introspection / typed help | `sddk introspect commands` → `run_introspect` → `render_command_spec` (no runtime clap parsing) | `command_spec.rs::render_command_spec`, `run_introspect` |
| Agent command surface | `command_surface::surface_with_filter` → `all_command_specs()` | `command_surface.rs:95`, `agent_surface_golden::current` |
| Examples | `ExampleSpec` carried by each `CommandSpec`; walked against the runtime by the M7.1B walker | `m7_1.examples_published_for_core_commands`, `m7_1b.examples_walked_against_runtime` |
| Allowed-command sets | `skill_definition::SkillRegistry::admits_command(&CommandSpec)` and `agent_profile::AgentProfile::admits(&CommandSpec)` | `skill_definition.rs:491`, `agent_profile.rs:66` |

## The only remaining source: the clap runtime parser

`crates/sddk-cli/src/lib.rs` defines the clap `Command` enum (`derive(Parser)`) that actually parses argv. It is a distinct responsibility (typed argument parsing) from the agent-facing metadata substrate. Its linkage to `CommandSpec` is mandatory and guarded:

- `crates/sddk-cli/tests/command_spec_tests.rs::clap_surface_and_command_specs_are_in_sync` fails the build if any top-level clap command lacks a `CommandSpec`, or any `CommandSpec` names a non-existent command path.
- `agent_surface_golden::matches_checked_in_fixture` locks the derived surface shape.

## Disposition: `PASS_WITH_COMPAT`

- **Canonical authority after change:** `CommandSpec` is the single typed source for all agent-facing surfaces (introspection/typed help, agent surface, examples, allowed-command sets).
- **Compatibility surface:** the clap `Command` enum remains the runtime argv parser; the hand-curated spec table is coupled to it by the mandated drift test, not by silent duplication.
  - **scope:** CLI argv parsing only (not agent-facing metadata authority)
  - **owner:** `sddk-cli` maintainers
  - **fixture / detection guard:** `clap_surface_and_command_specs_are_in_sync`; `agent_surface_golden::matches_checked_in_fixture`
  - **removal trigger:** when the typed registry is generated from clap metadata (or a runtime clap-introspection pass replaces the hand-curated table), the drift test becomes redundant and is removed with the table.
- **No writable/bypass authority is duplicated.** This is a metadata/parse coupling, not a second governed-effect path.
