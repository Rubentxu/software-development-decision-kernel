//! Dependency Resolution Service for planning work items.
//!
//! Pure domain service that resolves whether a WorkItem can transition based on
//! its dependency edges. Implements the locked DependencyEdgeKind semantics:
//! - `Blocks`: blocks any non-terminal transition of the destination when source is non-terminal
//! - `BlocksOnClosure`: blocks only terminal transitions of the destination when source is non-terminal
//!
//! This service is PURE — no I/O, no clock, no filesystem access.

use crate::planning::{DependencyEdgeKind, DependencyEdgeV1, WorkItemId, WorkItemStatus};

/// Errors from dependency resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyResolutionError {
    /// The edge has a self-loop (from_id == to_id).
    SelfLoop {
        /// The WorkItem that appears as both source and target.
        work_item_id: WorkItemId,
    },
    /// A cycle was detected in the Blocks subgraph.
    CycleDetected {
        /// The WorkItem IDs forming the cycle, starting and ending at the same node.
        work_item_ids: Vec<WorkItemId>,
    },
    /// The target WorkItem is blocked by a predecessor that is not in a terminal state.
    BlockedBy {
        /// The predecessor WorkItem blocking the transition.
        predecessor_id: WorkItemId,
        /// The current status of the predecessor.
        current_status: WorkItemStatus,
        /// The kind of dependency edge causing the block.
        kind: DependencyEdgeKind,
    },
    /// A predecessor referenced in an edge does not exist.
    UnknownPredecessor {
        /// The target WorkItem.
        work_item_id: WorkItemId,
        /// The predecessor WorkItem ID that was not found.
        predecessor_id: WorkItemId,
    },
}

impl std::fmt::Display for DependencyResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyResolutionError::SelfLoop { work_item_id } => {
                write!(f, "self-loop detected: {} references itself", work_item_id)
            }
            DependencyResolutionError::CycleDetected { work_item_ids } => {
                write!(f, "cycle detected: {:?}", work_item_ids)
            }
            DependencyResolutionError::BlockedBy { predecessor_id, current_status, kind } => {
                write!(
                    f,
                    "blocked by predecessor {} in status {:?} (kind: {:?})",
                    predecessor_id, current_status, kind
                )
            }
            DependencyResolutionError::UnknownPredecessor { work_item_id, predecessor_id } => {
                write!(
                    f,
                    "unknown predecessor {} for work item {}",
                    predecessor_id, work_item_id
                )
            }
        }
    }
}

impl std::error::Error for DependencyResolutionError {}

/// The dependency resolution service.
///
/// Pure domain service that determines whether a WorkItem can transition based on
/// its incoming dependency edges. Does not perform any I/O.
pub struct DependencyResolutionService;

impl DependencyResolutionService {
    /// Determines whether a WorkItem can transition to Active status.
    ///
    /// Checks all incoming `Blocks` edges. If any predecessor is in a non-terminal
    /// state (Draft, Active, Paused), the transition is blocked.
    /// `BlocksOnClosure` edges do not affect non-terminal transitions.
    ///
    /// # Arguments
    /// * `target` - The WorkItem being transitioned
    /// * `incoming_edges` - All edges where `edge.to_id == target.id`
    /// * `status_lookup` - A function to look up the current status of a WorkItem
    ///
    /// # Returns
    /// * `Ok(())` if the transition is allowed
    /// * `Err(DependencyResolutionError)` if blocked
    pub fn resolve_can_activate(
        target: &crate::planning::WorkItemV1,
        incoming_edges: &[DependencyEdgeV1],
        status_lookup: &impl Fn(&WorkItemId) -> Option<WorkItemStatus>,
    ) -> Result<(), DependencyResolutionError> {
        // Filter edges that point to our target
        let relevant_edges: Vec<&DependencyEdgeV1> = incoming_edges
            .iter()
            .filter(|e| e.to_id == target.id)
            .collect();

        // Check for self-loops
        for edge in &relevant_edges {
            if edge.is_self_loop() {
                return Err(DependencyResolutionError::SelfLoop {
                    work_item_id: target.id.clone(),
                });
            }
        }

        // Check Blocks edges
        for edge in &relevant_edges {
            if edge.kind == DependencyEdgeKind::Blocks {
                let predecessor_status = status_lookup(&edge.from_id).ok_or_else(|| {
                    DependencyResolutionError::UnknownPredecessor {
                        work_item_id: target.id.clone(),
                        predecessor_id: edge.from_id.clone(),
                    }
                })?;

                // Non-terminal predecessor blocks the transition
                if !predecessor_status.is_terminal() {
                    return Err(DependencyResolutionError::BlockedBy {
                        predecessor_id: edge.from_id.clone(),
                        current_status: predecessor_status,
                        kind: DependencyEdgeKind::Blocks,
                    });
                }
                // Terminal predecessor (Done/Superseded/Cancelled) → allow
            }
            // BlocksOnClosure does NOT affect non-terminal transitions
            // (it only applies when the target is transitioning to a terminal state)
        }

        Ok(())
    }

    /// Determines whether a WorkItem can transition to a terminal status.
    ///
    /// Checks all incoming edges. Both `Blocks` and `BlocksOnClosure` edges
    /// block the transition if their predecessor is in a non-terminal state.
    ///
    /// # Arguments
    /// * `target` - The WorkItem being transitioned
    /// * `to_status` - The terminal status being transitioned to (must be Done, Superseded, or Cancelled)
    /// * `incoming_edges` - All edges where `edge.to_id == target.id`
    /// * `status_lookup` - A function to look up the current status of a WorkItem
    ///
    /// # Returns
    /// * `Ok(())` if the transition is allowed
    /// * `Err(DependencyResolutionError)` if blocked
    pub fn resolve_can_terminalize(
        target: &crate::planning::WorkItemV1,
        to_status: WorkItemStatus,
        incoming_edges: &[DependencyEdgeV1],
        status_lookup: &impl Fn(&WorkItemId) -> Option<WorkItemStatus>,
    ) -> Result<(), DependencyResolutionError> {
        // Verify the target status is actually terminal
        if !to_status.is_terminal() {
            // This is a programming error, not a business logic error
            // We return Ok(()) here to not block non-terminal transitions
            // The state machine should prevent this from being called
            return Ok(());
        }

        // Filter edges that point to our target
        let relevant_edges: Vec<&DependencyEdgeV1> = incoming_edges
            .iter()
            .filter(|e| e.to_id == target.id)
            .collect();

        // Check for self-loops
        for edge in &relevant_edges {
            if edge.is_self_loop() {
                return Err(DependencyResolutionError::SelfLoop {
                    work_item_id: target.id.clone(),
                });
            }
        }

        // For terminal transitions, BOTH Blocks and BlocksOnClosure block
        // if the predecessor is non-terminal
        for edge in &relevant_edges {
            let predecessor_status = status_lookup(&edge.from_id).ok_or_else(|| {
                DependencyResolutionError::UnknownPredecessor {
                    work_item_id: target.id.clone(),
                    predecessor_id: edge.from_id.clone(),
                }
            })?;

            if !predecessor_status.is_terminal() {
                return Err(DependencyResolutionError::BlockedBy {
                    predecessor_id: edge.from_id.clone(),
                    current_status: predecessor_status,
                    kind: edge.kind,
                });
            }
            // Terminal predecessor → allow
        }

        Ok(())
    }

    /// Detects cycles in the Blocks subgraph using Kahn's algorithm.
    ///
    /// Only `Blocks` edges are considered for cycle detection, as `BlocksOnClosure`
    /// edges do not block non-terminal transitions and therefore cannot form
    /// blocking cycles.
    ///
    /// # Arguments
    /// * `edges` - All dependency edges in the graph
    ///
    /// # Returns
    /// * `Ok(())` if no cycles exist
    /// * `Err(DependencyResolutionError::CycleDetected)` if a cycle is found
    pub fn detect_cycles(edges: &[DependencyEdgeV1]) -> Result<(), DependencyResolutionError> {
        // Build adjacency list for Blocks edges only
        let mut in_degree: std::collections::HashMap<&WorkItemId, usize> = std::collections::HashMap::new();
        let mut adjacency: std::collections::HashMap<&WorkItemId, Vec<&WorkItemId>> = std::collections::HashMap::new();

        // Collect all unique nodes
        let mut all_nodes: std::collections::HashSet<&WorkItemId> = std::collections::HashSet::new();
        for edge in edges {
            if edge.kind == DependencyEdgeKind::Blocks {
                all_nodes.insert(&edge.from_id);
                all_nodes.insert(&edge.to_id);
            }
        }

        // Initialize in-degree for all nodes
        for node in &all_nodes {
            in_degree.entry(node).or_default();
            adjacency.entry(node).or_default();
        }

        // Build the graph and compute in-degrees
        for edge in edges {
            if edge.kind == DependencyEdgeKind::Blocks {
                // Increment in-degree for to_id
                *in_degree.entry(&edge.to_id).or_default() += 1;
                // Add edge to adjacency list
                adjacency.entry(&edge.from_id).or_default().push(&edge.to_id);
            }
        }

        // Kahn's algorithm: start with nodes that have no incoming edges
        let mut queue: Vec<&WorkItemId> = Vec::new();
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push(*node);
            }
        }

        let mut processed_count = 0;

        while let Some(node) = queue.pop() {
            processed_count += 1;

            // Reduce in-degree for all neighbors
            if let Some(neighbors) = adjacency.get(node) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        // If not all nodes were processed, there's a cycle
        if processed_count != all_nodes.len() {
            // Find nodes involved in cycle (those with in-degree > 0 after Kahn's)
            let mut cycle_nodes: Vec<WorkItemId> = Vec::new();
            for (node, &degree) in &in_degree {
                if degree > 0 {
                    cycle_nodes.push((*node).clone());
                }
            }

            return Err(DependencyResolutionError::CycleDetected {
                work_item_ids: cycle_nodes,
            });
        }

        Ok(())
    }

    /// Resolves a transition using the locked semantics.
    ///
    /// This is the main entry point that dispatches to the appropriate
    /// resolution function based on the target status.
    ///
    /// # Arguments
    /// * `edges` - All dependency edges in the graph
    /// * `target` - The WorkItem being transitioned
    /// * `next_status` - The status being transitioned to
    /// * `status_lookup` - A function to look up the current status of a WorkItem
    ///
    /// # Returns
    /// * `Ok(())` if the transition is allowed
    /// * `Err(DependencyResolutionError)` if blocked
    pub fn resolve_transition(
        edges: &[DependencyEdgeV1],
        target: &crate::planning::WorkItemV1,
        next_status: WorkItemStatus,
        status_lookup: &impl Fn(&WorkItemId) -> Option<WorkItemStatus>,
    ) -> Result<(), DependencyResolutionError> {
        // Filter to only incoming edges for our target
        let incoming: Vec<DependencyEdgeV1> = edges.iter().filter(|e| e.to_id == target.id).cloned().collect();

        if next_status.is_terminal() {
            Self::resolve_can_terminalize(target, next_status, &incoming, status_lookup)
        } else {
            // For non-terminal transitions, only Blocks edges matter
            Self::resolve_can_activate(target, &incoming, status_lookup)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_draft() -> WorkItemStatus { WorkItemStatus::Draft }
    fn status_active() -> WorkItemStatus { WorkItemStatus::Active }
    fn status_paused() -> WorkItemStatus { WorkItemStatus::Paused }
    fn status_done() -> WorkItemStatus { WorkItemStatus::Done }
    fn status_superseded() -> WorkItemStatus { WorkItemStatus::Superseded }
    fn status_cancelled() -> WorkItemStatus { WorkItemStatus::Cancelled }

    fn make_work_item(id: &str) -> crate::planning::WorkItemV1 {
        crate::planning::WorkItemV1::new(
            id.into(),
            "cycle-001".into(),
            format!("Work item {}", id),
            "Description".into(),
            None,
            1700000000,
        )
    }

    fn make_blocks_edge(from: &str, to: &str) -> DependencyEdgeV1 {
        DependencyEdgeV1::new(from.into(), to.into(), DependencyEdgeKind::Blocks, None)
    }

    fn make_boc_edge(from: &str, to: &str) -> DependencyEdgeV1 {
        DependencyEdgeV1::new(from.into(), to.into(), DependencyEdgeKind::BlocksOnClosure, None)
    }

    // ── resolve_can_activate tests ──────────────────────────────────────────────

    #[test]
    fn no_predecessors_allowed() {
        let target = make_work_item("B");
        let edges: Vec<DependencyEdgeV1> = vec![];
        let lookup = |_: &WorkItemId| None;

        let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);
        assert!(result.is_ok());
    }

    #[test]
    fn all_predecessors_done_allowed() {
        let target = make_work_item("C");
        let edges = vec![
            make_blocks_edge("A", "C"),
            make_blocks_edge("B", "C"),
        ];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Done);
        lookup_map.insert("B".into(), WorkItemStatus::Done);
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);
        assert!(result.is_ok());
    }

    #[test]
    fn blocks_predecessor_draft_rejected() {
        let target = make_work_item("B");
        let edges = vec![make_blocks_edge("A", "B")];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Draft);
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);
        match result {
            Err(DependencyResolutionError::BlockedBy { predecessor_id, current_status: WorkItemStatus::Draft, .. }) if predecessor_id == "A" => (),
            other => panic!("expected BlockedBy with predecessor_id=A, got {:?}", other),
        }
    }

    #[test]
    fn blocks_predecessor_active_rejected() {
        let target = make_work_item("B");
        let edges = vec![make_blocks_edge("A", "B")];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Active);
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);
        match result {
            Err(DependencyResolutionError::BlockedBy { predecessor_id, current_status, .. }) => {
                assert_eq!(predecessor_id, "A");
                assert_eq!(current_status, WorkItemStatus::Active);
            }
            _ => panic!("expected BlockedBy error"),
        }
    }

    #[test]
    fn blocks_on_closure_non_terminal_target_allowed() {
        // BlocksOnClosure should NOT block non-terminal transitions
        let target = make_work_item("B");
        let edges = vec![make_boc_edge("A", "B")];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Active); // non-terminal predecessor
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);
        assert!(result.is_ok());
    }

    #[test]
    fn self_loop_rejected() {
        let target = make_work_item("A");
        let edges = vec![make_blocks_edge("A", "A")];
        let lookup = |_: &WorkItemId| Some(WorkItemStatus::Draft);

        let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);
        match result {
            Err(DependencyResolutionError::SelfLoop { work_item_id }) if work_item_id == "A" => (),
            other => panic!("expected SelfLoop with work_item_id=A, got {:?}", other),
        }
    }

    #[test]
    fn mixed_status_uses_active_predecessor() {
        // When multiple predecessors exist with different statuses,
        // the first non-terminal one should be reported
        let target = make_work_item("D");
        let edges = vec![
            make_blocks_edge("A", "D"), // Done
            make_blocks_edge("B", "D"), // Active ← should block
            make_blocks_edge("C", "D"), // Cancelled
        ];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Done);
        lookup_map.insert("B".into(), WorkItemStatus::Active);
        lookup_map.insert("C".into(), WorkItemStatus::Cancelled);
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);
        // The order in the edges vec determines which is found first
        // We just check it's blocked
        assert!(result.is_err());
    }

    // ── resolve_can_terminalize tests ──────────────────────────────────────────

    #[test]
    fn terminalize_blocks_on_closure_predecessor_active_rejected() {
        let target = make_work_item("B");
        let edges = vec![make_boc_edge("A", "B")];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Active); // non-terminal
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result = DependencyResolutionService::resolve_can_terminalize(
            &target,
            WorkItemStatus::Done,
            &edges,
            &lookup,
        );
        match result {
            Err(DependencyResolutionError::BlockedBy { predecessor_id, current_status, kind: DependencyEdgeKind::BlocksOnClosure }) => {
                assert_eq!(predecessor_id, "A");
                assert_eq!(current_status, WorkItemStatus::Active);
            }
            _ => panic!("expected BlockedBy error with BlocksOnClosure"),
        }
    }

    #[test]
    fn terminalize_blocks_on_closure_predecessor_done_allowed() {
        let target = make_work_item("B");
        let edges = vec![make_boc_edge("A", "B")];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Done);
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result = DependencyResolutionService::resolve_can_terminalize(
            &target,
            WorkItemStatus::Done,
            &edges,
            &lookup,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn terminalize_with_superseded_predecessor_allowed() {
        let target = make_work_item("B");
        let edges = vec![make_blocks_edge("A", "B")];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Superseded);
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result = DependencyResolutionService::resolve_can_terminalize(
            &target,
            WorkItemStatus::Cancelled,
            &edges,
            &lookup,
        );
        assert!(result.is_ok());
    }

    // ── detect_cycles tests ────────────────────────────────────────────────────

    #[test]
    fn no_cycles_allowed() {
        let edges = vec![
            make_blocks_edge("A", "B"),
            make_blocks_edge("B", "C"),
        ];

        let result = DependencyResolutionService::detect_cycles(&edges);
        assert!(result.is_ok());
    }

    #[test]
    fn simple_cycle_rejected() {
        // A → B → A cycle
        let edges = vec![
            make_blocks_edge("A", "B"),
            make_blocks_edge("B", "A"),
        ];

        let result = DependencyResolutionService::detect_cycles(&edges);
        assert!(matches!(result, Err(DependencyResolutionError::CycleDetected { .. })));
    }

    #[test]
    fn diamond_graph_no_cycle() {
        // A → B, A → C, B → D, C → D (no cycle)
        let edges = vec![
            make_blocks_edge("A", "B"),
            make_blocks_edge("A", "C"),
            make_blocks_edge("B", "D"),
            make_blocks_edge("C", "D"),
        ];

        let result = DependencyResolutionService::detect_cycles(&edges);
        assert!(result.is_ok());
    }

    #[test]
    fn cycle_in_mixed_graph_detected() {
        // Only Blocks edges are considered for cycle detection
        // A → B (Blocks), B → C (Blocks), C → A (Blocks) should be detected
        let edges = vec![
            make_blocks_edge("A", "B"),
            make_blocks_edge("B", "C"),
            make_blocks_edge("C", "A"),
            // These should be ignored for cycle detection
            make_boc_edge("A", "D"),
            make_boc_edge("D", "A"),
        ];

        let result = DependencyResolutionService::detect_cycles(&edges);
        assert!(matches!(result, Err(DependencyResolutionError::CycleDetected { .. })));
    }

    // ── resolve_transition integration tests ────────────────────────────────────

    #[test]
    fn resolve_transition_to_active_uses_blocks_semantics() {
        let target = make_work_item("B");
        let edges = vec![make_blocks_edge("A", "B")];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Active);
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        // Active is non-terminal, so Blocks should block
        let result = DependencyResolutionService::resolve_transition(
            &edges,
            &target,
            WorkItemStatus::Active,
            &lookup,
        );
        assert!(result.is_err());
    }

    #[test]
    fn resolve_transition_to_done_uses_both_edge_kinds() {
        let target = make_work_item("B");
        let edges = vec![
            make_blocks_edge("A", "B"),
            make_boc_edge("C", "B"),
        ];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Done); // terminal
        lookup_map.insert("C".into(), WorkItemStatus::Active); // non-terminal → should block for terminal transition
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        // Both Blocks and BlocksOnClosure block terminal transitions when predecessor is non-terminal
        let result = DependencyResolutionService::resolve_transition(
            &edges,
            &target,
            WorkItemStatus::Done,
            &lookup,
        );
        assert!(result.is_err());
    }

    #[test]
    fn pure_function_no_io() {
        // Verify the service is pure by calling with the same inputs
        let target = make_work_item("B");
        let edges = vec![make_blocks_edge("A", "B")];
        let mut lookup_map: std::collections::HashMap<WorkItemId, WorkItemStatus> = std::collections::HashMap::new();
        lookup_map.insert("A".into(), WorkItemStatus::Done);
        let lookup = |id: &WorkItemId| lookup_map.get(id).copied();

        let result1 = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);
        let result2 = DependencyResolutionService::resolve_can_activate(&target, &edges, &lookup);

        assert_eq!(result1.is_ok(), result2.is_ok());
    }
}
