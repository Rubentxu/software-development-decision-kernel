//! Neutral Forge boundary and provider adapters.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::runner::{RunOutcome, RunSpec, RunnerError, run};

/// Errors emitted by forge operations.
#[derive(Debug, Error)]
pub enum ForgeError {
    /// The provider command failed.
    #[error("forge operation failed: {0}")]
    Command(String),
    /// The provider response could not be parsed.
    #[error("forge response parse failure: {0}")]
    Parse(String),
    /// A required effect is missing.
    #[error("forge effect missing: {0}")]
    Missing(String),
}

/// Request to open a pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrRequest {
    /// PR title.
    pub title: String,
    /// PR body.
    pub body: String,
    /// Source branch.
    pub head: String,
    /// Target branch.
    pub base: String,
}

/// Receipt for an opened pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PrReceipt {
    /// Provider pull request number.
    pub pr_number: u64,
    /// Provider URL.
    pub url: String,
}

/// State of one required check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CheckState {
    /// Check name.
    pub name: String,
    /// `Some(true)` passed, `Some(false)` failed, `None` still pending.
    pub passed: Option<bool>,
}

/// Receipt for a merge attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MergeReceipt {
    /// Whether the PR is merged after the attempt.
    pub merged: bool,
    /// Merge commit SHA when merged.
    pub merge_sha: Option<String>,
}

/// Request to publish a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseRequest {
    /// Release tag.
    pub tag: String,
    /// Release title.
    pub title: String,
    /// Release notes.
    pub notes: String,
    /// Commit, branch, or tag the release targets.
    pub target_commitish: String,
}

/// Receipt for a published release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseReceipt {
    /// Published tag.
    pub tag: String,
    /// Provider URL.
    pub url: String,
}

/// Observable state of a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseState {
    /// Whether the release exists at the provider.
    pub published: bool,
}

/// Neutral forge boundary without provider-specific types.
pub trait Forge {
    /// Opens a pull request.
    fn create_pr(&mut self, request: &PrRequest) -> Result<PrReceipt, ForgeError>;
    /// Finds an open PR for a head/base pair.
    fn find_open_pr(&self, head: &str, base: &str) -> Result<Option<u64>, ForgeError>;
    /// Reads the required checks of a pull request.
    fn read_checks(&self, pr_number: u64) -> Result<Vec<CheckState>, ForgeError>;
    /// Merges a pull request, tolerating an already-merged state.
    fn merge_pr(&mut self, pr_number: u64) -> Result<MergeReceipt, ForgeError>;
    /// Publishes a release, tolerating an already-published state.
    fn create_release(&mut self, request: &ReleaseRequest) -> Result<ReleaseReceipt, ForgeError>;
    /// Reads the observable state of a release.
    fn release_state(&self, tag: &str) -> Result<Option<ReleaseState>, ForgeError>;
}

/// Runner function shared by adapters for test injection.
type Runner = dyn Fn(&RunSpec) -> Result<RunOutcome, RunnerError>;

/// GitHub adapter driven by the `gh` CLI through the typed runner.
pub struct GitHubForge {
    repo: String,
    runner: Box<Runner>,
}

impl GitHubForge {
    /// Creates an adapter for one `owner/repo` using the real runner.
    pub fn new(repo: impl Into<String>) -> Self {
        Self {
            repo: repo.into(),
            runner: Box::new(run),
        }
    }

    /// Creates an adapter with an injected runner (tests).
    pub fn with_runner(repo: impl Into<String>, runner: Box<Runner>) -> Self {
        Self {
            repo: repo.into(),
            runner,
        }
    }

    fn run_gh(&self, args: &[&str]) -> Result<RunOutcome, ForgeError> {
        let mut spec = RunSpec::new("gh");
        spec.args.push("--repo".into());
        spec.args.push(self.repo.clone());
        spec.args.extend(args.iter().map(|arg| (*arg).to_owned()));
        let outcome =
            (self.runner)(&spec).map_err(|error| ForgeError::Command(error.to_string()))?;
        if let Some(status) = outcome.exit_status
            && status != 0
        {
            return Err(ForgeError::Command(format!(
                "gh exited {status}: {}",
                outcome.stderr.trim()
            )));
        }
        Ok(outcome)
    }
}

#[derive(Deserialize)]
struct PrCreateJson {
    number: u64,
    url: String,
}

#[derive(Deserialize)]
struct PrListJson {
    number: u64,
}

#[derive(Deserialize)]
struct PrViewJson {
    state: String,
    #[serde(rename = "mergeCommit")]
    merge_commit: Option<MergeCommitJson>,
}

#[derive(Deserialize)]
struct MergeCommitJson {
    oid: String,
}

#[derive(Deserialize)]
struct CheckJson {
    name: String,
    state: String,
    #[serde(rename = "conclusion")]
    conclusion: Option<String>,
}

#[derive(Deserialize)]
struct ReleaseJson {
    url: String,
}

impl Forge for GitHubForge {
    fn create_pr(&mut self, request: &PrRequest) -> Result<PrReceipt, ForgeError> {
        let outcome = self.run_gh(&[
            "pr",
            "create",
            "--title",
            &request.title,
            "--body",
            &request.body,
            "--head",
            &request.head,
            "--base",
            &request.base,
            "--json",
            "number,url",
        ])?;
        let parsed: PrCreateJson = serde_json::from_str(&outcome.stdout)
            .map_err(|error| ForgeError::Parse(error.to_string()))?;
        if parsed.number == 0 {
            return Err(ForgeError::Missing("pr number".into()));
        }
        Ok(PrReceipt {
            pr_number: parsed.number,
            url: parsed.url,
        })
    }

    fn find_open_pr(&self, head: &str, base: &str) -> Result<Option<u64>, ForgeError> {
        let outcome = self.run_gh(&[
            "pr", "list", "--head", head, "--base", base, "--state", "open", "--json", "number",
        ])?;
        let parsed: Vec<PrListJson> = serde_json::from_str(&outcome.stdout)
            .map_err(|error| ForgeError::Parse(error.to_string()))?;
        Ok(parsed.first().map(|pr| pr.number))
    }

    fn read_checks(&self, pr_number: u64) -> Result<Vec<CheckState>, ForgeError> {
        let outcome = self.run_gh(&[
            "pr",
            "checks",
            &pr_number.to_string(),
            "--json",
            "name,state,conclusion",
        ])?;
        let parsed: Vec<CheckJson> = serde_json::from_str(&outcome.stdout)
            .map_err(|error| ForgeError::Parse(error.to_string()))?;
        Ok(parsed
            .into_iter()
            .map(|check| CheckState {
                name: check.name,
                passed: match (check.state.as_str(), check.conclusion.as_deref()) {
                    ("COMPLETED", Some("SUCCESS")) => Some(true),
                    ("COMPLETED", Some("FAILURE")) => Some(false),
                    ("COMPLETED", Some("CANCELLED")) | ("COMPLETED", Some("SKIPPED")) => Some(true),
                    _ => None,
                },
            })
            .collect())
    }

    fn merge_pr(&mut self, pr_number: u64) -> Result<MergeReceipt, ForgeError> {
        match self.run_gh(&["pr", "merge", &pr_number.to_string(), "--merge"]) {
            Ok(_) => Ok(MergeReceipt {
                merged: true,
                merge_sha: None,
            }),
            Err(_) => {
                let view = self.run_gh(&[
                    "pr",
                    "view",
                    &pr_number.to_string(),
                    "--json",
                    "state,mergeCommit",
                ])?;
                let parsed: PrViewJson = serde_json::from_str(&view.stdout)
                    .map_err(|error| ForgeError::Parse(error.to_string()))?;
                if parsed.state == "MERGED" {
                    Ok(MergeReceipt {
                        merged: true,
                        merge_sha: parsed.merge_commit.map(|commit| commit.oid),
                    })
                } else {
                    Err(ForgeError::Command(format!(
                        "PR {pr_number} could not be merged"
                    )))
                }
            }
        }
    }

    fn create_release(&mut self, request: &ReleaseRequest) -> Result<ReleaseReceipt, ForgeError> {
        match self.run_gh(&[
            "release",
            "create",
            &request.tag,
            "--title",
            &request.title,
            "--notes",
            &request.notes,
            "--target",
            &request.target_commitish,
            "--json",
            "url",
        ]) {
            Ok(outcome) => {
                let parsed: ReleaseJson = serde_json::from_str(&outcome.stdout)
                    .map_err(|error| ForgeError::Parse(error.to_string()))?;
                Ok(ReleaseReceipt {
                    tag: request.tag.clone(),
                    url: parsed.url,
                })
            }
            Err(_) => {
                if self
                    .release_state(&request.tag)?
                    .is_some_and(|state| state.published)
                {
                    Ok(ReleaseReceipt {
                        tag: request.tag.clone(),
                        url: String::new(),
                    })
                } else {
                    Err(ForgeError::Command(format!(
                        "release {} could not be created",
                        request.tag
                    )))
                }
            }
        }
    }

    fn release_state(&self, tag: &str) -> Result<Option<ReleaseState>, ForgeError> {
        match self.run_gh(&["release", "view", tag, "--json", "url"]) {
            Ok(_) => Ok(Some(ReleaseState { published: true })),
            Err(_) => Ok(Some(ReleaseState { published: false })),
        }
    }
}

/// In-memory forge double for contract and interruption tests.
#[derive(Debug, Default)]
pub struct MockForge {
    open_prs: Vec<(String, String, u64)>,
    next_number: u64,
    merged: HashSet<u64>,
    releases: HashSet<String>,
    checks: HashMap<u64, Vec<CheckState>>,
    pr_urls: HashMap<u64, String>,
    fail_merge: bool,
}

impl MockForge {
    /// Creates an empty forge double.
    pub fn new() -> Self {
        Self {
            next_number: 1,
            ..Self::default()
        }
    }

    /// Seeds an open PR as if created by a previous run.
    pub fn seed_open_pr(&mut self, head: &str, base: &str, number: u64) {
        self.open_prs
            .push((head.to_owned(), base.to_owned(), number));
        self.next_number = self.next_number.max(number + 1);
    }

    /// Seeds a merged PR as if merged by a previous run.
    pub fn seed_merged(&mut self, number: u64) {
        self.merged.insert(number);
    }

    /// Seeds a published release as if created by a previous run.
    pub fn seed_release(&mut self, tag: &str) {
        self.releases.insert(tag.to_owned());
    }

    /// Seeds required checks for a PR.
    pub fn seed_checks(&mut self, number: u64, checks: Vec<CheckState>) {
        self.checks.insert(number, checks);
    }

    /// Simulates a provider merge failure (e.g., checks not ready).
    pub fn set_fail_merge(&mut self, fail: bool) {
        self.fail_merge = fail;
    }

    /// Returns whether a PR is merged.
    pub fn is_merged(&self, number: u64) -> bool {
        self.merged.contains(&number)
    }

    /// Returns whether a release is published.
    pub fn is_published(&self, tag: &str) -> bool {
        self.releases.contains(tag)
    }

    /// Renders the mock state for assertions.
    pub fn state_text(&self) -> String {
        let mut output = String::new();
        writeln!(output, "open_prs: {}", self.open_prs.len()).unwrap();
        writeln!(output, "merged: {}", self.merged.len()).unwrap();
        write!(output, "releases: {}", self.releases.len()).unwrap();
        output
    }
}

impl Forge for MockForge {
    fn create_pr(&mut self, request: &PrRequest) -> Result<PrReceipt, ForgeError> {
        let number = self.next_number;
        self.next_number += 1;
        self.open_prs
            .push((request.head.clone(), request.base.clone(), number));
        self.pr_urls
            .insert(number, format!("https://mock.example/pr/{number}"));
        Ok(PrReceipt {
            pr_number: number,
            url: format!("https://mock.example/pr/{number}"),
        })
    }

    fn find_open_pr(&self, head: &str, base: &str) -> Result<Option<u64>, ForgeError> {
        Ok(self
            .open_prs
            .iter()
            .find(|(candidate_head, candidate_base, _)| {
                candidate_head == head && candidate_base == base
            })
            .map(|(_, _, number)| *number))
    }

    fn read_checks(&self, pr_number: u64) -> Result<Vec<CheckState>, ForgeError> {
        Ok(self.checks.get(&pr_number).cloned().unwrap_or_default())
    }

    fn merge_pr(&mut self, pr_number: u64) -> Result<MergeReceipt, ForgeError> {
        if self.fail_merge {
            return Err(ForgeError::Command("merge rejected by policy".into()));
        }
        if self.merged.contains(&pr_number) {
            return Ok(MergeReceipt {
                merged: true,
                merge_sha: Some(format!("sha-{pr_number}")),
            });
        }
        if !self
            .open_prs
            .iter()
            .any(|(_, _, number)| *number == pr_number)
        {
            return Err(ForgeError::Missing(format!("PR {pr_number}")));
        }
        self.merged.insert(pr_number);
        self.open_prs.retain(|(_, _, number)| *number != pr_number);
        Ok(MergeReceipt {
            merged: true,
            merge_sha: Some(format!("sha-{pr_number}")),
        })
    }

    fn create_release(&mut self, request: &ReleaseRequest) -> Result<ReleaseReceipt, ForgeError> {
        if self.releases.contains(&request.tag) {
            return Ok(ReleaseReceipt {
                tag: request.tag.clone(),
                url: format!("https://mock.example/releases/{}", request.tag),
            });
        }
        self.releases.insert(request.tag.clone());
        Ok(ReleaseReceipt {
            tag: request.tag.clone(),
            url: format!("https://mock.example/releases/{}", request.tag),
        })
    }

    fn release_state(&self, tag: &str) -> Result<Option<ReleaseState>, ForgeError> {
        Ok(Some(ReleaseState {
            published: self.releases.contains(tag),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::runner::RunOutcome;

    use super::{CheckState, Forge, GitHubForge, MockForge, PrRequest, ReleaseRequest};

    #[test]
    fn mock_forge_implements_contract() {
        let mut forge = MockForge::new();
        let receipt = forge
            .create_pr(&PrRequest {
                title: "t".into(),
                body: "b".into(),
                head: "feat/x".into(),
                base: "main".into(),
            })
            .unwrap();
        assert_eq!(
            forge.find_open_pr("feat/x", "main").unwrap(),
            Some(receipt.pr_number)
        );
        let merge = forge.merge_pr(receipt.pr_number).unwrap();
        assert!(merge.merged);
        assert_eq!(forge.find_open_pr("feat/x", "main").unwrap(), None);
        let release = forge
            .create_release(&ReleaseRequest {
                tag: "v1.0.0".into(),
                title: "t".into(),
                notes: "n".into(),
                target_commitish: "main".into(),
            })
            .unwrap();
        assert!(
            forge
                .release_state(&release.tag)
                .unwrap()
                .unwrap()
                .published
        );
    }

    #[test]
    fn github_forge_parses_gh_outputs() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut forge = GitHubForge::with_runner("owner/repo", {
            let calls = calls.clone();
            Box::new(move |spec| {
                calls.borrow_mut().push(spec.args.join(" "));
                let output = match spec.args.last().map(String::as_str) {
                    Some("number,url") => {
                        r#"{"number":7,"url":"https://github.com/owner/repo/pull/7"}"#
                    }
                    Some("number") => r#"[{"number":7}]"#,
                    Some("name,state,conclusion") => {
                        r#"[{"name":"CI","state":"COMPLETED","conclusion":"SUCCESS"}]"#
                    }
                    Some("state,mergeCommit") => {
                        r#"{"state":"MERGED","mergeCommit":{"oid":"abc123"}}"#
                    }
                    Some("url") => r#"{"url":"https://github.com/owner/repo/releases/tag/v1.0.0"}"#,
                    _ => "{}",
                };
                Ok(RunOutcome {
                    exit_status: Some(0),
                    stdout: output.to_owned(),
                    stderr: String::new(),
                    timed_out: false,
                })
            })
        });
        let pr = forge
            .create_pr(&PrRequest {
                title: "t".into(),
                body: "b".into(),
                head: "feat/x".into(),
                base: "main".into(),
            })
            .unwrap();
        assert_eq!(pr.pr_number, 7);
        assert_eq!(forge.find_open_pr("feat/x", "main").unwrap(), Some(7));
        let checks = forge.read_checks(7).unwrap();
        assert_eq!(
            checks,
            vec![CheckState {
                name: "CI".into(),
                passed: Some(true)
            }]
        );
        assert_eq!(calls.borrow().len(), 3);
    }

    #[test]
    fn github_forge_merge_tolerates_already_merged() {
        let mut forge = GitHubForge::with_runner(
            "owner/repo",
            Box::new(|spec| {
                let failed = spec.args.windows(2).any(|pair| pair == ["merge", "12"]);
                let output = if failed {
                    RunOutcome {
                        exit_status: Some(1),
                        stdout: String::new(),
                        stderr: "merge failed: already merged".into(),
                        timed_out: false,
                    }
                } else {
                    RunOutcome {
                        exit_status: Some(0),
                        stdout: r#"{"state":"MERGED","mergeCommit":{"oid":"abc123"}}"#.into(),
                        stderr: String::new(),
                        timed_out: false,
                    }
                };
                Ok(output)
            }),
        );
        let receipt = forge.merge_pr(12).unwrap();
        assert!(receipt.merged);
        assert_eq!(receipt.merge_sha.as_deref(), Some("abc123"));
    }
}
