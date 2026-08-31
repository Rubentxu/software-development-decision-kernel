//! Release channels (Phase 9).
//!
//! Channel metadata and promotion rules: Dev → Edge → Candidate → Stable.
//! Promotion to Candidate/Stable requires gates to pass (policy-driven).

use serde::{Deserialize, Serialize};

/// Release channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    /// Stable — production releases with full gate evidence.
    Stable,
    /// Candidate — release candidates awaiting final gates.
    Candidate,
    /// Edge — nightly/experimental builds.
    Edge,
    /// Dev — local development builds.
    Dev,
}

crate::assert_variant_count_eq!(
    ReleaseChannel,
    4,
    [
        ReleaseChannel::Stable,
        ReleaseChannel::Candidate,
        ReleaseChannel::Edge,
        ReleaseChannel::Dev,
    ]
);

impl ReleaseChannel {
    /// Parses a channel from its lowercase name.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "stable" => Some(Self::Stable),
            "candidate" => Some(Self::Candidate),
            "edge" => Some(Self::Edge),
            "dev" => Some(Self::Dev),
            _ => None,
        }
    }
}

/// Metadata for one channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelMetadata {
    /// Channel name.
    pub name: String,
    /// Human description.
    pub description: String,
    /// Gate ids required to promote INTO this channel.
    pub promotion_requires: Vec<String>,
}

/// Returns metadata for all four channels.
pub fn channel_metadata() -> Vec<ChannelMetadata> {
    vec![
        ChannelMetadata {
            name: "stable".into(),
            description: "Production releases with full gate evidence".into(),
            promotion_requires: vec![
                "release-staleness-approved".into(),
                "release-uat-approved".into(),
                "release-verified".into(),
            ],
        },
        ChannelMetadata {
            name: "candidate".into(),
            description: "Release candidates awaiting final gates".into(),
            promotion_requires: vec!["release-verified".into()],
        },
        ChannelMetadata {
            name: "edge".into(),
            description: "Nightly/experimental builds".into(),
            promotion_requires: vec![],
        },
        ChannelMetadata {
            name: "dev".into(),
            description: "Local development builds".into(),
            promotion_requires: vec![],
        },
    ]
}

/// Looks up metadata for one channel.
pub fn metadata_for(channel: ReleaseChannel) -> ChannelMetadata {
    channel_metadata()
        .into_iter()
        .find(|m| m.name == channel_name(channel))
        .expect("builtin channel metadata")
}

/// Channel name in lowercase.
pub fn channel_name(channel: ReleaseChannel) -> &'static str {
    match channel {
        ReleaseChannel::Stable => "stable",
        ReleaseChannel::Candidate => "candidate",
        ReleaseChannel::Edge => "edge",
        ReleaseChannel::Dev => "dev",
    }
}

/// Returns the next promotion target, if any (Dev→Edge→Candidate→Stable).
pub fn promotion_target(channel: ReleaseChannel) -> Option<ReleaseChannel> {
    match channel {
        ReleaseChannel::Dev => Some(ReleaseChannel::Edge),
        ReleaseChannel::Edge => Some(ReleaseChannel::Candidate),
        ReleaseChannel::Candidate => Some(ReleaseChannel::Stable),
        ReleaseChannel::Stable => None,
    }
}

/// Whether promotion from `from` to `to` is allowed.
///
/// Dev→Edge is free; Edge→Candidate and Candidate→Stable require `gates_ok`.
pub fn can_promote(from: ReleaseChannel, to: ReleaseChannel, gates_ok: bool) -> bool {
    if promotion_target(from) != Some(to) {
        return false;
    }
    match to {
        ReleaseChannel::Stable | ReleaseChannel::Candidate => gates_ok,
        ReleaseChannel::Edge => true,
        ReleaseChannel::Dev => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_has_four_channels() {
        let metadata = channel_metadata();
        assert_eq!(metadata.len(), 4);
        let names: Vec<&str> = metadata.iter().map(|m| m.name.as_str()).collect();
        for name in ["stable", "candidate", "edge", "dev"] {
            assert!(names.contains(&name), "missing channel {name}");
        }
    }

    #[test]
    fn edge_promotes_to_candidate_with_gates() {
        assert!(can_promote(
            ReleaseChannel::Edge,
            ReleaseChannel::Candidate,
            true
        ));
        assert!(!can_promote(
            ReleaseChannel::Edge,
            ReleaseChannel::Candidate,
            false
        ));
    }

    #[test]
    fn candidate_blocked_to_stable_without_gates() {
        assert!(!can_promote(
            ReleaseChannel::Candidate,
            ReleaseChannel::Stable,
            false
        ));
        assert!(can_promote(
            ReleaseChannel::Candidate,
            ReleaseChannel::Stable,
            true
        ));
    }

    #[test]
    fn dev_promotes_to_edge_freely() {
        assert!(can_promote(
            ReleaseChannel::Dev,
            ReleaseChannel::Edge,
            false
        ));
    }

    #[test]
    fn non_adjacent_promotion_rejected() {
        assert!(!can_promote(
            ReleaseChannel::Dev,
            ReleaseChannel::Stable,
            true
        ));
        assert!(!can_promote(
            ReleaseChannel::Stable,
            ReleaseChannel::Dev,
            true
        ));
    }

    #[test]
    fn parse_roundtrip() {
        assert_eq!(
            ReleaseChannel::parse("stable"),
            Some(ReleaseChannel::Stable)
        );
        assert_eq!(ReleaseChannel::parse("nope"), None);
        assert_eq!(channel_name(ReleaseChannel::Candidate), "candidate");
    }
}
