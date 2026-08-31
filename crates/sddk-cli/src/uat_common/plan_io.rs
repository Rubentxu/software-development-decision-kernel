//! Plan read/write helpers.

#![allow(dead_code)]

use sddk_domain::UatPlan;
use std::path::Path;

/// Read a UatPlan from a YAML file.
pub fn read_plan(path: &Path) -> anyhow::Result<UatPlan> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", path.display()))?;
    serde_saphyr::from_str(&content)
        .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", path.display()))
}

/// Write a UatPlan to a YAML file.
pub fn write_plan(plan: &UatPlan, path: &Path) -> anyhow::Result<()> {
    let yaml = serde_saphyr::to_string(plan).map_err(|e| anyhow::anyhow!("serialization: {e}"))?;
    std::fs::write(path, yaml)?;
    Ok(())
}

/// Atomic write: write to temp file in same directory, then rename.
/// This ensures no partial file exists on error.
pub fn atomic_write_plan(plan: &UatPlan, path: &Path) -> anyhow::Result<()> {
    use std::io::Write;

    let yaml = serde_saphyr::to_string(plan).map_err(|e| anyhow::anyhow!("serialization: {e}"))?;

    // Get parent directory
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?;

    // Create temp file in same directory
    let temp_path = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("plan"),
        std::process::id()
    ));

    // Write to temp file
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(yaml.as_bytes())?;
    file.sync_all()?;
    drop(file);

    // Atomic rename
    std::fs::rename(&temp_path, path)?;

    Ok(())
}
