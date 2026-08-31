//! Scoped filesystem access restricted to configured roots.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

/// Errors emitted by scoped filesystem operations.
#[derive(Debug, Error)]
pub enum FsError {
    /// A path escapes every configured root or cannot be resolved safely.
    #[error("path escapes the scoped filesystem roots: {path:?}")]
    Escape {
        /// Rejected path.
        path: PathBuf,
    },
    /// The path is absolute or contains parent components.
    #[error("path must be relative and without parent components: {path:?}")]
    UnsafePath {
        /// Rejected path.
        path: PathBuf,
    },
    /// A filesystem operation failed.
    #[error("filesystem operation failed for {path}: {source}")]
    Io {
        /// Affected path.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
}

/// Filesystem boundary limiting writes and reads to allowed roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedFs {
    roots: Vec<PathBuf>,
}

impl ScopedFs {
    /// Creates a scope over the supplied canonical roots.
    pub fn new(roots: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            roots: roots.into_iter().collect(),
        }
    }

    /// Returns the configured roots.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Resolves a relative path inside the first matching root.
    ///
    /// The path must be relative and must not traverse parents. Each existing
    /// prefix is canonicalized so symlink escapes are rejected, while missing
    /// leaves are resolved lexically below the canonical root.
    pub fn resolve(&self, relative: &Path) -> Result<PathBuf, FsError> {
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
        {
            return Err(FsError::UnsafePath {
                path: relative.to_path_buf(),
            });
        }
        for root in &self.roots {
            let canonical_root = fs::canonicalize(root).map_err(|source| FsError::Io {
                path: root.clone(),
                source,
            })?;
            let mut current = canonical_root.clone();
            let mut escaped = false;
            for component in relative.components() {
                current.push(component.as_os_str());
                if current.exists()
                    && let Ok(resolved) = fs::canonicalize(&current)
                {
                    if !resolved.starts_with(&canonical_root) {
                        escaped = true;
                        break;
                    }
                    current = resolved;
                }
            }
            if !escaped && current.starts_with(&canonical_root) {
                return Ok(current);
            }
        }
        Err(FsError::Escape {
            path: relative.to_path_buf(),
        })
    }

    /// Atomically writes bytes to a resolved relative path.
    pub fn write_atomic(&self, relative: &Path, bytes: &[u8]) -> Result<PathBuf, FsError> {
        let destination = self.resolve(relative)?;
        let parent = destination.parent().ok_or_else(|| FsError::UnsafePath {
            path: relative.to_path_buf(),
        })?;
        fs::create_dir_all(parent).map_err(|source| FsError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        let file_name = destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("artifact");
        let mut last_error = None;
        for attempt in 0..100 {
            let temporary =
                parent.join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(mut file) => {
                    let result = (|| {
                        use std::io::Write;
                        file.write_all(bytes)?;
                        file.sync_all()?;
                        drop(file);
                        fs::rename(&temporary, &destination)
                    })();
                    if let Err(source) = result {
                        let _ = fs::remove_file(&temporary);
                        return Err(FsError::Io {
                            path: destination.clone(),
                            source,
                        });
                    }
                    return Ok(destination);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    last_error = Some(error);
                }
                Err(source) => {
                    return Err(FsError::Io {
                        path: temporary,
                        source,
                    });
                }
            }
        }
        Err(FsError::Io {
            path: destination,
            source: last_error.unwrap_or_else(|| io::Error::other("no temporary path available")),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::ScopedFs;

    #[test]
    fn rejects_absolute_and_parent_paths() {
        let directory = tempfile::tempdir().unwrap();
        let fs = ScopedFs::new([directory.path().to_path_buf()]);
        assert!(fs.resolve(std::path::Path::new("/etc/passwd")).is_err());
        assert!(fs.resolve(std::path::Path::new("../escape")).is_err());
    }

    #[test]
    fn resolves_inside_configured_root() {
        let directory = tempfile::tempdir().unwrap();
        let fs = ScopedFs::new([directory.path().to_path_buf()]);
        let resolved = fs.resolve(std::path::Path::new("nested/file.txt")).unwrap();
        assert!(resolved.starts_with(directory.path()));
    }

    #[test]
    fn rejects_symlink_escape() {
        let outer = tempfile::tempdir().unwrap();
        let inner = tempfile::tempdir().unwrap();
        let scope = ScopedFs::new([inner.path().to_path_buf()]);
        let link = inner.path().join("escape");
        std::os::unix::fs::symlink(outer.path(), &link).unwrap();
        assert!(
            scope
                .resolve(std::path::Path::new("escape/file.txt"))
                .is_err()
        );
    }

    #[test]
    fn writes_atomically_without_temp_leftovers() {
        let directory = tempfile::tempdir().unwrap();
        let fs = ScopedFs::new([directory.path().to_path_buf()]);
        let destination = fs
            .write_atomic(std::path::Path::new("dir/artifact.txt"), b"payload")
            .unwrap();
        assert_eq!(fs::read_to_string(destination).unwrap(), "payload");
        assert!(
            fs::read_dir(directory.path().join("dir"))
                .unwrap()
                .all(|entry| {
                    !entry
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .contains(".tmp-")
                })
        );
    }
}
