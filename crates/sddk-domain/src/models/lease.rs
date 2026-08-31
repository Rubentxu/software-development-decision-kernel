//! An exclusive cycle lease with a monotonic fencing token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleLease {
    pub cycle_id: String,
    pub owner: String,
    pub acquired_at_ms: i64,
    pub expires_at_ms: i64,
    pub fencing_token: i64,
}
