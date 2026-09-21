# Product Vision

## Problema

Los IDEs agenticos y los agentes modernos ejecutan cada vez más trabajo, pero el usuario pierde control sobre:

- qué workflow se está ejecutando;
- qué agente o modelo está actuando;
- cuánto tiempo, tokens o coste consume;
- qué ocurrió cuando un proveedor falló;
- por qué se tomó una decisión;
- qué contexto leyó realmente el agente;
- cómo continuar después de una interrupción;
- cómo sustituir un modelo/proveedor agotado;
- cómo validar resultados mediante evidencia y humanos;
- cómo reconstruir una sesión o comparar estrategias.

Cada IDE resuelve una parte y además lo hace de forma distinta. El resultado es dependencia del host, sesiones opacas y workflows frágiles.

## Visión

SDDK será un **control plane local-first y event-sourced para ingeniería de software humano + agentes**.

El IDE agentico se convierte en un **execution host**, no en la autoridad del sistema.

```text
User Goal
   ↓
SDDK Supervisor
   ↓
Workflow Runtime
   ↓
Capability / Execution Router
   ↓
Agentic IDE / Tool / Human
   ↓
Evidence + Events
   ↓
SDDK Ledger / Graph / Cockpit
```

## Capacidades estratégicas

### 1. Workflow agnóstico
SDDK puede ejecutar SDD, bugfix, incident response, UAT, release, security review, migration o research sobre el mismo runtime.

### 2. Orquestación resiliente
Un agente lógico puede cambiar de host, proveedor o modelo sin perder el trabajo lógico.

### 3. Runtime reactivo
Los eventos alimentan behaviors deterministas y cognitivos. El supervisor sólo se despierta cuando aporta valor.

### 4. Contexto compilado
Cada agente recibe un `ContextCapsule` mínimo, trazable y adecuado a su objetivo.

### 5. Evidencia y gobernanza
Las acciones relevantes siguen:

```text
Proposal → Policy → Approval → Capability → Effect → Verify → Receipt
```

### 6. UAT de primera clase
UAT separa ejecución técnica, machine assessment, human decision, acceptance y release sign-off.

### 7. Observabilidad operacional
El usuario puede navegar sesiones, workflows, agentes, tiempos, costes, fallos, contextos y decisiones sin depender de los paneles del proveedor.

### 8. Grafo activo
El grafo representa conocimiento y causalidad operacional; behaviors reaccionan a eventos y producen nuevos eventos.

### 9. Replay y aprendizaje
Los mismos eventos permiten reconstrucción, forks, A/B, debugging y evaluación histórica.

## North Star

El usuario debería poder preguntar:

```text
sddk why workflow wf-123 failed
sddk why decision dec-42
sddk why model gpt-x was selected
sddk why evidence ev-92 became stale
```

y obtener una explicación apoyada en eventos, relaciones y evidence refs, no en memoria volátil del LLM.
