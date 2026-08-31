//! XDG storage directory resolution for event bus projects.

use std::path::PathBuf;

use sddk_domain::StorageError;

/// Returns the canonical XDG storage dir for a project:
/// `$XDG_STATE_HOME/sddk/projects/<id>/`.
pub fn project_storage_dir(project_id: &str) -> Result<PathBuf, StorageError> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("state")))
        .ok_or_else(|| StorageError::Other("cannot resolve XDG state dir".into()))?;
    Ok(base.join("sddk").join("projects").join(project_id))
}

/// Internal logic for `project_storage_dir`, parameterised so tests can
/// supply fake env values without unsafe env manipulation.
#[cfg(test)]
fn project_storage_dir_with(
    xdg_state_home: Option<&str>,
    home: Option<&str>,
    project_id: &str,
) -> Result<PathBuf, StorageError> {
    let base = xdg_state_home
        .map(PathBuf::from)
        .or_else(|| home.map(|h| PathBuf::from(h).join(".local").join("state")))
        .ok_or_else(|| StorageError::Other("cannot resolve XDG state dir".into()))?;
    Ok(base.join("sddk").join("projects").join(project_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_storage_dir_uses_xdg_state_home() {
        let dir = project_storage_dir_with(Some("/custom/xdg/state"), Some("/home/test"), "proj-1")
            .unwrap();
        assert_eq!(
            dir.to_str().unwrap(),
            "/custom/xdg/state/sddk/projects/proj-1"
        );
    }

    #[test]
    fn project_storage_dir_falls_back_to_home() {
        let dir = project_storage_dir_with(None, Some("/home/test"), "proj-2").unwrap();
        assert_eq!(
            dir.to_str().unwrap(),
            "/home/test/.local/state/sddk/projects/proj-2"
        );
    }
}
