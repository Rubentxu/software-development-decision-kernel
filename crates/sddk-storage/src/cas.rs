//! Filesystem-based Content-Addressable Storage (CAS) for planning evidence and decisions.
//!
//! Default CAS root: `~/.local/share/sddk/cas/`
//! Layout: `<root>/<sha[0:2]>/<sha[2:4]>/<sha>.json`
//!
//! Bodies are stored as JSON files addressed by SHA-256 content hash.

use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

use sddk_domain::planning::CasHash;
use sddk_domain::ports::{CasError, CasPort};

/// Default CAS root directory name within the SDDK data directory.
pub const DEFAULT_CAS_ROOT: &str = "cas";

/// Filesystem CAS implementation.
///
/// Stores immutable JSON bodies in a hierarchical directory structure:
/// `<root>/<sha[0:2]>/<sha[2:4]>/<sha>.json`
///
/// The hash is computed over the raw bytes before storage, and the file
/// name is the hex-encoded SHA-256 of those bytes.
#[derive(Debug, Clone)]
pub struct FilesystemCas {
    root: PathBuf,
}

impl FilesystemCas {
    /// Creates a new FilesystemCas with the given root directory.
    ///
    /// The root is created if it does not exist.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Returns the default CAS root path.
    ///
    /// Computed from `$XDG_DATA_HOME/sddk/cas` or `$HOME/.local/share/sddk/cas`.
    pub fn default_root() -> PathBuf {
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from(".local/share"))
                    .join(".local/share")
                    .join("sddk")
            });
        base.join(DEFAULT_CAS_ROOT)
    }

    /// Returns the path where a given CAS hash would be stored.
    fn hash_path(&self, hash: &str) -> PathBuf {
        let hash = hash.strip_prefix("sha256:").unwrap_or(hash);
        let hash = hash.trim();
        let p1 = &hash[..2.min(hash.len())];
        let p2 = &hash[2..4.min(hash.len())];
        self.root.join(p1).join(p2).join(hash)
    }

    /// Computes the CAS hash for content.
    fn compute_hash(content: &[u8]) -> String {
        let digest = Sha256::digest(content);
        format!("{:x}", digest)
    }
}

impl CasPort for FilesystemCas {
    fn put(&self, content: &[u8]) -> Result<String, CasError> {
        let hash = Self::compute_hash(content);
        let path = self.hash_path(&hash);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CasError::Storage(e.to_string()))?;
        }

        // Write the file (idempotent — skip if already exists with same content)
        if path.exists() {
            let existing = fs::read(&path).map_err(|e| CasError::Storage(e.to_string()))?;
            if existing == content {
                return Ok(format!("sha256:{}", hash));
            }
            // Hash collision is astronomically unlikely but would indicate corruption
            return Err(CasError::HashMismatch {
                expected: format!("sha256:{}", hash),
                computed: format!("sha256:{}", Self::compute_hash(&existing)),
            });
        }

        fs::write(&path, content).map_err(|e| CasError::Storage(e.to_string()))?;
        Ok(format!("sha256:{}", hash))
    }

    fn get(&self, hash: &CasHash) -> Result<Vec<u8>, CasError> {
        let hash_str = hash.strip_prefix("sha256:").unwrap_or(hash.as_str()).trim();
        let path = self.hash_path(hash_str);

        fs::read(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => CasError::NotFound(hash.clone()),
            _ => CasError::Storage(e.to_string()),
        })
    }

    fn exists(&self, hash: &CasHash) -> Result<bool, CasError> {
        let hash_str = hash.strip_prefix("sha256:").unwrap_or(hash.as_str()).trim();
        let path = self.hash_path(hash_str);
        Ok(path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cas() -> (FilesystemCas, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        let cas = FilesystemCas::new(temp.path()).unwrap();
        (cas, temp)
    }

    #[test]
    fn put_and_get_round_trip() {
        let (cas, _dir) = temp_cas();
        let content = b"{\"test\": true}";
        let hash = cas.put(content).unwrap();
        let retrieved = cas.get(&hash).unwrap();
        assert_eq!(retrieved, content);
    }

    #[test]
    fn put_idempotent() {
        let (cas, _dir) = temp_cas();
        let content = b"{\"idempotent\": true}";
        let hash1 = cas.put(content).unwrap();
        let hash2 = cas.put(content).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn exists_after_put() {
        let (cas, _dir) = temp_cas();
        let content = b"test content";
        let hash = cas.put(content).unwrap();
        assert!(cas.exists(&hash).unwrap());
        assert!(
            !cas.exists(
                &"sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string()
            )
            .unwrap()
        );
    }

    #[test]
    fn get_not_found() {
        let (cas, _dir) = temp_cas();
        let result = cas.get(
            &"sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CasError::NotFound(_)));
    }

    #[test]
    fn default_root_uses_xdg() {
        // Without XDG_DATA_HOME set, should fall back to ~/.local/share/sddk/cas
        let root = FilesystemCas::default_root();
        assert!(root.to_str().unwrap().contains("sddk"));
        assert!(root.to_str().unwrap().ends_with("cas"));
    }
}
