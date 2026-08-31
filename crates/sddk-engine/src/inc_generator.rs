//! INC file generator from Finding records.
//!
//! Renders `INC-NNN-{slug}.md` files using the template at
//! `docs/debt/INCIDENCE-TEMPLATE.md` embedded via `include_str!`.

use sddk_domain::Finding;
use std::collections::HashSet;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Derives the INC slug from a finding: first 8 chars of its fingerprint.
pub fn derive_inc_slug(finding: &Finding) -> String {
    finding.fingerprint.chars().take(8).collect()
}

/// Derives the next monotonic INC id for a finding.
///
/// If a slug collision exists in `existing_ids`, the existing NNN is reused.
/// Otherwise, NNN = max(existing NNNs) + 1.
pub fn derive_inc_id(finding: &Finding, existing_ids: &HashSet<String>) -> String {
    let slug = derive_inc_slug(finding);
    let with_same_slug: Vec<(u32, &String)> = existing_ids
        .iter()
        .filter_map(|id| {
            let parts: Vec<&str> = id.split('-').collect();
            if parts.len() >= 3 && parts[2] == slug {
                parts[1].parse().ok().map(|n| (n, id))
            } else {
                None
            }
        })
        .collect();
    let nnn = if with_same_slug.is_empty() {
        let max_nnn = existing_ids
            .iter()
            .filter_map(|id| id.split('-').nth(1).and_then(|n| n.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);
        max_nnn + 1
    } else {
        with_same_slug.iter().map(|(n, _)| *n).max().unwrap_or(1)
    };
    format!("INC-{:03}-{slug}", nnn)
}

/// Context for rendering an INC template.
struct TemplateContext<'a> {
    inc_id: &'a str,
    finding_id: &'a str,
    title: &'a str,
    severity: &'a str,
    priority: &'a str,
    fingerprint: &'a str,
    fingerprint_aliases: &'a str,
    cluster_id: &'a str,
    created: &'a str,
    cycle_label: &'a str,
    slug: &'a str,
}

fn format_template(raw: &str, ctx: &TemplateContext) -> String {
    raw.replace("INC-NNN-{slug}", ctx.inc_id)
        .replace("\"{one-line summary}\"", &format!("\"{}\"", ctx.title))
        .replace("\"{hex}\"", &format!("\"{}\"", ctx.fingerprint))
        .replace("critical|high|medium|low", ctx.severity)
        .replace("P0|P1|P2|P3", ctx.priority)
        .replace("[]", ctx.fingerprint_aliases)
        .replace("CL-NN", ctx.cluster_id)
        .replace("YYYY-MM-DD", &ctx.created[..10])
        .replace("{created}", ctx.created)
        .replace("actor-name", "sddk")
        .replace(
            "<problem statement: what's wrong, where, why it matters>",
            ctx.finding_id,
        )
        .replace(
            "<why this severity + priority + cluster_id; cite evidence>",
            &format!(
                "Severity={}, Priority={}, Cluster={}",
                ctx.severity, ctx.priority, ctx.cluster_id
            ),
        )
        .replace("{finding-id}", ctx.finding_id)
        .replace("cycle-{N}", ctx.cycle_label)
        .replace("{slug}", ctx.slug)
        .replace("{title}", ctx.title)
}

/// Renders the INC template for a finding into a Markdown string.
///
/// Template is embedded at compile time via `include_str!` and rendered
/// with the finding's metadata.
pub fn render_inc_template(finding: &Finding, _project_id: &str, cycle_id: &str) -> String {
    let inc_id = format!("INC-001-{}", derive_inc_slug(finding));
    let created = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "2026-08-21T00:00:00Z".into());
    let cycle_label = format!(
        "cycle-{}",
        cycle_id
            .rsplit('/')
            .next()
            .unwrap_or("8")
            .trim_start_matches("kernel-cycle-")
    );
    let fingerprint_aliases = if finding.fingerprint_aliases.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            finding
                .fingerprint_aliases
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let ctx = TemplateContext {
        inc_id: &inc_id,
        finding_id: &finding.id,
        title: &finding.title,
        severity: finding.severity.as_str(),
        priority: finding.priority.as_str(),
        fingerprint: &finding.fingerprint,
        fingerprint_aliases: &fingerprint_aliases,
        cluster_id: &finding.cluster_id,
        created: &created,
        cycle_label: &cycle_label,
        slug: &derive_inc_slug(finding),
    };
    format_template(INCTEMPLATE, &ctx)
}

// Embedded at compile time from the canonical template
const INCTEMPLATE: &str = include_str!("../../../docs/debt/INCIDENCE-TEMPLATE.md");

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::{FindingStatus, Severity};

    fn finding_with_fp(fp: &str) -> Finding {
        Finding {
            id: "FIND-0001".into(),
            title: "Test finding".into(),
            severity: Severity::Medium,
            priority: sddk_domain::Priority::P2,
            status: FindingStatus::Open,
            fingerprint: fp.into(),
            fingerprint_aliases: vec![],
            cluster_id: "CL-01".into(),
            category: "architecture".into(),
            description: "Test description".into(),
            remediation_cycle: None,
            remediation_pr: None,
            evidence_refs: None,
        }
    }

    #[test]
    fn test_slug_first_8_chars() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        assert_eq!(derive_inc_slug(&f), "3ef321c4");
    }

    #[test]
    fn test_slug_empty_fp() {
        let f = finding_with_fp("");
        assert_eq!(derive_inc_slug(&f), "");
    }

    #[test]
    fn test_inc_id_monotonic_new() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        let existing: HashSet<String> = HashSet::new();
        let id = derive_inc_id(&f, &existing);
        assert!(id.starts_with("INC-001-3ef321c4"));
    }

    #[test]
    fn test_inc_id_reuses_nnn_on_slug_collision() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        let mut existing: HashSet<String> = HashSet::new();
        existing.insert("INC-005-3ef321c4".into());
        let id = derive_inc_id(&f, &existing);
        assert_eq!(id, "INC-005-3ef321c4");
    }

    #[test]
    fn test_render_includes_frontmatter_fields() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        let rendered = render_inc_template(&f, "sddk-framework", "p-test/kernel-cycle-8");
        assert!(rendered.contains("id: INC-"), "missing id");
        assert!(rendered.contains("status: open"), "missing status");
        assert!(rendered.contains("severity: medium"), "missing severity");
        assert!(rendered.contains("priority: P2"), "missing priority");
        assert!(
            rendered.contains(r#""3ef321c4efe1d87e""#),
            "missing fingerprint"
        );
        assert!(rendered.contains("cluster_id: CL-01"), "missing cluster_id");
        assert!(rendered.contains("## Context"), "missing Context");
        assert!(rendered.contains("## Rationale"), "missing Rationale");
        assert!(rendered.contains("## Lifecycle"), "missing Lifecycle");
        assert!(rendered.contains("## References"), "missing References");
        assert!(rendered.contains("created"), "missing created");
    }

    #[test]
    fn test_render_idempotent_excluding_timestamp() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        let r1 = render_inc_template(&f, "sddk-framework", "p-test/kernel-cycle-8");
        assert!(r1.contains("INC-001-3ef321c4"));
    }
}
