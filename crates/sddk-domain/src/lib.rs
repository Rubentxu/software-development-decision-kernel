//! sddk-domain — Core domain types for SDDK
//!
//! This crate contains the canonical domain model for SDDK workflows,
//! including project identity, cycle state machines, and artifact references.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

pub mod channel;
pub mod compiler;
pub mod context_read;
pub mod cycle;
pub mod delivery_kind;
pub mod error;
pub mod event_envelope;
pub mod event_registry;
pub mod evidence;
pub mod execution_scope;
pub mod fork;
pub mod format;
pub mod goal;
pub mod graph;
pub mod identity;
pub mod legacy;
pub mod macros;
pub mod metrics;
pub mod models;
pub mod pack;
pub mod ports;
pub mod projections;
pub mod proposal;
pub mod replay;
pub mod rules;
pub mod schema;
pub mod staleness;
pub mod test_adapters;
pub mod test_evidence;
pub mod test_model;
pub mod test_ports;
pub mod test_select;
pub mod transition_ast;
pub mod uat;
pub mod validator;
pub mod view;
pub mod workflow;
pub mod workflow_ir;
pub mod workflow_run;

pub use channel::*;
pub use context_read::*;
pub use cycle::*;
pub use delivery_kind::*;
pub use error::*;
pub use event_envelope::*;
pub use event_registry::{
    CanonicalEventValidator, EventRegistryError, EventSchema, EventSchemaInfo, EventSchemaRegistry,
    EventValidatorError,
};
pub use evidence::*;
pub use execution_scope::{ExecutionScope, ExecutionScopeV1, ScopeError, ScopePath, ScopePhase};
pub use fork::*;
pub use goal::*;
pub use graph::*;
pub use identity::*;
pub use legacy::*;
pub use metrics::*;
pub use models::*;
pub use pack::*;
pub use ports::{
    ArtifactStore, ControlPlane, EventAppended, EventStore, GraphStore, Ledger, LedgerFactory,
    NoopTaskExecutor, TaskError, TaskExecutor, TaskOutput,
};
pub use projections::{
    Checkpoint, CycleState, CycleStateProjection, JournalEntry, JournalProjection, Projection,
    ProjectionError, ProjectionVersion,
};
pub use replay::*;
pub use rules::*;
pub use schema::*;
pub use staleness::*;
pub use test_model::*;
pub use test_ports::*;
pub use transition_ast::{
    ContentHash, EvalContext, MAX_PREDICATE_DEPTH, PredicateExpr, SCHEMA_VERSION, TransitionAst,
    TransitionAstError, TransitionSpecV1,
};
pub use uat::*;
pub use view::*;
pub use workflow::*;
pub use workflow_ir::*;
pub use workflow_run::*;
