//! Architecture conformance commands (arch-spec-A3-S10, 12-CLI-AGENT-UX).
//!
//! `sddk architecture receipt` loads a declarative architecture description,
//! runs the native conformance capabilities and emits the named
//! `ARCHITECTURE-CONFORMANCE-RECEIPT`.
//!
//! Read-only: the handler reads one file under the resolved root and prints.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::{CliEnvironment, CommandOutput, OutputFormat};
use sddk_engine::architecture_conformance::{
    ConformanceInputs, ContractEvidence, compute_conformance_delta,
};
use sddk_engine::architecture_debverify::run_debverify_audit;
use sddk_engine::architecture_declaration::{DeclarationFile, validate};
use sddk_engine::architecture_graph::ArchitectureGraphOverlay;
use sddk_engine::architecture_mutation::{MutationSandbox, run_mutation_suite};
use sddk_engine::architecture_receipt::{ReceiptInputs, ReceiptVerdict, compose_receipt};
use sddk_engine::knowledge::EventTime;

/// Exit code for a `Blocked` receipt.
const EXIT_BLOCKED: i32 = 1;
/// Exit code for a declaration/IO error.
const EXIT_INPUT_ERROR: i32 = 2;

/// Default declaration location relative to the resolved root.
pub(crate) const DEFAULT_DECLARATION: &str = ".sddk/architecture/contracts.yaml";

#[derive(Debug, Subcommand)]
pub(crate) enum ArchitectureCommand {
    /// Emit the architecture-conformance receipt for a declarative contract set.
    Receipt(ArchitectureReceiptArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ArchitectureReceiptArgs {
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Declaration file, relative to `--root`.
    #[arg(long, default_value = DEFAULT_DECLARATION)]
    pub(crate) contracts: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Dispatch the `architecture` command family.
pub(crate) fn run_architecture(
    command: ArchitectureCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        ArchitectureCommand::Receipt(args) => run_receipt(args, environment),
    }
}

fn run_receipt(args: ArchitectureReceiptArgs, _environment: &CliEnvironment) -> CommandOutput {
    let declaration_path = args.root.join(&args.contracts);
    let label = args.contracts.display().to_string();

    // ── read + parse (CLI owns the format; the engine owns semantics) ──────
    let text = match std::fs::read_to_string(&declaration_path) {
        Ok(t) => t,
        Err(e) => {
            return error_output(format!(
                "architecture receipt: cannot read declaration `{}`: {e}",
                declaration_path.display()
            ));
        }
    };
    let parsed: DeclarationFile = match serde_yaml::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            return error_output(format!(
                "architecture receipt: `{}` is not a valid declaration: {e}",
                declaration_path.display()
            ));
        }
    };

    // ── validate (fail-closed) ─────────────────────────────────────────────
    let declared = match validate(&parsed, &label) {
        Ok(d) => d,
        Err(e) => return error_output(format!("architecture receipt: {e}")),
    };

    // ── AC2 overlay ────────────────────────────────────────────────────────
    let mut overlay = ArchitectureGraphOverlay::new();
    for unit in &declared.units {
        overlay.add_unit(unit);
    }

    // ── AC4: change-scoped delta over an EMPTY change basis ────────────────
    // The receipt is global; the delta contributes its digests and its
    // (empty) claim/witness sets honestly.
    let evidence: ContractEvidence = std::collections::BTreeMap::new();
    let delta = match compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &declared.contracts,
            evidence: &evidence,
            contradiction_witnesses: &[],
        },
        EventTime(0),
        &[],
    ) {
        Ok(d) => d,
        Err(e) => return error_output(format!("architecture receipt: delta failed: {e}")),
    };

    // ── AC5: global audit ──────────────────────────────────────────────────
    let audit = match run_debverify_audit(&overlay, &declared.contracts, EventTime(0)) {
        Ok(a) => a,
        Err(e) => return error_output(format!("architecture receipt: audit failed: {e}")),
    };

    // ── AC6: no mutation sandbox supplied by v1 declarations ───────────────
    // An empty suite is recorded honestly: the receipt reports the
    // negative-evidence class as NOT reproduced rather than pretending.
    let mutations = match run_mutation_suite(&MutationSandbox::new(), &[], &[]) {
        Ok(m) => m,
        Err(e) => return error_output(format!("architecture receipt: mutations failed: {e}")),
    };

    // ── AC8: compose the receipt ───────────────────────────────────────────
    let receipt = compose_receipt(
        ReceiptInputs {
            revision: declared.revision.clone(),
            knowledge_basis: declared.knowledge_basis.clone(),
            delta: &delta,
            audit: &audit,
            mutations: &mutations,
            lenses: &[],
            waivers: &declared.waivers,
            provider_basis: &[],
        },
        EventTime(0),
    );

    let status = match receipt.verdict {
        ReceiptVerdict::Pass | ReceiptVerdict::PassWithWaivers => 0,
        ReceiptVerdict::Blocked => EXIT_BLOCKED,
    };

    let stdout = match args.format {
        OutputFormat::Json => match serde_json::to_string_pretty(&receipt) {
            Ok(s) => format!("{s}\n"),
            Err(e) => return error_output(format!("architecture receipt: cannot serialise: {e}")),
        },
        OutputFormat::Text => render_text(&receipt),
    };

    CommandOutput {
        status,
        stdout,
        stderr: String::new(),
    }
}

fn render_text(
    receipt: &sddk_engine::architecture_receipt::ArchitectureConformanceReceipt,
) -> String {
    let mut out = String::new();
    out.push_str("ARCHITECTURE-CONFORMANCE-RECEIPT\n");
    out.push_str(&format!("receipt_id:        {}\n", receipt.id));
    out.push_str(&format!("revision:          {}\n", receipt.basis.revision));
    out.push_str(&format!(
        "knowledge_basis:   {}\n",
        receipt.basis.knowledge_basis
    ));
    out.push_str(&format!(
        "verdict:           {}\n",
        receipt.verdict.canonical_tag()
    ));
    out.push_str(&format!(
        "audited_contracts: {}\n",
        receipt.audited_contracts
    ));
    out.push_str(&format!(
        "affected_contracts: {} (change-scoped; empty without --changed)\n",
        receipt.claim_results.len()
    ));
    out.push_str(&format!("unknowns:          {}\n", receipt.unknowns.len()));
    out.push_str(&format!(
        "contradictions:    {}\n",
        receipt.contradictions.len()
    ));
    out.push_str(&format!("waivers:           {}\n", receipt.waivers.len()));
    out.push_str("historical_class_coverage:\n");
    for c in &receipt.class_coverage {
        out.push_str(&format!(
            "  {:<26} {}\n",
            c.class.canonical_tag(),
            if c.reproduced {
                "reproduced"
            } else {
                "not_reproduced"
            }
        ));
    }
    let mandatory = receipt.mandatory_unresolved();
    if mandatory.is_empty() {
        out.push_str("unresolved_must_findings: none\n");
    } else {
        out.push_str("unresolved_must_findings:\n");
        for f in mandatory {
            out.push_str(&format!(
                "  {:<24} subjects={:?} waivers={:?}\n",
                f.kind, f.subjects, f.waiver_refs
            ));
        }
    }
    out
}

fn error_output(message: String) -> CommandOutput {
    CommandOutput {
        status: EXIT_INPUT_ERROR,
        stdout: String::new(),
        stderr: format!("{message}\n"),
    }
}
