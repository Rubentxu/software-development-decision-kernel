//! Typed local Git executor with postcondition verification.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;

use crate::runner::{RawRunOutcome, RunOutcome, RunSpec, run_raw};

const LOCAL_GIT_ENV_KEYS: &[&str] = &[
    "HOME",
    "PATH",
    "USER",
    "LOGNAME",
    "XDG_CONFIG_HOME",
    "GH_CONFIG_DIR",
    "SSH_AUTH_SOCK",
    "GIT_ASKPASS",
    "SSH_ASKPASS",
    "GIT_TERMINAL_PROMPT",
];

/// Returns the environment allowlist for `git.*` capabilities.
/// Keys are filtered against `LOCAL_GIT_ENV_KEYS`; secrets like `GH_TOKEN`
/// and `GITHUB_TOKEN` are intentionally excluded.
pub fn git_capability_env() -> BTreeMap<String, String> {
    LOCAL_GIT_ENV_KEYS
        .iter()
        .filter_map(|key| {
            std::env::var_os(key)
                .map(|value| ((*key).to_owned(), value.to_string_lossy().into_owned()))
        })
        .collect()
}

/// Errors emitted by typed Git operations.
#[derive(Debug, Error)]
pub enum GitError {
    /// The typed git run failed to spawn or execute.
    #[error("git runner error: {0}")]
    Runner(#[from] crate::runner::RunnerError),
    /// The git command exited with a non-zero status.
    #[error("git {command} failed with exit status {status}: {stderr}")]
    CommandFailed {
        /// Executed git subcommand.
        command: String,
        /// Non-zero exit status.
        status: i32,
        /// Captured standard error.
        stderr: String,
    },
    /// The verified postcondition did not hold after the command.
    #[error("git {command} postcondition failed: expected {expected}, found {actual}")]
    Postcondition {
        /// Executed git subcommand.
        command: String,
        /// Expected observable state.
        expected: String,
        /// Observed state.
        actual: String,
    },
    /// The git command failed due to missing or invalid credentials.
    #[error("git {command} auth failure: {stderr}\n{hint}")]
    AuthFailed {
        /// Executed git subcommand.
        command: String,
        /// Captured standard error.
        stderr: String,
        /// Actionable remediation hint.
        hint: String,
    },
}

/// Hint emitted when `git push` fails due to missing credentials.
const AUTH_HINT: &str = r#"credentials not available to the typed runner.
The runner has no TTY and uses an env allowlist that excludes GH_TOKEN/GITHUB_TOKEN.
To fix, choose ONE of:
  1. gh auth login                       # interactive, requires TTY
  2. gh auth setup-git                   # configure git credential helper via gh
  3. git config --global credential.helper store
     git push                            # one-time cache"#;

/// Returns `Some(AuthFailed)` if `stderr` matches a known auth-failure marker,
/// or `None` for non-auth failures.
pub(crate) fn classify_auth_failure(command: &str, stderr: &str) -> Option<GitError> {
    let lower = stderr.to_lowercase();
    let markers = [
        "could not read username",
        "terminal prompts disabled",
        "403 forbidden",
        "bad credentials",
        "failed to authenticate",
        "fatal: authentication failed",
    ];
    if markers.iter().any(|m| lower.contains(m)) {
        Some(GitError::AuthFailed {
            command: command.to_owned(),
            stderr: stderr.to_owned(),
            hint: AUTH_HINT.to_owned(),
        })
    } else {
        None
    }
}

/// Read-only snapshot of repository state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GitInspect {
    /// Current HEAD short SHA, when the repository has commits.
    pub head: Option<String>,
    /// Current branch name, when detached or unborn.
    pub branch: Option<String>,
    /// Whether the worktree has uncommitted changes.
    pub dirty: bool,
}

/// Result of creating a branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GitBranch {
    /// Created branch name.
    pub branch: String,
}

/// Result of creating a commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GitCommit {
    /// Short SHA of the new HEAD.
    pub sha: String,
}

/// Result of creating a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GitTag {
    /// Created tag name.
    pub tag: String,
}

/// Typed Git boundary executing commands without a shell.
#[derive(Debug, Clone)]
pub struct GitExecutor {
    root: PathBuf,
    /// Environment allowlist applied to every invocation.
    env: BTreeMap<String, String>,
    timeout_ms: u64,
    output_max_bytes: usize,
}

impl GitExecutor {
    /// Creates an executor over one repository root.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            env: git_capability_env(),
            timeout_ms: 30_000,
            output_max_bytes: 1_048_576,
        }
    }

    /// Returns the repository root.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Overrides the environment allowlist (for example Git identity).
    pub fn with_env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Reads the repository head, branch, and dirty state.
    pub fn inspect(&self) -> Result<GitInspect, GitError> {
        let head = self
            .run_ok("rev-parse", &["--short", "HEAD"])
            .ok()
            .map(|outcome| outcome.stdout.trim().to_owned());
        let branch = self
            .run_ok("branch", &["--show-current"])
            .ok()
            .map(|outcome| outcome.stdout.trim().to_owned())
            .filter(|name| !name.is_empty());
        let dirty = self
            .run_ok("status", &["--porcelain"])
            .map(|outcome| !outcome.stdout.trim().is_empty())
            .unwrap_or(false);
        Ok(GitInspect {
            head,
            branch,
            dirty,
        })
    }

    /// Creates a branch and verifies it is the current branch afterwards.
    pub fn create_branch(&self, name: &str) -> Result<GitBranch, GitError> {
        self.run_ok("checkout", &["-b", name])?;
        let current = self
            .run_ok("symbolic-ref", &["--short", "HEAD"])?
            .stdout
            .trim()
            .to_owned();
        if current != name {
            return Err(GitError::Postcondition {
                command: "checkout -b".into(),
                expected: name.into(),
                actual: current,
            });
        }
        Ok(GitBranch {
            branch: name.to_owned(),
        })
    }

    /// Creates an empty commit and verifies HEAD matches the reported SHA.
    pub fn commit(&self, message: &str) -> Result<GitCommit, GitError> {
        self.run_ok("commit", &["--allow-empty", "-m", message])?;
        let head = self
            .run_ok("rev-parse", &["--short", "HEAD"])?
            .stdout
            .trim()
            .to_owned();
        if head.is_empty() {
            return Err(GitError::Postcondition {
                command: "commit".into(),
                expected: "a non-empty HEAD".into(),
                actual: "empty".into(),
            });
        }
        Ok(GitCommit { sha: head })
    }

    /// Creates a tag and verifies it is listed afterwards.
    pub fn tag(&self, name: &str) -> Result<GitTag, GitError> {
        self.run_ok("tag", &[name])?;
        let listed = self
            .run_ok("tag", &["--list", name])?
            .stdout
            .trim()
            .to_owned();
        if listed != name {
            return Err(GitError::Postcondition {
                command: "tag".into(),
                expected: name.into(),
                actual: listed,
            });
        }
        Ok(GitTag {
            tag: name.to_owned(),
        })
    }

    /// Executes an arbitrary read-only `git` subcommand and returns its
    /// stdout. Fail-closed on non-zero exit.
    ///
    /// Used by deterministic read-only reducers (cycle files inventory) that
    /// need to call `git diff`/`git status`/`git check-ignore` and capture
    /// raw text. The same env allowlist, timeout, and output-cap policy as
    /// the rest of the gateway apply — this method never pushes, mutates
    /// refs, or escapes the configured repository root.
    pub fn run_read_only(&self, command: &str, args: &[&str]) -> Result<String, GitError> {
        let outcome = self.run_ok(command, args)?;
        Ok(outcome.stdout)
    }

    /// Returns the full SHA of the current HEAD.
    pub fn head_sha(&self) -> Result<String, GitError> {
        let head = self
            .run_ok("rev-parse", &["HEAD"])?
            .stdout
            .trim()
            .to_owned();
        if head.is_empty() {
            return Err(GitError::Postcondition {
                command: "rev-parse HEAD".into(),
                expected: "a non-empty HEAD SHA".into(),
                actual: "empty".into(),
            });
        }
        Ok(head)
    }

    /// Probes whether `root` is inside a Git worktree.
    ///
    /// Returns `Ok(true)` for a regular worktree and `Ok(false)` for a bare
    /// repository or a non-Git directory. Git failures are only treated as
    /// outside Git when no `.git` marker exists in `root` or its ancestors.
    pub fn is_inside_work_tree(&self) -> Result<bool, GitError> {
        let has_git_marker = self
            .root
            .ancestors()
            .any(|directory| std::fs::symlink_metadata(directory.join(".git")).is_ok());

        match self.run_ok("rev-parse", &["--is-inside-work-tree"]) {
            Ok(outcome) => {
                let membership = outcome.stdout.trim();
                match membership {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    other => Err(GitError::Postcondition {
                        command: "rev-parse --is-inside-work-tree".into(),
                        expected: "true or false".into(),
                        actual: other.into(),
                    }),
                }
            }
            Err(GitError::CommandFailed { status: 128, .. }) if !has_git_marker => Ok(false),
            Err(GitError::Runner(_)) if !has_git_marker => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Lists tracked paths matching the supplied Git pathspecs.
    ///
    /// Returns `Err(GitError::CommandFailed)` when root is not in a worktree
    /// or git enumeration fails. Callers must check if each path exists
    /// and is a regular file.
    pub fn ls_files(&self, pathspecs: &[&str]) -> Result<Vec<PathBuf>, GitError> {
        let mut args = vec!["-z", "--"];
        args.extend(pathspecs.iter().copied());
        let outcome = self.run_raw_ok("ls-files", &args)?;
        if outcome.stdout.len() > self.output_max_bytes {
            return Err(GitError::Postcondition {
                command: "ls-files".into(),
                expected: format!("output no larger than {} bytes", self.output_max_bytes),
                actual: format!("{} bytes", outcome.stdout.len()),
            });
        }
        outcome
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| {
                std::str::from_utf8(path)
                    .map(PathBuf::from)
                    .map_err(|_| GitError::Postcondition {
                        command: "ls-files".into(),
                        expected: "tracked paths encoded as UTF-8".into(),
                        actual: "non-UTF-8 tracked path".into(),
                    })
            })
            .collect()
    }

    /// Returns the remote SHA of a branch, or `None` when it does not exist.
    pub fn remote_branch_sha(&self, branch: &str) -> Result<Option<String>, GitError> {
        let reference = format!("refs/heads/{branch}");
        let output = self.run_ok("ls-remote", &["--heads", "origin", &reference])?;
        Ok(output.stdout.split_whitespace().next().map(str::to_owned))
    }

    /// Pushes a branch and verifies that its remote SHA equals the local HEAD.
    pub fn push_and_verify_branch(&self, branch: &str) -> Result<String, GitError> {
        let head = self.head_sha()?;
        self.run_push_raw(branch)?; // propagates AuthFailed on credential errors
        let remote = self.remote_branch_sha(branch)?.unwrap_or_default();
        if remote != head {
            return Err(GitError::Postcondition {
                command: format!("push origin {branch}"),
                expected: head,
                actual: remote,
            });
        }
        self.head_sha()
    }

    /// Verifies that the remote branch SHA equals the current local HEAD.
    pub fn verify_head_matches_remote_branch(&self, branch: &str) -> Result<String, GitError> {
        let head = self.head_sha()?;
        let remote = self.remote_branch_sha(branch)?.unwrap_or_default();
        if remote != head {
            return Err(GitError::Postcondition {
                command: format!("verify origin/{branch}"),
                expected: head,
                actual: remote,
            });
        }
        Ok(head)
    }

    /// Returns the peeled commit SHA of a local annotated tag.
    pub fn annotated_tag_target(&self, tag: &str) -> Result<Option<String>, GitError> {
        let reference = format!("refs/tags/{tag}");
        let output = self.run_ok("for-each-ref", &["--format=%(objecttype)", &reference])?;
        let line = output.stdout.trim();
        if line.is_empty() {
            return Ok(None);
        }
        if line != "tag" {
            return Err(GitError::Postcondition {
                command: format!("verify annotated tag {tag}"),
                expected: "an annotated tag pointing to a commit".into(),
                actual: line.to_owned(),
            });
        }
        let peeled_reference = format!("{reference}^{{}}");
        let target = self
            .run_ok("rev-parse", &[&peeled_reference])?
            .stdout
            .trim()
            .to_owned();
        if target.is_empty() {
            return Err(GitError::Postcondition {
                command: format!("verify annotated tag {tag}"),
                expected: "an annotated tag pointing to a commit".into(),
                actual: "empty peeled target".into(),
            });
        }
        Ok(Some(target))
    }

    /// Returns the peeled commit SHA of a remote annotated tag.
    pub fn remote_annotated_tag_target(&self, tag: &str) -> Result<Option<String>, GitError> {
        let reference = format!("refs/tags/{tag}");
        let peeled_reference = format!("{reference}^{{}}");
        let output = self.run_ok(
            "ls-remote",
            &["--tags", "origin", &reference, &peeled_reference],
        )?;
        let lines = output.stdout.lines().collect::<Vec<_>>();
        if lines.is_empty() {
            return Ok(None);
        }
        let target = lines.iter().find_map(|line| {
            line.strip_suffix(&peeled_reference)
                .and_then(|prefix| prefix.split_whitespace().next())
        });
        match target {
            Some(target) => Ok(Some(target.to_owned())),
            None => Err(GitError::Postcondition {
                command: format!("verify remote annotated tag {tag}"),
                expected: "an annotated tag pointing to a commit".into(),
                actual: output.stdout.trim().to_owned(),
            }),
        }
    }

    /// Creates an annotated tag at `target`, accepting an existing identical tag.
    pub fn create_annotated_tag(
        &self,
        tag: &str,
        target: &str,
        message: &str,
    ) -> Result<GitTag, GitError> {
        match self.annotated_tag_target(tag)? {
            Some(existing) if existing == target => {
                return Ok(GitTag {
                    tag: tag.to_owned(),
                });
            }
            Some(existing) => {
                return Err(GitError::Postcondition {
                    command: format!("create annotated tag {tag}"),
                    expected: target.to_owned(),
                    actual: existing,
                });
            }
            None => {}
        }
        self.run_ok("tag", &["-a", tag, target, "-m", message])?;
        let actual = self.annotated_tag_target(tag)?.unwrap_or_default();
        if actual != target {
            return Err(GitError::Postcondition {
                command: format!("tag -a {tag}"),
                expected: target.to_owned(),
                actual,
            });
        }
        Ok(GitTag {
            tag: tag.to_owned(),
        })
    }

    /// Pushes an annotated tag and verifies its remote peeled commit SHA.
    pub fn push_and_verify_annotated_tag(&self, tag: &str, target: &str) -> Result<(), GitError> {
        let local = self.annotated_tag_target(tag)?.unwrap_or_default();
        if local != target {
            return Err(GitError::Postcondition {
                command: format!("push annotated tag {tag}"),
                expected: target.to_owned(),
                actual: local,
            });
        }
        let reference = format!("refs/tags/{tag}");
        self.run_push_ref_raw(&reference)?; // propagates AuthFailed on credential errors
        let remote = self.remote_annotated_tag_target(tag)?.unwrap_or_default();
        if remote != target {
            return Err(GitError::Postcondition {
                command: format!("push origin {tag}"),
                expected: target.to_owned(),
                actual: remote,
            });
        }
        Ok(())
    }

    fn run_ok(&self, command: &str, args: &[&str]) -> Result<RunOutcome, GitError> {
        Ok(self
            .run_raw_ok(command, args)?
            .into_lossy(self.output_max_bytes))
    }

    fn run_raw_ok(&self, command: &str, args: &[&str]) -> Result<RawRunOutcome, GitError> {
        let mut spec = RunSpec {
            program: "git".into(),
            args: vec![
                "-C".into(),
                self.root.to_string_lossy().into_owned(),
                command.into(),
            ],
            env: self.env.clone(),
            timeout_ms: self.timeout_ms,
            output_max_bytes: self.output_max_bytes,
        };
        spec.args.extend(args.iter().map(|arg| (*arg).to_owned()));
        let outcome = run_raw(&spec)?;
        if outcome.timed_out {
            return Err(GitError::CommandFailed {
                command: command.to_owned(),
                status: -1,
                stderr: "timed out".into(),
            });
        }
        if let Some(status) = outcome.exit_status
            && status != 0
        {
            return Err(GitError::CommandFailed {
                command: command.to_owned(),
                status,
                stderr: String::from_utf8_lossy(&outcome.stderr).into_owned(),
            });
        }
        Ok(outcome)
    }

    /// Like `run_raw_ok` but specialised for pushing tag references where we need
    /// to inspect the raw exit status before classifying auth failures.
    fn run_push_ref_raw(&self, reference: &str) -> Result<RawRunOutcome, GitError> {
        let spec = RunSpec {
            program: "git".into(),
            args: vec![
                "-C".into(),
                self.root.to_string_lossy().into_owned(),
                "push".into(),
                "origin".into(),
                reference.into(),
            ],
            env: self.env.clone(),
            timeout_ms: self.timeout_ms,
            output_max_bytes: self.output_max_bytes,
        };
        let outcome = run_raw(&spec)?;
        if outcome.timed_out {
            return Err(GitError::CommandFailed {
                command: "push".into(),
                status: -1,
                stderr: "timed out".into(),
            });
        }
        if let Some(status) = outcome.exit_status
            && status != 0
        {
            let stderr = String::from_utf8_lossy(&outcome.stderr).into_owned();
            if let Some(auth) = classify_auth_failure("push", &stderr) {
                return Err(auth);
            }
            return Err(GitError::CommandFailed {
                command: "push".into(),
                status,
                stderr,
            });
        }
        Ok(outcome)
    }

    /// Like `run_raw_ok` but specialised for push operations where we need
    /// to inspect the raw exit status before classifying auth failures.
    fn run_push_raw(&self, branch: &str) -> Result<RawRunOutcome, GitError> {
        let spec = RunSpec {
            program: "git".into(),
            args: vec![
                "-C".into(),
                self.root.to_string_lossy().into_owned(),
                "push".into(),
                "origin".into(),
                branch.into(),
            ],
            env: self.env.clone(),
            timeout_ms: self.timeout_ms,
            output_max_bytes: self.output_max_bytes,
        };
        let outcome = run_raw(&spec)?;
        if outcome.timed_out {
            return Err(GitError::CommandFailed {
                command: "push".into(),
                status: -1,
                stderr: "timed out".into(),
            });
        }
        if let Some(status) = outcome.exit_status
            && status != 0
        {
            let stderr = String::from_utf8_lossy(&outcome.stderr).into_owned();
            // Try to classify as auth failure first; fall back to generic CommandFailed.
            if let Some(auth) = classify_auth_failure("push", &stderr) {
                return Err(auth);
            }
            return Err(GitError::CommandFailed {
                command: "push".into(),
                status,
                stderr,
            });
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use super::{GitError, GitExecutor, LOCAL_GIT_ENV_KEYS};

    fn git_repo() -> (tempfile::TempDir, GitExecutor) {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .output()
            .unwrap();
        let mut env = BTreeMap::new();
        env.insert("GIT_AUTHOR_NAME".into(), "SDDK Test".into());
        env.insert("GIT_AUTHOR_EMAIL".into(), "test@sddk.dev".into());
        env.insert("GIT_COMMITTER_NAME".into(), "SDDK Test".into());
        env.insert("GIT_COMMITTER_EMAIL".into(), "test@sddk.dev".into());
        (directory, GitExecutor::new(root).with_env(env))
    }

    #[test]
    fn new_inherits_environment_required_by_local_git_credentials() {
        let directory = tempfile::tempdir().unwrap();
        let git = GitExecutor::new(directory.path().to_path_buf());

        for key in ["HOME", "PATH", "USER"] {
            if let Some(value) = std::env::var_os(key) {
                assert_eq!(
                    git.env.get(key).map(String::as_str),
                    Some(value.to_string_lossy().as_ref()),
                    "GitExecutor must preserve {key} for local credential helpers"
                );
            }
        }
    }

    #[test]
    fn inspect_reports_head_branch_and_dirty_state() {
        let (_directory, git) = git_repo();
        let before = git.inspect().unwrap();
        assert!(before.head.is_none());
        assert!(before.branch.is_some());

        git.commit("initial").unwrap();
        fs::write(git.root().join("file.txt"), "change").unwrap();
        let after = git.inspect().unwrap();
        assert!(after.head.is_some());
        assert!(after.dirty);
    }

    #[test]
    fn create_branch_verifies_postcondition() {
        let (_directory, git) = git_repo();
        let branch = git.create_branch("feat/cas").unwrap();
        assert_eq!(branch.branch, "feat/cas");
        assert_eq!(git.inspect().unwrap().branch.as_deref(), Some("feat/cas"));
    }

    #[test]
    fn commit_reports_new_head() {
        let (_directory, git) = git_repo();
        let commit = git.commit("first commit").unwrap();
        assert!((7..=12).contains(&commit.sha.len()));
        assert_eq!(
            git.inspect().unwrap().head.as_deref(),
            Some(commit.sha.as_str())
        );
    }

    #[test]
    fn tag_verifies_postcondition() {
        let (_directory, git) = git_repo();
        git.commit("initial").unwrap();
        let tag = git.tag("v0.1.0").unwrap();
        assert_eq!(tag.tag, "v0.1.0");
    }

    #[test]
    fn is_inside_work_tree_true_for_worktree() {
        let (_dir, git) = git_repo();
        assert!(git.is_inside_work_tree().unwrap());
    }

    #[test]
    fn is_inside_work_tree_true_for_linked_worktree_and_subdirectory() {
        let (_dir, git) = git_repo();
        git.commit("initial").unwrap();
        let linked_parent = tempfile::tempdir().unwrap();
        let linked = linked_parent.path().join("linked");
        let output = std::process::Command::new("git")
            .args(["-C"])
            .arg(git.root())
            .args(["worktree", "add", "--detach"])
            .arg(&linked)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let nested = linked.join("nested");
        fs::create_dir_all(&nested).unwrap();
        assert!(GitExecutor::new(linked).is_inside_work_tree().unwrap());
        assert!(GitExecutor::new(nested).is_inside_work_tree().unwrap());
    }

    #[test]
    fn is_inside_work_tree_false_for_bare_repository() {
        let directory = tempfile::tempdir().unwrap();
        let output = std::process::Command::new("git")
            .args(["init", "--bare", "-q"])
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let git = GitExecutor::new(directory.path().to_path_buf());
        assert!(!git.is_inside_work_tree().unwrap());
    }

    #[test]
    fn ls_files_returns_tracked_regular_files() {
        let (_dir, git) = git_repo();
        let agents = git.root().join("agents");
        fs::create_dir_all(&agents).unwrap();
        fs::write(agents.join("a.md"), "# Test\n").unwrap();
        std::process::Command::new("git")
            .args(["-C"])
            .arg(git.root())
            .args(["add", "agents/a.md"])
            .output()
            .unwrap();
        git.commit("add").unwrap();
        let files = git.ls_files(&["agents"]).unwrap();
        assert!(
            files
                .iter()
                .any(|p| p.to_string_lossy().contains("agents/a.md"))
        );
    }

    #[test]
    fn local_git_env_keys_includes_git_terminal_prompt() {
        assert!(
            LOCAL_GIT_ENV_KEYS.contains(&"GIT_TERMINAL_PROMPT"),
            "LOCAL_GIT_ENV_KEYS must contain GIT_TERMINAL_PROMPT"
        );
    }

    #[test]
    fn local_git_env_keys_excludes_gh_token() {
        assert!(
            !LOCAL_GIT_ENV_KEYS.contains(&"GH_TOKEN"),
            "LOCAL_GIT_ENV_KEYS must NOT contain GH_TOKEN"
        );
        assert!(
            !LOCAL_GIT_ENV_KEYS.contains(&"GITHUB_TOKEN"),
            "LOCAL_GIT_ENV_KEYS must NOT contain GITHUB_TOKEN"
        );
    }

    #[test]
    fn git_push_auth_failure_classifies_stderr() {
        use super::classify_auth_failure;

        // Positive cases — four markers that MUST classify as AuthFailed.
        let cases = [
            (
                "could not read Username",
                "could not read Username for 'https://github.com': terminal prompts disabled",
            ),
            (
                "terminal prompts disabled",
                "fatal: terminal prompts disabled",
            ),
            ("403 Forbidden", "remote: GitHub API error: 403 Forbidden"),
            ("Bad credentials", "remote: Bad credentials"),
            ("failed to authenticate", "error: failed to authenticate"),
            (
                "fatal: Authentication failed",
                "fatal: Authentication failed.",
            ),
        ];
        for (marker, stderr) in cases {
            let result = classify_auth_failure("push", stderr);
            let err = result.unwrap_or_else(|| {
                panic!("stderr containing '{marker}' should be classified as AuthFailed")
            });
            match err {
                GitError::AuthFailed { hint, .. } => {
                    assert!(
                        hint.contains("gh auth login"),
                        "AuthFailed hint must contain 'gh auth login'"
                    );
                }
                other => panic!("Expected AuthFailed for marker '{marker}', got {:?}", other),
            }
        }

        // Negative case — non-auth stderr must NOT be classified.
        let result = classify_auth_failure("push", "error: src refspec main does not match any");
        assert!(
            result.is_none(),
            "non-auth stderr should not be classified as AuthFailed"
        );
    }
}
