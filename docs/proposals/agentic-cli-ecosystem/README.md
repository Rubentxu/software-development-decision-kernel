# Ecosistema de CLI Rust para producir evidencias en desarrollo y automatización agéntica

> **Estado:** investigación y cartera de *candidatos*, 19-09-2026; ninguna herramienta descrita existe por el mero hecho de estar documentada aquí. **No constituye un roadmap aprobado, un ADR aceptado ni un cambio de autoridad de SDDK.**
>
> **Base consultada:** SDDK main en \`d3194ff74abe14025964a9d95a96e5c21dda81ee\`; fuentes públicas primarias enlazadas en cada ficha y en [FUENTES-Y-ENCAJE.md](FUENTES-Y-ENCAJE.md). Las funcionalidades y versiones externas requieren validación al abrir cada spike.

## 1. Decisión estratégica: construir lo que falta *entre* herramientas

CogniCode proporciona observaciones de estructura de código; Chronos, observaciones de comportamiento temporal. Ninguno debe convertirse en propietario de escenarios de integración, fidelidad del entorno, selección de pruebas, compatibilidad contractual, cadena de suministro o verificación de efectos. Tampoco debemos añadir esas responsabilidades a SDDK indiscriminadamente.

**Hipótesis de producto:** herramientas Rust autónomas, utilizables directamente por humanos, agentes, CI y workflows, producen resultados específicos y verificables; SDDK los admite mediante puertos, ejecuciones, evidencia, Knowledge y Verification ya existentes. Un resultado de herramienta no es automáticamente un hecho del dominio, una aprobación ni prueba de ausencia de un problema.

### Mapa de propuestas

| Ficha | Pregunta única | Tratamiento inicial | Dependencia y alternativa principal |
|---|---|---|---|
| [ScenarioLab](SCENARIOLAB.md) | ¿Puedo reproducir y verificar un SUT con dependencias reales sin afectar a otros? | **Spike de CLI independiente**; evaluar primero integración con herramientas existentes | mirrord/Telepresence/DevSpace, Docker Compose, testcontainers |
| [EnvLens](ENVLENS.md) | ¿Bajo qué condiciones se obtuvo un resultado y son comparables dos ejecuciones? | **Prototipo pequeño** con consumidores reales; posible librería/CLI independiente | mise/Nix, metadatos de runner, manifests |
| [TestAtlas](TESTATLAS.md) | ¿Qué pruebas corresponden al cambio y cuáles realmente verifican una claim? | **Primero integrar inputs de tests en SDDK**; extraer solo al demostrar consumo multi-proyecto | runners, cargo-affected/testmon, CogniCode |
| [ContractProbe](CONTRACTPROBE.md) | ¿Qué pares exactos de versiones y contratos se han comprobado? | **Spike de correlación**, no un nuevo motor de testing | Pact Broker, Buf, Schemathesis |
| [SupplyScope](SUPPLYSCOPE.md) | ¿Dónde está realmente presente un componente/hallazgo en artefactos desplegados? | **Spike sobre SBOM→artefacto→despliegue** | Syft, OSV-Scanner/Grype, Cosign |
| [EffectCheck](EFFECTCHECK.md) | ¿Una operación produjo el estado esperado, no solo exit 0? | **Capacidad interna de SDDK primero**; CLI autónomo solo con consumidor externo probado | receipts, probes de infraestructura, Conftest |

**No crear seis repositorios por adelantado.** Los nombres son etiquetas de investigación, susceptibles de cambio o absorción por otras herramientas. La separación de binarios/crates de SDDK sigue bajo R11 y su disciplina de justificación.

## 2. Qué no construiremos

- Otro compilador de workflows, scheduler, controlador de CI, base de datos de grafos, motor de tests, túnel de Kubernetes, scanner CVE o motor de policy-as-code.
- Un registro genérico de «insights» ni modelos canónicos \`ScenarioLabResult\`, \`TestAtlasFinding\` o \`EffectCheckRecord\` dentro del dominio SDDK por cada proveedor.
- Una API «verde/rojo» universal: ausencia de evidencia, alcance parcial, timeout, cancelación, fallo de infraestructura, contradicción, incumplimiento y rechazo de autorización tienen semánticas diferentes.
- Automatizaciones autónomas con acceso implícito a clusters, secretos o cambios destructivos. SDDK media autoridad para las operaciones invocadas por él; los CLI independientes deben incorporar controles apropiados para el contexto externo.

## 3. Preguntas de admisión obligatorias para cada CLI

1. **Consumidor**: ¿qué pregunta material de un humano, agente, pipeline o verificador no responde una herramienta ya existente?
2. **Propietario**: ¿cuál es la responsabilidad única de este CLI? ¿Dónde termina SDDK y empieza el proveedor?
3. **Alternativas**: demostrar con fixtures por qué una composición mediante herramientas existentes o una pequeña librería no basta.
4. **Datos**: identificar input, origen, revisión/entorno/scope, salida tipada, parcialidad, productor y condiciones de invalidación; la captura no equivale a verificación.
5. **Seguridad**: quién selecciona/autoriza/ejecuta; qué efectos puede tener; límites de red, secreto, recursos, cancelación y cleanup.
6. **UAT**: especificar un test que **refutaría** el valor o la fiabilidad de la propuesta.
7. **Extracción**: confirmar al menos un consumidor externo real antes de crear otra frontera binaria si el código puede permanecer dentro de SDDK.
8. **Coste**: comparar tiempo, bytes, tokens, falsos resultados y trabajo humano frente al proceso anterior sin inventar ahorros.

## 4. Arquitectura de interoperabilidad mínima

\`\`\`text
Humano / agente / Jenkins / GitHub Actions / OpenShift / otros workflows
                  │ solicita trabajo autorizado
                  ▼
       CLI especializado (propietario de una capacidad)
                  │ invoca herramientas de su ecosistema
                  ▼
 Resultado + alcance + base + estado de completitud + refs
                  │
          ┌───────┴────────┐
          ▼                ▼
  Uso independiente     Adaptador SDDK
  JSON / texto / CI     ├─ Gateway / receipts de ejecución
                       ├─ Observación + evidencia existentes
                       ├─ Verify / Knowledge / Planning
                       └─ ContextCapsule y Orchestrator
\`\`\`

La salida humana y JSON se renderiza desde un resultado semántico común. JSON versionado y no interactivo; stdout queda reservado para resultado estructurado y stderr para diagnóstico sin secretos. Exit code reporta la **operación del CLI**, no demuestra por sí solo una claim. Artefactos pesados permanecen en la herramienta o almacén adecuado; SDDK conserva referencias, bases y evidencias admitidas mediante sus contratos. Para uso autónomo sin SDDK se permite almacenamiento local explícito, sin presuponer su autoridad sobre un proyecto SDDK.

Protocolo preferido: invocación bajo demanda con argv tipado; formato declarativo opcional para escenarios complejos; \`--format json\`, \`--output\`, \`--timeout\`, \`--dry-run\`, \`--version\`, \`--capabilities\` solo si su implementación y semántica están justificadas. No exigir daemon SDDK. Un CLI externo, como Telepresence, puede utilizar su propio proceso auxiliar si el usuario lo autoriza: esto **no** crea una dependencia de daemon para el kernel.

## 5. Proceso de investigación / decisión por etapas

**Descubrir → Reproducir → Comparar → Decidir.** Primero ejecutar una herramienta madura contra un fixture controlado; después localizar el hueco de composición; escribir un prototipo Rust mínimo; por último probar con un segundo consumidor real y determinar si se justifica extraer un producto. Si el paso de comparación falla, **descartar o integrar**, no construir por inercia.

Orden de exploración sugerido, no obligatorio: ScenarioLab con captura EnvLens mínima; test input ingestion de SDDK antes de TestAtlas; un experimento estrecho ContractProbe y otro SupplyScope; EffectCheck como extensión del receipt/gateway actual. Ninguno bloquea A6/A7 ni sustituye el roadmap canónico.

## 6. Integración con documentación existente

- [Evolutivo vigente propuesto sobre inputs y workflows](../2026-09-19-adaptive-inputs-workflows/README.md). Esta cartera **no lo sustituye** ni importa automáticamente nuevos WorkItems.
- [Arquitectura canónica de SDDK](../../architecture/README.md) y [roadmap vigente](../../architecture/a5/A5-CURRENT-ROADMAP.md): priman sobre cualquier sugerencia aquí.
- [Intelligence Provider Boundary](../../architecture/specs/arch-spec-021-intelligence-provider-boundary.md), [Agentic Integration API](../../architecture/specs/arch-spec-030-agentic-integration-api-sdk.md) y [contrato de agente/handoff](../../architecture/specs/arch-spec-007-agent-protocol-and-handoff.md).
- [Fuentes, evidencia comparativa, condiciones de reutilización y UAT transversal](FUENTES-Y-ENCAJE.md).

**Regla de promoción:** trasladar una propuesta al roadmap solo mediante el proceso actual de adopción, con propietario, dependencia, alcance y evidencia. No reabrir A5-C ni afirmar que CC-S0 equivale al proveedor real A6.
