//! Stable fingerprint generator for debt findings.
//!
//! Computes a deterministic hex hash from `(file, line, rule, code_block)` tuples.
//! Output matches the schema regex `^[a-f0-9]{16,64}$`:
//! - Default `Sha256_64`: 16 hex chars (8 bytes / 64 bits)
//! - `Sha256_128`: 32 hex chars (16 bytes / 128 bits)

use sha2::{Digest, Sha256};

/// Fingerprint hash strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FingerprintStrategy {
    /// 16 hex chars (SHA256 truncated to first 8 bytes).
    Sha256_64,
    /// 32 hex chars (SHA256 truncated to first 16 bytes).
    Sha256_128,
}

/// Computes a fingerprint for the given source location and rule.
///
/// Default strategy [`FingerprintStrategy::Sha256_64`] produces 16 hex chars.
pub fn fingerprint(file: &str, line: u32, rule: &str, code_block: &str) -> String {
    fingerprint_with_strategy(file, line, rule, code_block, FingerprintStrategy::Sha256_64)
}

/// Computes a fingerprint using the given strategy.
pub fn fingerprint_with_strategy(
    file: &str,
    line: u32,
    rule: &str,
    code_block: &str,
    strategy: FingerprintStrategy,
) -> String {
    let input = format!("{file}:{line}:{rule}:{code_block}");
    let digest = Sha256::digest(input.as_bytes());
    match strategy {
        FingerprintStrategy::Sha256_64 => {
            // Full digest then truncate to 16 hex chars (8 bytes)
            let hex = format!("{:x}", digest);
            hex[..16].to_string()
        }
        FingerprintStrategy::Sha256_128 => {
            // Full digest then truncate to 32 hex chars (16 bytes)
            let hex = format!("{:x}", digest);
            hex[..32].to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_determinism() {
        let a = fingerprint("src/lib.rs", 42, "RUST-001", "fn foo() {}");
        let b = fingerprint("src/lib.rs", 42, "RUST-001", "fn foo() {}");
        assert_eq!(a, b, "same inputs must produce same hash");
    }

    #[test]
    fn test_fingerprint_variability() {
        let base = fingerprint("src/lib.rs", 42, "RUST-001", "fn foo() {}");
        let changed_file = fingerprint("src/main.rs", 42, "RUST-001", "fn foo() {}");
        let changed_line = fingerprint("src/lib.rs", 99, "RUST-001", "fn foo() {}");
        let changed_rule = fingerprint("src/lib.rs", 42, "RUST-002", "fn foo() {}");
        let changed_code = fingerprint("src/lib.rs", 42, "RUST-001", "fn bar() {}");
        assert_ne!(base, changed_file, "file change must change hash");
        assert_ne!(base, changed_line, "line change must change hash");
        assert_ne!(base, changed_rule, "rule change must change hash");
        assert_ne!(base, changed_code, "code_block change must change hash");
    }

    #[test]
    fn test_fingerprint_default_length() {
        let fp = fingerprint("src/lib.rs", 10, "RUST-001", "x");
        assert_eq!(fp.len(), 16, "default Sha256_64 must produce 16 hex chars");
        assert!(
            fp.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "must be lowercase hex"
        );
    }

    #[test]
    fn test_fingerprint_strategy_128() {
        let fp = fingerprint_with_strategy(
            "src/lib.rs",
            10,
            "RUST-001",
            "x",
            FingerprintStrategy::Sha256_128,
        );
        assert_eq!(fp.len(), 32, "Sha256_128 must produce 32 hex chars");
        assert!(
            fp.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "must be lowercase hex"
        );
    }

    #[test]
    fn test_fingerprint_empty_code_block() {
        let fp = fingerprint("src/lib.rs", 10, "RUST-001", "");
        assert_eq!(fp.len(), 16, "empty code_block is valid");
        assert!(
            fp.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "must be lowercase hex"
        );
    }
}
