# Síntesis ejecutiva

## Problema observado

SDDK ya contiene un sustrato notablemente avanzado: WorkflowIR, WorkflowRuntime, planning graph, EvidenceRef, DecisionRecord, semantic graph, ContextCompiler, ContextDelta, session binding, typed child outputs, capability gateway, CogniCode/Chronos adapters, replay y storage durable. Sin embargo, la experiencia real de los agentes sigue dependiendo en exceso de:

- workflows estáticos por fases;
- prompts que recuerdan comandos y pasos;
- handoffs narrativos;
- bootstrap con contexto pobre;
- uso opcional/manual de intelligence providers;
- persistencia semántica infrautilizada;
- varias generaciones de workflow coexistiendo como autoridades parciales.

La consecuencia es una paradoja: **SDDK implementa más conocimiento del que sus agentes consumen**.

## Tesis de producto

El activo principal de SDDK no debe ser un conjunto de prompts de desarrollo de software. Debe ser el **grafo persistente de trabajo, decisiones, observaciones y evidencias** que cualquier workflow deja detrás.

Un workflow pasa a ser una máquina gobernada que transforma:

```text
ContextBasis + Desired Work
          ↓
      StepRun
          ↓
Actions / Observations / Contributions
          ↓
Canonical Events + Semantic Relations
          ↓
      ContextDelta
          ↓
new ContextBasis + legal next affordances
```

## Cambio de paradigma

### Antes

```text
Prompt A → resumen → Prompt B → comandos recordados → fichero → siguiente fase
```

### Destino

```text
Resource State
   ↓
Hypermedia Representation
(state + relations + legal affordances)
   ↓
StepRun aumentado automáticamente
   ↓
Agent / deterministic executor / provider
   ↓
Typed Contributions
   ↓
Reconciler persists canonical facts
   ↓
New Resource State + affordances
```

## La idea HATEOAS aplicada a agentes

No se propone convertir SDDK en una API REST. Se reutiliza el principio HATEOAS: **el consumidor no necesita conocer de antemano el protocolo completo; el estado actual le indica qué acciones son legales**.

Ejemplo conceptual:

```json
{
  "resource": "sddk://runs/R7/steps/design",
  "state": "ready",
  "context": {"href": "sddk://contexts/C91", "basis": "sha256:..."},
  "relations": [
    {"rel": "work-item", "href": "sddk://work-items/W12"},
    {"rel": "evidence", "href": "sddk://evidence/E44"}
  ],
  "actions": [
    {"rel": "execute", "action": "step.execute", "input_schema": "StepInputV1"},
    {"rel": "record-decision", "action": "decision.record", "input_schema": "DecisionV1"},
    {"rel": "complete", "action": "step.complete", "requires": ["gate:design-sufficient"]},
    {"rel": "replan", "action": "workflow.replan"}
  ]
}
```

CLI, MCP y un futuro HTTP adapter proyectan el mismo recurso; no crean semánticas distintas.

## Augmented Steps

El creador de workflows no debe saber que CogniCode existe para aprovechar análisis estático, ni que Chronos existe para capturar runtime evidence.

Define:

```yaml
- id: design
  use: software.architecture-design
```

El `StepDefinition` ya declara task kind, inputs, outputs, contexto y capabilities deseadas. Un `StepAugmentor` resuelve automáticamente:

- skills compatibles;
- contexto necesario;
- capabilities requeridas/preferidas;
- provider concreto disponible;
- evidence contract;
- authority/policy;
- persistencia de contributions;
- observabilidad y budgets.

El workflow pide `code.dependencies`, no `cognicode`. Pide `runtime.trace`, no `chronos`. Un override de provider puede existir para reproducibilidad/debug, pero no es el default.

## Reutilización real de steps

Un step reusable no puede depender de `proposal.md → design.md`. Debe depender de contratos:

```text
Input: Goal + Constraints + ContextCapsule
Output: Decision[] + ArtifactRef[] + Evidence[] + ContextDelta
```

Eso permite componerlo en software, investigación, documentación o authoring.

## Segundo dominio como falsador

La propuesta exige un workflow no-software antes de declarar el core realmente genérico. Se utiliza **crear un libro**:

```text
brief → research → outline → map(write-section) → consistency-review → revise(loop) → publish
```

El mismo kernel aporta context, evidence, decisions, handoff, retries, budgets, provenance y resource navigation. Cambian los packs y providers, no el kernel.

## Secuencia propuesta

1. **C3i** corrige bootstrap/adoption/resume y elimina redescubrimiento innecesario.
2. **C3j** hace durable ContextCapsule/ContextDelta/session binding y expone un handoff hipermedia mínimo.
3. **C4** sigue con su contrato de certificación actual.
4. **C6** unifica workflow authority, introduce StepDefinition, augmentation y hypermedia protocol completo, y migra los workflows software por strangler.
5. **C7** extrae packs y demuestra un segundo dominio real.

C5 existente continúa como carril opcional; C6 no depende de que X08/J7/J8/J9/R11 se ejecuten.

## Resultado esperado

Al cerrar C7, un agente nuevo debería poder entrar en cualquier workflow y preguntar únicamente:

```text
¿Dónde estoy?
¿Qué contexto es relevante?
¿Qué puedo hacer ahora?
¿Qué contrato tiene cada acción?
```

SDDK responde desde estado persistido, no desde memoria narrativa del prompt.
