// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_debverify/audit.rs — A3-S7 / AC5 audit runner.
//
// `run_debverify_audit(overlay, contracts, now)` is the global challenge pass.
// It takes NO change basis: that absence is the AC-034-002 exit criterion
// ("DebVerify is not verify --full"), pinned by tests.

use sha2::{Digest, Sha256};

use crate::architectural_contract::ArchitecturalContract;
use crate::architecture_graph::ArchitectureGraphOverlay;
use crate::knowledge::EventTime;

use super::detectors;
use super::types::{DebVerifyAudit, DebVerifyError, DebVerifyFinding};

/// Domain prefix for the audit digest.
pub const AUDIT_DIGEST_DOMAIN: &str = "sddk.architecture_debverify.audit.v1|";

/// Run the global DebVerify pass.
///
/// Global, pure and delta-independent: it inspects **every** supplied contract,
/// not those reachable from a change. Identical `(overlay, contracts, now)`
/// yields an identical audit.
pub fn run_debverify_audit(
    overlay: &ArchitectureGraphOverlay,
    contracts: &[ArchitecturalContract],
    now: EventTime,
) -> Result<DebVerifyAudit, DebVerifyError> {
    let mut findings: Vec<DebVerifyFinding> = Vec::new();
    findings.extend(detectors::shadow_authority(contracts));
    findings.extend(detectors::authority_bypass(contracts, overlay));
    findings.extend(detectors::missing_owner(contracts, overlay));
    findings.extend(detectors::contradiction(contracts));
    findings.extend(detectors::stale_compatibility(contracts, now));

    findings.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.subjects.cmp(&b.subjects))
            .then_with(|| a.contract_ids.cmp(&b.contract_ids))
    });

    let graph_digest = overlay.digest();
    let digest = audit_digest(&findings, &graph_digest);
    Ok(DebVerifyAudit {
        findings,
        audited_contracts: contracts.len(),
        graph_digest,
        digest,
        evaluated_at: now,
    })
}

/// sha256 over the canonical audit payload (REQ-AC5-016).
pub fn audit_digest(findings: &[DebVerifyFinding], graph_digest: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(AUDIT_DIGEST_DOMAIN.as_bytes());
    h.update(b"|graph|");
    h.update(graph_digest);
    h.update(b"|findings|");
    for f in findings {
        h.update(f.kind.canonical_tag().as_bytes());
        h.update(b"|");
        h.update(f.severity.canonical_tag().as_bytes());
        h.update(b"|");
        for s in &f.subjects {
            h.update(s.as_bytes());
            h.update(b",");
        }
        h.update(b"|");
        for c in &f.contract_ids {
            h.update(c.as_str().as_bytes());
            h.update(b",");
        }
        h.update(b"\n");
    }
    h.finalize().into()
}
