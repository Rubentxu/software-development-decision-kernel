//! Identity records for projects and workspaces.
use crate::cycle::CycleManifest;

/// A logical SDDK project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    pub project_id: String,
    pub display_name: String,
    pub remote_url: Option<String>,
    pub scope: String,
    pub created_at: String,
}

/// A checkout or worktree belonging to a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub workspace_id: String,
    pub project_id: String,
    pub canonical_path: String,
    pub created_at: String,
}

/// A persisted cycle manifest and its storage timestamps.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleRecord {
    pub manifest: CycleManifest,
    pub created_at: String,
    pub updated_at: String,
}
