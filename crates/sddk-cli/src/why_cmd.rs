//! `sddk why architecture <CONTRACT_OR_FINDING>` — READ / EXPLAIN.
//!
//! Answers "why does SDDK claim this about the architecture?" by traversing
//! provenance AC1–AC5 already computed. It runs no second audit, evaluates
//! nothing, and mutates nothing.
//!
//! # Why this is not `graph why`
//!
//! `sddk graph why` traverses the **event-ledger** reactive graph (runtime facts:
//! capabilities, cycles, runs) and returns a node plus its incident edges. The
//! architecture substrate is a different projection — the AC2 overlay is rebuilt
//! from the declaration on every run (ADR-0120) — so it is not in that graph and
//! cannot be reached from it. Two substrates, two surfaces, no alias: this module
//! does **not** add `sddk why graph`, and `sddk architecture why` is not created
//! either (one question, one canonical surface).

use std::path::PathBuf;

use clap::Args;

use sddk_engine::architecture_debverify::FindingId;
use sddk_engine::architecture_why::{
    ArchitectureWhy, WhyInput, WhyResolvedAs, explain as explain_why,
};

use crate::architecture_cmd::{
    ArchitectureContext, DEFAULT_DECLARATION, build_context, error_output, resolve_now,
};
use crate::{CommandOutput, OutputFormat};

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WhyCommand {
    /// Explain why SDDK claims something about the architecture.
    Architecture(WhyArchitectureArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct WhyArchitectureArgs {
    /// A declared contract id, or a finding id printed by `architecture findings`.
    pub(crate) query: String,
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Declaration file, relative to `--root`.
    #[arg(long, default_value = DEFAULT_DECLARATION)]
    pub(crate) contracts: PathBuf,
    /// Evaluation time in epoch-ms. Defaults to the current wall clock.
    ///
    /// Finding ids are derived from a clock-stable basis, so an id printed by
    /// `architecture findings` remains resolvable here at any clock.
    #[arg(long)]
    pub(crate) now_ms: Option<i64>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Dispatch the `why` command family.
pub(crate) fn run_why(command: WhyCommand, _environment: &crate::CliEnvironment) -> CommandOutput {
    match command {
        WhyCommand::Architecture(args) => run_architecture(args),
    }
}

/// Resolve the argument against the two namespaces, explicitly and fail-closed.
///
/// Deliberately **not** shape-based. A declaration may legally carry a contract id
/// that is 64 hex, so "looks like an id" cannot decide a namespace; the two
/// namespaces are consulted and a collision is reported rather than resolved.
fn resolve(
    ctx: &ArchitectureContext,
    query: &str,
) -> Result<(WhyResolvedAs, Option<usize>), CommandOutput> {
    let as_contract = ctx
        .declared
        .contracts
        .iter()
        .any(|c| c.id().as_str() == query);
    let as_finding = ctx.audit.findings.iter().position(|f| {
        FindingId::derive(&ctx.finding_basis, f.kind, &f.subjects, &f.contract_ids).as_str()
            == query
    });

    match (as_contract, as_finding) {
        (true, None) => Ok((WhyResolvedAs::Contract, None)),
        (false, Some(i)) => Ok((WhyResolvedAs::Finding, Some(i))),
        (true, Some(_)) => Err(error_output(format!(
            "why architecture: `{query}` is ambiguous — it is both a declared contract id and a \
             finding id in the current audit. Pass a distinct id, or rename the contract: the two \
             namespaces are not silently ordered."
        ))),
        (false, None) => Err(error_output(format!(
            "why architecture: `{query}` is unknown. Tried the declared contract ids in `{}` and \
             every finding id in the current audit; neither matched.",
            ctx.declared.knowledge_basis
        ))),
    }
}

fn run_architecture(args: WhyArchitectureArgs) -> CommandOutput {
    let ctx = match build_context(&args.root, &args.contracts, resolve_now(args.now_ms)) {
        Ok(c) => c,
        Err(o) => return o,
    };

    let (resolved_as, finding_index) = match resolve(&ctx, &args.query) {
        Ok(r) => r,
        Err(o) => return o,
    };
    // The id as a string, so the traversal borrows it instead of re-deriving.
    let query_id = args.query.clone();

    let answer: ArchitectureWhy = explain_why(WhyInput {
        query: &args.query,
        resolved_as,
        basis: &ctx.finding_basis,
        contracts: &ctx.declared.contracts,
        claims: &ctx.claims,
        audit: &ctx.audit,
        overlay: &ctx.overlay,
        finding: finding_index.map(|i| (query_id.as_str(), i)),
        // Observations the declaration supplies (A4-0b). When it declares none the
        // leg stays unresolved and the answer says so, rather than pretending the
        // leg is structurally impossible.
        observations: Some(&ctx.observations),
    });

    match args.format {
        OutputFormat::Json => match serde_json::to_string_pretty(&answer) {
            Ok(body) => CommandOutput {
                status: 0,
                stdout: format!("{body}\n"),
                stderr: String::new(),
            },
            Err(e) => error_output(format!("why architecture: cannot serialise: {e}")),
        },
        OutputFormat::Text => CommandOutput {
            status: 0,
            stdout: render_text(&answer),
            stderr: String::new(),
        },
    }
}

/// Causal, readable rendering. Same facts as the JSON, no verdict, no score.
fn render_text(a: &ArchitectureWhy) -> String {
    let mut out = String::new();
    out.push_str("ARCHITECTURE-WHY\n");
    out.push_str(&format!("query:             {}\n", a.query));
    out.push_str(&format!(
        "resolved_as:       {}\n",
        a.resolved_as.canonical_tag()
    ));
    out.push_str(&format!(
        "basis:             revision={} knowledge={} contracts_sha256={}\n",
        a.basis.revision, a.basis.knowledge_basis, a.basis.finding_basis_digest
    ));

    if let Some(f) = &a.finding {
        out.push_str("\nfinding:\n");
        out.push_str(&format!("  id:              {}\n", f.id));
        out.push_str(&format!("  kind:            {}\n", f.kind));
        out.push_str(&format!("  severity:        {}\n", f.severity));
        out.push_str(&format!("  subjects:        {}\n", f.subjects.join(", ")));
        out.push_str(&format!(
            "  contracts:       {}\n",
            f.contract_ids.join(", ")
        ));
    }

    out.push_str(&format!("\ncontracts ({}):\n", a.contracts.len()));
    if a.contracts.is_empty() {
        out.push_str("  (none)\n");
    }
    for c in &a.contracts {
        out.push_str(&format!("  {}\n", c.contract));
        out.push_str(&format!("    kind:          {}\n", c.kind));
        out.push_str(&format!("    subject:       {}\n", c.subject));
        out.push_str(&format!("    revision:      {}\n", c.revision));
        out.push_str(&format!("    participates:  {}\n", c.participates_because));
        out.push_str(&format!(
            "    assessment:    {}{}\n",
            c.assessment.outcome,
            if c.assessment.evidence_present {
                " (evidence supplied)"
            } else {
                " (no evidence supplied)"
            }
        ));
        if !c.assessment.missing_evidence.is_empty() {
            out.push_str(&format!(
                "    missing:       {}\n",
                c.assessment.missing_evidence.join(", ")
            ));
        }
        out.push_str(&format!(
            "    decided_by:    {}\n",
            c.intent.decisions.join(", ")
        ));
        out.push_str(&format!(
            "    specified_by:  {}\n",
            c.intent.specs.join(", ")
        ));
        if c.evidence.is_empty() {
            out.push_str("    evidence:      (none supplied)\n");
        } else {
            for e in &c.evidence {
                out.push_str(&format!(
                    "    evidence:      provider={} reference={}\n",
                    e.provider, e.reference
                ));
            }
        }
        out.push_str(&format!(
            "    software:      {}\n",
            if c.software_units.is_empty() {
                "(none reachable)".to_string()
            } else {
                c.software_units.join(", ")
            }
        ));
    }

    out.push_str(&format!(
        "\nunresolved_edges ({}):\n",
        a.unresolved_edges.len()
    ));
    for u in &a.unresolved_edges {
        out.push_str(&format!("  {}\n    {}\n", u.edge, u.reason));
    }

    out.push_str(&format!("\nwhy_not ({}):\n", a.why_not.len()));
    for w in &a.why_not {
        out.push_str(&format!("  {w:?}\n"));
    }
    out
}
