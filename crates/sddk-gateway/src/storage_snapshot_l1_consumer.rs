//! AIW-S7b — StorageSnapshot → SecretaryL1 consumer.
//!
//! Closes G01 (snapshot planning reconciliado, A bloquea B) and G03
//! (proposal surfaces from durable state without holding live Storage).
//!
//! Spec: `G01` **NO** se define aquí. Su fila canónica está en
//! `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md`
//! y el enunciado que la transcribe, en
//! `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7-secretary-attention/SCOPE-CONTRACT.md`
//! §S7-STOP-2.
//!
//! La cita que ocupaba esta línea apuntaba a un `SCOPE-CONTRACT.md` de este
//! mismo ciclo que **nunca se commiteó** (INC-DEBT-066). El error no era de
//! ruta sino de clase: un módulo que cita una spec inexistente declara canónica
//! la nada, y un recibo que declara un criterio que la spec no contiene declara
//! canónica una invención. Lo que este módulo cumple de G01 es exactamente lo
//! que la fila pide —agenda candidato/causa y refs, sin autorización de
//! ejecución— mediante el `evidence_ref` derivado del par reconciliado. La
//! cláusula de `confidence` que el recibo de este ciclo le añadía **no está en
//! la fila** y se retiró con ADR-0156.

use sddk_engine::context_compiler::storage_adapter::StorageSnapshot;
use sddk_engine::{
    BoundedWindow, ClosedSetKind, EvidenceState, ProposalTemplate, RiskTier, SecretaryId,
    SecretaryL1Engine, SecretaryL1Error, SecretaryProposal,
};

/// Error taxonomy for the snapshot consumer.
#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotConsumerError {
    /// The snapshot carried no usable source identifier or payload.
    EmptySnapshot,
    /// The snapshot failed structural validation (`reason` explains why).
    InvalidSnapshot(String),
    /// The underlying `SecretaryL1Engine` rejected the proposal.
    ProposalRejected(SecretaryL1Error),
}

impl std::fmt::Display for SnapshotConsumerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotConsumerError::EmptySnapshot => write!(f, "snapshot is empty"),
            SnapshotConsumerError::InvalidSnapshot(reason) => {
                write!(f, "invalid snapshot: {reason}")
            }
            SnapshotConsumerError::ProposalRejected(err) => write!(f, "proposal rejected: {err}"),
        }
    }
}

impl std::error::Error for SnapshotConsumerError {}

/// Consumes a durable [`StorageSnapshot`] and emits an L1
/// [`SecretaryProposal`] via the real [`SecretaryL1Engine`], without
/// ever holding a live `Storage` handle (G03).
#[derive(Debug)]
pub struct SnapshotL1Consumer {
    engine: SecretaryL1Engine,
    secretary: SecretaryId,
    now_ms: i64,
}

impl SnapshotL1Consumer {
    /// New consumer with the default `snapshot-rehydrate` template
    /// registered against a fresh engine.
    pub fn new(now_ms: i64) -> Self {
        let consumer = Self {
            engine: SecretaryL1Engine::new(),
            secretary: SecretaryId::new("snapshot-secretary"),
            now_ms,
        };
        // Registration is infallible for this fixed template shape.
        consumer
            .engine
            .register_template(consumer.default_template())
            .expect("default template registers");
        consumer
    }

    /// Raw constructor with an empty engine (no default template). Used
    /// by tests and callers that register their own templates.
    pub fn unregistered(now_ms: i64) -> Self {
        Self {
            engine: SecretaryL1Engine::new(),
            secretary: SecretaryId::new("snapshot-secretary"),
            now_ms,
        }
    }

    /// Override the issuing secretary identity.
    pub fn with_secretary(mut self, s: SecretaryId) -> Self {
        self.secretary = s;
        // Re-register the default template under the new identity so the
        // bounded window matches the issuing secretary.
        self.engine
            .register_template(self.default_template())
            .expect("default template re-registers");
        self
    }

    fn default_template(&self) -> ProposalTemplate {
        ProposalTemplate::new(
            "snapshot-rehydrate",
            ClosedSetKind::SuggestRehydrationStep,
            "rehydrate context from a durable storage snapshot",
            RiskTier::Low,
            BoundedWindow::new(self.secretary.clone(), 0, i64::MAX, u32::MAX),
        )
    }

    /// Register an additional template on the underlying engine.
    pub fn register(&self, t: ProposalTemplate) -> Result<(), SecretaryL1Error> {
        self.engine.register_template(t)
    }

    /// Consume a durable snapshot into a closed-set L1 proposal.
    ///
    /// G01: the evidence ref is derived from the snapshot's reconciled
    /// `(adapter_id, log_head)` pair, so a proposal produced from an
    /// older snapshot can never be confused with one produced after a
    /// new write (A blocks B).
    ///
    /// G03: no live `Storage` handle is consulted — the snapshot is the
    /// only input.
    pub fn consume(
        &self,
        snapshot: &StorageSnapshot,
    ) -> Result<SecretaryProposal, SnapshotConsumerError> {
        if snapshot.adapter_id.trim().is_empty() {
            return Err(SnapshotConsumerError::InvalidSnapshot(
                "empty adapter_id".into(),
            ));
        }
        let evidence_ref = format!("storage:{}:{}", snapshot.adapter_id, snapshot.log_head);
        // ADR-0156 / C3m.4. Esto era `if log_head > 0 { 0.95 } else { 0.5 }`, con
        // el comentario «an empty fact log is reported at half confidence» — y
        // esa frase es la que no se sostiene: un log vacío no es media
        // observación, es la ausencia de una. El numero no podia distinguir
        // `Empty` de `Observed`, que es justo la distincion que el consumidor
        // de la propuesta necesita, y por eso el numero era 0.5 y no «0.5 por
        // alguna razon». El `evidence_ref` ya lleva el hecho exacto
        // (`…:{log_head}`), luego el estado no pierde nada y dice lo que el
        // numero no decia.
        let evidence = if snapshot.log_head > 0 {
            EvidenceState::Observed
        } else {
            EvidenceState::Empty
        };
        self.engine
            .propose(
                self.now_ms,
                "snapshot-rehydrate",
                vec![evidence_ref],
                vec!["durable-snapshot".into()],
                vec![],
                format!("rehydrate from snapshot {}", snapshot.adapter_id),
                evidence,
            )
            .map_err(SnapshotConsumerError::ProposalRejected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_engine::context_compiler::storage_adapter::StorageSnapshot;

    fn consumer() -> SnapshotL1Consumer {
        SnapshotL1Consumer::new(1000)
    }

    #[test]
    fn rejects_empty_adapter_id() {
        let snapshot = StorageSnapshot::from_bytes("", 0, vec![]);
        let err = consumer().consume(&snapshot).expect_err("must reject");
        assert_eq!(
            err,
            SnapshotConsumerError::InvalidSnapshot("empty adapter_id".into())
        );
    }

    #[test]
    fn rejects_when_template_not_registered() {
        // Fresh engine with no templates: propose must fail with
        // UnknownTemplate, surfaced as ProposalRejected.
        let c = SnapshotL1Consumer::unregistered(1000);
        let snapshot = StorageSnapshot::from_bytes("storage.ledger_head", 0, vec![]);
        let err = c.consume(&snapshot).expect_err("must reject");
        match err {
            SnapshotConsumerError::ProposalRejected(SecretaryL1Error::UnknownTemplate {
                template_id,
            }) => {
                assert_eq!(template_id, "snapshot-rehydrate");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn accepts_durable_snapshot() {
        let snapshot = StorageSnapshot::from_ledger_events("storage.ledger_head", &[]);
        let proposal = consumer().consume(&snapshot).expect("consume ok");
        assert_eq!(proposal.template_id, "snapshot-rehydrate");
        assert_eq!(proposal.kind, ClosedSetKind::SuggestRehydrationStep);
        assert_eq!(proposal.evidence, EvidenceState::Empty);
        assert_eq!(
            proposal.evidence_refs,
            vec!["storage:storage.ledger_head:0".to_string()]
        );
    }

    #[test]
    fn empty_log_head_is_empty_not_weaker() {
        // ADR-0156. Este test se llamaba `empty_log_head_reduces_confidence` y
        // afirmaba `0.95` / `0.5`. La distincion que realmente importa no es de
        // magnitud: un log vacio no es una observacion mas debil, es la
        // ausencia de una observacion, y `Empty` dice eso mientras que `0.5`
        // solo decia «la mitad de algo que nadie definio».
        let durable = StorageSnapshot::from_bytes("storage.ledger_head", 42, vec![1, 2, 3]);
        let p_hi = consumer().consume(&durable).expect("consume ok");
        assert_eq!(p_hi.evidence, EvidenceState::Observed);
        assert_eq!(
            p_hi.evidence_refs,
            vec!["storage:storage.ledger_head:42".to_string()]
        );

        let empty = StorageSnapshot::from_bytes("storage.ledger_head", 0, vec![]);
        let p_lo = consumer().consume(&empty).expect("consume ok");
        assert_eq!(p_lo.evidence, EvidenceState::Empty);
    }
}
