// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/registry.rs — A4-4b: deterministic lens registry.
//
// ## Why duplicate `LensId` is **refused**, not replaced
//
// A4-2's `debverify_kernel::ChallengeStrategySet::register` REPLACES
// on duplicate name. A4-4b deliberately diverges from that policy:
// `AlignmentLensRegistry::register` returns
// `Err(LensError::DuplicateLensId(...))` on the second registration of
// the same `LensId`. A re-registration with a DIFFERENT `LensVersion`
// returns `LensError::InconsistentLensVersion`.
//
// Rationale (recorded; A4-4b spec §3.X + pin 1):
// - Replacement silently mutates the registry; the previous lens was
//   exercised against an earlier argument set and may have produced
//   contributions already in flight. The kernel's determinism
//   contract is "two contributions sharing `(lens_id, lens_version,
//   concern, observations, evidence_resolution)` collide". Letting a
//   silently-replaced lens continue producing contributions while a
//   newer version is also in flight violates that contract.
//
// - First-registration-wins makes the version of a `LensId` a
//   contract: bumping the version REQUIRES a new `LensId` (or
//   `AlignmentLensRegistry::replace_under_explicit_lease`, a future
//   post-A4-4b addition that does NOT exist yet). This is the
//   "LensVersion is in identity" pin (id.rs test
//   `lens_version_is_in_identity`).
//
// ## Determinism contract
//
// - Primary index: `BTreeMap<LensId, Arc<dyn AlignmentLens>>`. Stored
//   in canonical (sorted-by-LensId) order.
// - Secondary index: `BTreeMap<UniversalConcern, BTreeSet<LensId>>`.
//   `for_concern(c)` returns a `BTreeSet<LensId>` snapshot; iteration
//   is in canonical order regardless of registration order.
// - `iter()` returns descriptors in canonical `(LensId, LensVersion)`
//   order.
// - `len()`, `is_empty()`, `contains(LensId)` are all `O(log n)` and
//   deterministic.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::error::LensError;
use super::lens::AlignmentLens;
use super::types::{LensDescriptor, LensId};
use crate::intent_universal_concern::UniversalConcern;

/// The deterministic registry of [`AlignmentLens`] implementations.
///
/// Concrete construction: `AlignmentLensRegistry::new()` for an empty
/// registry, or `AlignmentLensRegistry::default()`. A4-4b ships with
/// **no** built-in lens — that is A4-4M's surface.
#[derive(Default)]
pub struct AlignmentLensRegistry {
    primary: BTreeMap<LensId, Arc<dyn AlignmentLens>>,
    /// Mirror of `supported_concerns` per lens id. Maintained for
    /// fast `for_concern(c)` lookups. Recomputed per registration.
    by_concern: BTreeMap<UniversalConcern, BTreeSet<LensId>>,
}

impl std::fmt::Debug for AlignmentLensRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let descriptors: Vec<LensDescriptor> = self.descriptors_owned();
        f.debug_struct("AlignmentLensRegistry")
            .field("lens_count", &descriptors.len())
            .field("lenses", &descriptors)
            .finish()
    }
}

impl Clone for AlignmentLensRegistry {
    /// `Arc<dyn AlignmentLens>` is cheaply cloned; the secondary index
    /// rebuilds cheaply. We rebuild the `BTreeMap` from the primary.
    fn clone(&self) -> Self {
        let primary = self.primary.clone();
        let mut by_concern: BTreeMap<UniversalConcern, BTreeSet<LensId>> = BTreeMap::new();
        for (id, lens) in primary.iter() {
            for c in &lens.descriptor().supported_concerns {
                by_concern.entry(*c).or_default().insert(*id);
            }
        }
        Self {
            primary,
            by_concern,
        }
    }
}

impl AlignmentLensRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a lens. Refuses duplicates; see module docs.
    pub fn register<L>(mut self, lens: L) -> Result<Self, LensError>
    where
        L: AlignmentLens + 'static,
    {
        let descriptor = lens.descriptor();
        let id = descriptor.id;
        let version = descriptor.version;
        let supported_concerns_len = descriptor.supported_concerns.len();
        // Duplicate id: refuse. We DO NOT replace. See module header.
        if let Some(existing) = self.primary.get(&id) {
            let existing_version = existing.descriptor().version;
            // Idempotent re-registration only succeeds when EVERY field
            // of the descriptor equals the existing registration
            // (id, version, supported_concerns as a set — same length
            // AND same content). Any divergence is refused.
            if existing_version == version {
                let existing_descriptor = existing.descriptor();
                if existing_descriptor.supported_concerns == descriptor.supported_concerns {
                    return Ok(self); // idempotent
                }
                return Err(LensError::InconsistentSupportedConcerns {
                    id,
                    first: existing_descriptor.supported_concerns.len() as u32,
                    subsequent: supported_concerns_len as u32,
                });
            }
            return Err(LensError::InconsistentLensVersion {
                id,
                first: existing_version,
                subsequent: version,
            });
        }
        // Update secondary index.
        for c in &descriptor.supported_concerns {
            self.by_concern.entry(*c).or_default().insert(id);
        }
        self.primary.insert(id, Arc::new(lens));
        Ok(self)
    }

    /// Number of registered lenses.
    pub fn len(&self) -> usize {
        self.primary.len()
    }

    /// True iff no lenses are registered.
    pub fn is_empty(&self) -> bool {
        self.primary.is_empty()
    }

    /// Every registered `LensDescriptor`, collected (clones each
    /// descriptor). Returns a `Vec<LensDescriptor>` in canonical
    /// `(LensId, LensVersion)` order.
    pub fn descriptors_owned(&self) -> Vec<LensDescriptor> {
        self.primary.values().map(|l| l.descriptor()).collect()
    }

    /// Lens ids, in canonical order (deterministic, no allocation per
    /// call beyond a `Vec` of `LensId`s, which are copy).
    pub fn ids(&self) -> Vec<LensId> {
        self.primary.keys().copied().collect()
    }

    /// Look up a lens by `LensId`.
    pub fn lookup(&self, id: LensId) -> Option<&Arc<dyn AlignmentLens>> {
        self.primary.get(&id)
    }

    /// True iff a lens with this id is registered.
    pub fn contains(&self, id: LensId) -> bool {
        self.primary.contains_key(&id)
    }

    /// Every registered lens whose `descriptor.supported_concerns`
    /// contains `concern`, in canonical `(LensId, LensVersion)` order.
    ///
    /// The kernel evaluates **all** returned lenses; the kernel does
    /// NOT pick a winner. `NotEvaluated` is produced **only** when
    /// this set is empty (or when every lens in it refuses).
    pub fn for_concern(&self, concern: UniversalConcern) -> Vec<LensId> {
        self.by_concern
            .get(&concern)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// True iff at least one lens is registered for `concern`.
    pub fn has_concern(&self, concern: UniversalConcern) -> bool {
        self.by_concern.contains_key(&concern)
    }

    /// Every `(UniversalConcern, [LensId])` group, in canonical
    /// `UniversalConcern` order. Useful for diagnostic output.
    pub fn concern_index(&self) -> impl Iterator<Item = (UniversalConcern, Vec<LensId>)> + '_ {
        // Sort by canonical `UniversalConcern` order via the closed
        // enum's `Ord`.
        let mut v: Vec<(UniversalConcern, Vec<LensId>)> = self
            .by_concern
            .iter()
            .map(|(c, ids)| (*c, ids.iter().copied().collect()))
            .collect();
        v.sort_by_key(|(c, _)| *c);
        v.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The unit tests for registry reside in `tests.rs` (the kernel's
    // integration test fixture drives them with real lenses). This
    // stub is here only to surface compile errors here early.

    #[test]
    fn empty_registry_reports_zero_lenses() {
        let r = AlignmentLensRegistry::new();
        assert_eq!(r.len(), 0);
        assert!(r.is_empty());
        assert!(r.descriptors_owned().is_empty());
        assert!(
            r.for_concern(crate::intent_universal_concern::UniversalConcern::Freshness)
                .is_empty()
        );
    }
}
