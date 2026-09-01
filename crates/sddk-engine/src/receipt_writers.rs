//! Receipt writing utilities for SDDK engine.
//!
//! Provides atomic receipt writing for cycle lifecycle events (supersede, replan).

use std::io;
use std::path::Path;

/// Atomically writes `bytes` to `destination` using rename-overwrite.
/// Creates parent directories if needed.
pub fn write_atomic(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let parent = destination.parent().expect("destination has a parent");
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let mut last_error = None;
    for attempt in 0..100 {
        let temporary = parent.join(format!(
            ".{file_name}.tmp-{}-{}",
            std::process::id(),
            attempt
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    drop(file);
                    std::fs::rename(&temporary, destination)
                })();
                if let Err(source) = result {
                    let _ = std::fs::remove_file(&temporary);
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
