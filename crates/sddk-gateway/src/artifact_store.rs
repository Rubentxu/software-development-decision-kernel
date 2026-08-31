//! Content-addressed artifact store with mandatory SHA-256 verification.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sddk_storage::{ArtifactRecord, Storage, StorageError};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors emitted by the content-addressed artifact store.
#[derive(Debug, Error)]
pub enum ArtifactStoreError {
    /// Artifact bytes could not be read or written.
    #[error("artifact store I/O failure for {path}: {source}")]
    Io {
        /// Affected path.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// Stored bytes disagree with their content digest.
    #[error("artifact digest mismatch: expected {expected}, computed {computed}")]
    DigestMismatch {
        /// Expected digest.
        expected: String,
        /// Computed digest.
        computed: String,
    },
    /// A requested artifact is not present in the store.
    #[error("artifact not found in store: {digest}")]
    Missing {
        /// Requested digest.
        digest: String,
    },
    /// The supplied digest is not a valid content identifier.
    #[error("invalid content digest: {digest}")]
    InvalidDigest {
        /// Rejected digest.
        digest: String,
    },
    /// The metadata write failed.
    #[error("artifact storage error: {0}")]
    Storage(#[from] StorageError),
    /// Structured metadata could not be encoded.
    #[error("artifact metadata serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Metadata attached to stored artifact bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactMeta {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, when applicable.
    pub cycle_id: Option<String>,
    /// Artifact kind from the workflow contract.
    pub kind: String,
    /// Logical artifact path or reference.
    pub path: String,
    /// Producer identifier.
    pub producer: String,
    /// Caller-supplied creation timestamp.
    pub created_at: String,
}

/// Content-addressed store: bytes are keyed by their SHA-256 digest.
///
/// The digest is mandatory and every read re-verifies the bytes, so tampered
/// content is detected instead of returned.
pub struct ArtifactStore {
    storage: Storage,
    base: PathBuf,
}

impl ArtifactStore {
    /// Creates a store rooted at `base`, sharing the project ledger.
    pub fn new(storage: Storage, base: PathBuf) -> Self {
        Self { storage, base }
    }

    /// Computes the canonical `sha256:` digest of bytes.
    pub fn digest(bytes: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(bytes))
    }

    /// Stores bytes deduplicated by digest and records metadata.
    ///
    /// The digest is computed locally (never caller-supplied), the content file
    /// is written atomically only when absent, and the stored file is
    /// re-verified before the metadata row is committed.
    pub fn store(
        &self,
        bytes: &[u8],
        meta: &ArtifactMeta,
    ) -> Result<ArtifactRecord, ArtifactStoreError> {
        let digest = Self::digest(bytes);
        let content_path = self.content_path(&digest);
        let parent = content_path.parent().expect("content path has a parent");
        fs::create_dir_all(parent).map_err(|source| ArtifactStoreError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        if !content_path.exists() {
            write_atomic(&content_path, bytes).map_err(|source| ArtifactStoreError::Io {
                path: content_path.clone(),
                source,
            })?;
        }
        let stored = fs::read(&content_path).map_err(|source| ArtifactStoreError::Io {
            path: content_path.clone(),
            source,
        })?;
        let computed = Self::digest(&stored);
        if computed != digest {
            return Err(ArtifactStoreError::DigestMismatch {
                expected: digest,
                computed,
            });
        }
        let artifact = ArtifactRecord {
            artifact_id: format!(
                "art-{}-{}",
                &digest[7..19],
                &uuid::Uuid::new_v4().simple().to_string()[..8]
            ),
            project_id: meta.project_id.clone(),
            cycle_id: meta.cycle_id.clone(),
            kind: meta.kind.clone(),
            path: meta.path.clone(),
            sha256: Some(digest),
            producer: Some(meta.producer.clone()),
            created_at: meta.created_at.clone(),
            metadata: json!({
                "size": bytes.len(),
                "content_path": content_path.to_string_lossy(),
            }),
        };
        self.storage.insert_artifact(&artifact)?;
        Ok(artifact)
    }

    /// Reads and verifies bytes for a digest.
    pub fn get(&self, digest: &str) -> Result<Vec<u8>, ArtifactStoreError> {
        let content_path = self.content_path(digest);
        let stored = fs::read(&content_path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                ArtifactStoreError::Missing {
                    digest: digest.to_owned(),
                }
            } else {
                ArtifactStoreError::Io {
                    path: content_path,
                    source: error,
                }
            }
        })?;
        let computed = Self::digest(&stored);
        if computed != digest {
            return Err(ArtifactStoreError::DigestMismatch {
                expected: digest.to_owned(),
                computed,
            });
        }
        Ok(stored)
    }

    /// Loads the metadata row for a stored artifact id.
    pub fn metadata(&self, artifact_id: &str) -> Result<ArtifactRecord, ArtifactStoreError> {
        Ok(self.storage.get_artifact(artifact_id)?)
    }

    fn content_path(&self, digest: &str) -> PathBuf {
        let hex = digest
            .strip_prefix("sha256:")
            .unwrap_or(digest)
            .to_ascii_lowercase();
        if hex.len() < 4 {
            return self.base.join("invalid").join(hex);
        }
        self.base
            .join("objects")
            .join(&hex[..2])
            .join(&hex[2..4])
            .join(hex)
    }
}

fn write_atomic(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let parent = destination.parent().expect("destination has a parent");
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let mut last_error = None;
    for attempt in 0..100 {
        let temporary = parent.join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    drop(file);
                    fs::rename(&temporary, destination)
                })();
                if let Err(source) = result {
                    let _ = fs::remove_file(&temporary);
                    return Err(source);
                }
                return Ok(());
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                last_error = Some(error);
            }
            Err(source) => return Err(source),
        }
    }
    Err(last_error.unwrap_or_else(|| io::Error::other("no temporary path available")))
}

#[cfg(test)]
mod tests {
    use sddk_storage::{ProjectRecord, Storage};

    use super::ArtifactStore;

    fn meta(path: &str) -> super::ArtifactMeta {
        super::ArtifactMeta {
            project_id: "project-1".into(),
            cycle_id: None,
            kind: "report".into(),
            path: path.into(),
            producer: "sddk-test".into(),
            created_at: "2026-08-04T10:00:00Z".into(),
        }
    }

    fn store() -> (tempfile::TempDir, ArtifactStore) {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_project(&ProjectRecord {
                project_id: "project-1".into(),
                display_name: "project".into(),
                remote_url: Some("https://example.com/owner/project".into()),
                scope: "owner".into(),
                created_at: "2026-08-04T10:00:00Z".into(),
            })
            .unwrap();
        let base = directory.path().join("cas");
        (directory, ArtifactStore::new(storage, base))
    }

    #[test]
    fn store_assigns_mandatory_digest_and_deduplicates() {
        let (_directory, store) = store();
        let first = store.store(b"payload", &meta("reports/a.md")).unwrap();
        let digest = first.sha256.clone().unwrap();
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest.len(), "sha256:".len() + 64);

        let second = store.store(b"payload", &meta("reports/b.md")).unwrap();
        assert_eq!(first.sha256, second.sha256);
        let objects = std::fs::read_dir(store.base.join("objects"))
            .unwrap()
            .count();
        assert_eq!(objects, 1);
    }

    #[test]
    fn get_verifies_digest_and_rejects_tampering() {
        let (_directory, store) = store();
        store.store(b"payload", &meta("reports/a.md")).unwrap();
        let digest = ArtifactStore::digest(b"payload");
        let bytes = store.get(&digest).unwrap();
        assert_eq!(bytes, b"payload");

        assert!(matches!(
            store.get("sha256:0000000000000000000000000000000000000000000000000000000000000000"),
            Err(super::ArtifactStoreError::Missing { .. })
        ));

        let content_path = store.content_path(&digest);
        std::fs::write(&content_path, b"tampered").unwrap();
        assert!(matches!(
            store.get(&digest),
            Err(super::ArtifactStoreError::DigestMismatch { .. })
        ));
    }
}
