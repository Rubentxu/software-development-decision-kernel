# Hexagonal Boundaries

## Dependency rule

```text
kernel/domain <- application <- adapters <- hosts
```

Un adapter nunca define reglas del dominio; un caso de uso nunca instancia directamente un adapter concreto.

## Puertos recomendados

### Event

```rust
trait EventAppender {
    async fn append(&self, expected: Version, events: &[EventEnvelope]) -> Result<Version>;
}
trait EventReader { /* stream/range/query */ }
trait EventSubscriber { /* live subscription */ }
```

### Workflow

```rust
trait WorkflowDefinitionRepository {}
trait WorkflowRunRepository {}
trait NodeLeasePort {}
```

### Execution

```rust
trait AgentHost {}
trait CapabilityExecutor {}
trait HumanReviewPort {}
trait Clock {}
```

### Context

```rust
trait ArtifactReader {}
trait ContextReadRecorder {}
trait KnowledgeGraphQuery {}
```

### Evidence/Governance

```rust
trait EvidenceRecorder {}
trait PolicyEvaluator {}
trait ApprovalPort {}
trait ReceiptStore {}
```

## Qué debe desaparecer

### `Ledger` como super-port
Un caso de uso que sólo necesita append de eventos no debe recibir acceso implícito a todo el estado.

**Regla:** dependencias por necesidad real, no por comodidad.

### `Engine::from_path()` en application/domain
La apertura de SQLite y selección de filesystem pertenecen al composition root de CLI/host.

### `engine -> storage`
El engine/application consume ports. El host construye `Sqlite...` y lo inyecta.

## Composition root esperado

```rust
fn main() {
    let event_store = SqliteEventStore::open(path)?;
    let graph = EventDerivedGraph::new(...);
    let opencode = OpenCodeAgentHost::new(...);

    let app = SddkApp::builder()
        .event_store(event_store)
        .graph(graph)
        .agent_host(opencode)
        .build()?;
}
```

## Fitness tests

- domain/kernel no importa crates `adapters/*`;
- app no construye SQLite/OpenCode/HTTP clients;
- adapters no contienen transición de workflow;
- hosts no contienen business policy;
- ninguna nueva API recibe `dyn Ledger` si puede recibir un puerto más estrecho.
- los crates genéricos de kernel/domain no exportan tipos `Debt*`; schemas,
  evaluadores y políticas de deuda pertenecen al pack SDD.
