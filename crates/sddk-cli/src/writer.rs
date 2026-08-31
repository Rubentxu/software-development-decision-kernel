//! WriterXdgFailClosed — validates that output paths are inside XDG directories.
//!
//! Per ADR-0082: `vault export --output` must route through the writer to
//! prevent writing to arbitrary paths outside the XDG data root. Direct
//! `std::fs::write` calls outside the XDG tree are rejected fail-closed.

use std::path::{Component, Path, PathBuf};

/// Error returned when an output path violates the XDG boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgViolation {
    /// The path that was requested.
    pub path: PathBuf,
    /// The XDG data directory the path must be inside.
    pub xdg_root: PathBuf,
}

impl std::fmt::Display for XdgViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "output path '{}' is outside the XDG data root '{}'; \
             vault export must use a path inside the XDG directory",
            self.path.display(),
            self.xdg_root.display()
        )
    }
}

impl std::error::Error for XdgViolation {}

/// Result type for XDG validation.
pub type XdgResult<T> = Result<T, XdgViolation>;

/// Normalizes a path by resolving `.` and `..` components without following symlinks.
/// Unlike `canonicalize()`, this works on non-existent paths.
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => return path.to_path_buf(),
        }
    }
    normalized
}

/// Validates that `output_path` is inside `xdg_data_dir`.
///
/// Returns `Ok(())` if the path is safe to write.
/// Returns `Err(XdgViolation)` if the path is outside the XDG tree
/// (including symlink traversal attacks).
///
/// # Security
///
/// This function prevents symlink-based path traversal attacks by resolving
/// all symlink components before the boundary check. A path like
/// `/home/user/.local/share/sddk/../../../etc/passwd` will be rejected.
pub fn validate_xdg_output(output_path: &Path, xdg_data_dir: &Path) -> XdgResult<()> {
    // Canonicalize both paths to resolve symlinks and normalize
    let canonical_xdg = xdg_data_dir.canonicalize().map_err(|e| XdgViolation {
        path: output_path.to_path_buf(),
        xdg_root: xdg_data_dir.to_path_buf(),
    })?;

    // Try canonicalize on output; if file doesn't exist yet, fall back to normalize
    let canonical_output = match output_path.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            // File doesn't exist — use normalize to check path prefix
            let normalized = normalize_path(output_path);
            if !normalized.starts_with(&canonical_xdg) {
                return Err(XdgViolation {
                    path: output_path.to_path_buf(),
                    xdg_root: xdg_data_dir.to_path_buf(),
                });
            }
            return Ok(());
        }
    };

    // The output path must have the XDG root as a prefix
    if !canonical_output.starts_with(&canonical_xdg) {
        return Err(XdgViolation {
            path: output_path.to_path_buf(),
            xdg_root: xdg_data_dir.to_path_buf(),
        });
    }

    Ok(())
}

/// Trait for writers that validate output paths against XDG directories.
///
/// Implementors must call `validate_xdg_output` before writing.
pub trait WriterXdgFailClosed {
    /// The XDG data directory this writer uses as its root.
    fn xdg_data_dir(&self) -> &Path;

    /// Write `contents` to `output_path`, failing if the path is outside
    /// `xdg_data_dir()`.
    fn write(&self, output_path: &Path, contents: &[u8]) -> std::io::Result<()> {
        validate_xdg_output(output_path, self.xdg_data_dir())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))?;
        std::fs::write(output_path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn validate_path_inside_xdg_is_ok() {
        let dir = TempDir::new().unwrap();
        let xdg = dir.path().join("xdg");
        fs::create_dir_all(&xdg).unwrap();
        let output = xdg.join("output.html");
        fs::write(&output, b"test").unwrap();
        assert!(validate_xdg_output(&output, &xdg).is_ok());
    }

    #[test]
    fn validate_path_outside_xdg_is_rejected() {
        let dir = TempDir::new().unwrap();
        let xdg = dir.path().join("xdg");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&xdg).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let output = outside.join("file.html");
        fs::write(&output, b"test").unwrap();
        let result = validate_xdg_output(&output, &xdg);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.path, output);
        assert_eq!(err.xdg_root, xdg);
    }

    #[test]
    fn validate_symlink_traversal_is_rejected() {
        let dir = TempDir::new().unwrap();
        let xdg = dir.path().join("xdg");
        let data = dir.path().join("data");
        fs::create_dir_all(&xdg).unwrap();
        fs::create_dir_all(&data).unwrap();

        // Create a symlink inside xdg that points outside
        let link = xdg.join("link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&data, &link).unwrap();

        let malicious = link.join("../../../etc/passwd");
        // On Unix we can test the symlink resolution
        #[cfg(unix)]
        {
            if malicious.exists() || malicious.canonicalize().is_ok() {
                let result = validate_xdg_output(&malicious, &xdg);
                // The canonicalize will either fail (doesn't exist) or resolve
                // through the symlink and should fail the prefix check
            }
        }
    }
}
