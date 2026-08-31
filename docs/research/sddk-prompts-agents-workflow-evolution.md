# Evolución de la capa de prompts, agentes, skills y validaciones SDDK

**Fecha:** 2026-08-24

**Estado:** recomendación implementada y formalizada por `ADR-0060`; los trials con modelos reales siguen pendientes
**Issue:** [#93](https://github.com/Rubentxu/software-development-decision-kernel/issues/93)

**Baseline analizada:** `v1.41.0`, commit `51b4502`
**Objetivo:** mejorar la fiabilidad, utilidad y comunicación de las instrucciones y validaciones de agentes IA sin romper la automatización ni la regla de cero intrusión.

**Frontera de implementación:** este evolutivo solo puede modificar prompts, agentes, skills, contratos y plantillas de instrucciones, fixtures y evaluaciones de agentes. Quedan fuera de alcance `crates/`, runtime Rust, CLI, WorkflowIR, schemas del kernel, storage, CAS, ledger, control plane y sus migraciones. Los hallazgos sobre esas superficies son contexto o dependencias para el roadmap activo, no trabajo de esta iniciativa.

## Lectura rápida

| Necesidad | Sección |
|---|---|
| Entender la conclusión | [Conclusión ejecutiva](#1-conclusión-ejecutiva) |
| Ver los problemas actuales | [Problemas actuales y evidencia](#4-problemas-actuales-y-evidencia-observada) |
| Entender la propuesta | [Arquitectura de instrucciones mínima](#5-propuesta-recomendada-arquitectura-de-instrucciones-mínima) |
| Mejorar el roadmap activo sin duplicarlo | [Frontera con el roadmap activo](#11-frontera-con-el-roadmap-activo) |
| Revisar el orden de implementación | [Roadmap recomendado](#13-roadmap-recomendado) |
| Saber por dónde empezar | [Siguiente paso](#18-siguiente-paso) |

## 1. Conclusión ejecutiva

El evolutivo debe actuar exclusivamente sobre el comportamiento inducido por instrucciones y sobre cómo se evalúan los agentes. Muchas garantías deseadas ya están declaradas, pero varias dependen de que el modelo interprete correctamente texto duplicado o ambiguo. La oportunidad inmediata no es ampliar el kernel: es reducir drift, hacer observables los criterios de finalización y convertir los fallos reales de agentes en regresiones reproducibles.

La evolución recomendada tiene cinco movimientos:

1. Construir un baseline ejecutable de prompts, skills y agentes antes de refactorizarlos.
2. Clarificar la autoridad de cada superficie y centralizar un contrato CLI eficiente: wrapper, skill, prompt de fase, contrato compartido y referencia bajo demanda.
3. Reforzar `verify` como contrato de investigación y adjudicación: ejecutar herramientas existentes, exigir evidencia localizable y no reinterpretar fallos deterministas.
4. Evaluar agentes con tareas completas, casos adversariales, múltiples trials y graders separados del agente ejecutor.
5. Mejorar reportes y C4 desde instrucciones y plantillas: español por defecto, idioma configurable, progressive disclosure y distinción explícita entre hechos observados, intención y render.

La secuencia importa: eval baseline primero, refactor de instrucciones después y pruning al final. Si una mejora exige cambiar Rust, CLI, persistencia o lifecycle, se registra como dependencia del roadmap canónico y se detiene en esta rama.

### Estado de implementación parcial (2026-08-24)

- Infraestructura del Ciclo 0 implementada: 14 casos held-out, evaluator/judge
  separados, snapshots temporales, corpus de solo lectura, entorno allowlist,
  timeout/límite de salida, política de red explícita, procedencia por invocación,
  labels fijados por digest, validación del finding completo y grading
  determinista de sus etiquetas semánticas. La
  baseline empírica y el criterio de cierre siguen pendientes de trials con
  modelos reales; no se simularon resultados ni se incorporaron a CI.
- Ciclo 1 implementado en prompts/skills: autoridad CLI compartida, `cli_context`,
  ownership, JSON, freshness, lease, error policy, budgets y evidence material.
- Contratos del Ciclo 2 implementados: verify L0-L6, finding schema, referencia
  multi-stack y fixtures para placeholders, hardcodes, wiring, negative controls
  y evidencia. Su eficacia con modelos sigue pendiente de la baseline real.
- Ciclo 3 implementado: locale BCP 47 con `es` por defecto, progressive
  disclosure, HTML derivado/offline y skill C4/LikeC4 opcional con fallback.
- Ciclo 4 aplicado solo al pruning seguro respaldado por contratos. El pruning
  estadístico amplio permanece pendiente hasta disponer de una baseline de
  trials reales comparable.

## 2. Método y límites

Se inspeccionaron:

- `agents/`, `skills/`, `prompts/sddk/` y contratos compartidos.
- CLI, engine, domain, storage, gateway, ledger, CAS y control plane.
- Fases `propose`, `apply`, `verify`, `debt-verify`, `release` y `archive`.
- Reportes HTML, explorer, dashboard UAT, telemetría y C4 actual.
- Tests de contrato, tests CLI y golden dataset.

Se contrastaron las conclusiones con documentación primaria u oficial de Anthropic, OpenAI, Google, NIST, OWASP, SLSA, W3C, C4, LikeC4 y estándares de documentación por lenguaje.

Las superficies de runtime se inspeccionaron para identificar límites y falsas garantías, no para proponer cambios en ellas. Este informe no introduce herramientas ni contratos nuevos en el kernel. Una fuente externa respalda un principio; no convierte automáticamente una herramienta en dependencia de SDDK.

### 2.1 Alcance implementable

| Dentro de alcance | Fuera de alcance |
|---|---|
| `prompts/sddk/`, `agents/`, `skills/` | `crates/`, binarios y runtime Rust |
| Contratos y plantillas de salida de agentes | CLI, FSM, WorkflowIR y schemas del kernel |
| Golden tasks, fixtures y graders de agentes | CAS, ledger, storage y control plane |
| Validaciones semánticas y evidence checklists | Nuevos scanners o evaluators compilados |
| Instrucciones de reporting, locale y C4 opcional | Renderers, catálogo de artefactos y persistencia |

La capa de instrucciones puede exigir que un agente ejecute herramientas ya disponibles y puede validar sus resultados. No puede afirmar que una garantía está enforced por runtime cuando el roadmap activo todavía no la ha implementado.

## 3. Mapa sistémico

### Glosario mínimo

| Término | Significado en este informe |
|---|---|
| MCW | Mandatory Complete Workflow: secuencia end-to-end obligatoria |
| Gate | Condición obligatoria para aceptar una transición o resultado |
| Receipt | Evidencia registrada de que se evaluó un gate o capability |
| CAS | Content-addressed storage: almacenamiento identificado por el hash del contenido |
| Ledger | Registro ordenado e íntegro de eventos del workflow |

### 3.1 Propósito

SDDK debe transformar una intención humana en software verificable y publicable, manteniendo una cadena de decisiones y evidencias que permita responder:

- Qué se quiso conseguir.
- Qué se implementó realmente.
- Cómo se comprobó.
- Qué riesgos y deuda permanecen.
- Qué artefactos y decisiones condujeron al estado actual.

### 3.2 Elementos

| Elemento | Responsabilidad deseada |
|---|---|
| Orchestrator | Selección y secuenciación del workflow; no trabajo de fase inline |
| Agente de fase | Ejecutar un contrato de fase acotado |
| Skill | Activación, adaptación y referencias bajo demanda |
| Prompt de fase | Procedimiento, entradas, salidas y completion criteria |
| CLI/engine | Autoridad ejecutable, identidad, gates, transiciones y receipts |
| Ledger/CAS | Historia inmutable, integridad y bytes de artefactos |
| Control plane | Índice reconstruible y análisis temporal/cross-project |
| Renderer | Presentación humana localizada; nunca autoridad semántica |
| Developer | Autoridad de producto y consumidor de la comunicación |

### 3.3 Bucles dominantes

| Bucle | Dinámica | Riesgo |
|---|---|---|
| Más fallos -> más reglas textuales -> prompts más largos -> menor atención -> más fallos | Reforzador | Sedimentación y context rot |
| Más gates nominales -> mayor confianza -> menor inspección del enforcement real | Reforzador | Falsa seguridad |
| Fallo real -> caso de evaluación -> regresión detectable -> mejora del prompt/gate | Balanceador deseado | Hoy el golden dataset no ejecuta el sistema completo |
| Más reportes -> más información -> más carga cognitiva -> menos lectura | Reforzador | Comunicación aparente, no efectiva |
| Evidencia tipada -> vistas reutilizables -> menor coste de explicar -> mejor feedback | Reforzador deseado | Requiere separar hechos de render |

### 3.4 Leverage points

1. **Autoridad ejecutable única:** alinear workflow, gates, schemas y transiciones.
2. **Outcome sobre narrativa:** comprobar estado final y comportamiento, no declaraciones del agente.
3. **Corpus de evaluación:** convertir cada escape real en una regresión reproducible.
4. **Artefactos tipados:** una misma evidencia alimenta verify, reportes, timeline y analytics.
5. **Presentación progresiva:** cambiar cómo se comunica sin duplicar la verdad.

### 3.5 Trampas a evitar

- **Shifting the burden:** añadir prompts para compensar carencias del CLI.
- **Seeking the wrong goal:** optimizar número de gates, reportes o comentarios en vez de corrección y comprensión.
- **Rule beating:** detectar hardcodes solo por regex; el agente puede cambiar la forma sin cambiar el engaño.
- **Compliance art:** exigir un C4 decorativo que no está ligado a código ni evidencia.
- **Eroding goals:** convertir falta de evidencia obligatoria en warning para que el ciclo avance.
- **Goodhart:** medir volumen de documentación o reportes y provocar relleno inútil.

## 4. Problemas actuales y evidencia observada

Los cuatro problemas principales son: autoridades de workflow que no convergen, gates declarados sin enforcement suficiente, reportes derivados de prosa en lugar de hechos tipados y visualizaciones C4 sin evidencia semántica estable.

### 4.1 Sistema de instrucciones

El diseño conceptual ya distingue autoridades:

- CLI/ledger como estado real.
- MCW como secuencia declarativa.
- Prompt de fase como semántica operacional.
- Workflow YAML como proyección.
- Agentes y skills como wrappers que no deben redefinir gates.

La intención está documentada en `prompts/sddk/orchestrator.md:8-36`. Sin embargo, hay varias representaciones mantenidas manualmente y no todas convergen en runtime. Existe `WorkflowIR`, pero `cycle` intenta cargar `<root>/workflow/workflow.yaml` y, cuando no existe bajo la política de cero intrusión, usa el manifest embebido mediante `load_workflow_str` (`crates/sddk-cli/src/cycle.rs:113-125`). La presencia de IR, compilador y runtime nuevo no demuestra por sí sola que todas las rutas del CLI estén gobernadas por esa autoridad.

El problema no es la ausencia de arquitectura, sino la coexistencia de:

- MCW y prompts de fase.
- YAML canónico del workflow.
- Proyecciones YAML por path.
- WorkflowIR/runtime nuevo.
- Schemas y tests textuales.
- Bundle instalado en editores.

La regla correcta es que una representación sea canónica y las demás sean generadas o verificadas contra ella.

### 4.2 Contradicciones de workflow que limitan las instrucciones

Estas inconsistencias impiden que los prompts prometan más de lo que el runtime demuestra:

1. Las transiciones A-* de verify exigen receipts nominales `debt-severity-assigned` y `debt-priority-assigned` (`workflow/workflow.yaml:255-331`), pero su outcome todavía no está ligado por un evaluator tipado al `debt-report.json` real.
2. El prompt de verify solo evalúa `tests-pass` y `policy-compliant`, y ejecuta debt-verify después (`prompts/sddk/phases/verify.md:260-285`).
3. Debt-verify no dispone de una transición runtime propia y `Phase` no contiene `DebtVerify`; el contrato actual lo define como capability/gate deliberadamente externo a la enum (`prompts/sddk/phases/debt-verify.md:7-9`).
4. A-full transita a `review`; la fase y transición existen en runtime, pero no hay una superficie SDDK completa de agente, skill y prompt equivalente a las demás.
5. El CLI público de deuda crea y evalúa un informe vacío con cycle y vault hardcodeados (`crates/sddk-cli/src/debt.rs:63-100`, `285-320`). Puede devolver una señal tranquilizadora sin leer la deuda real del ciclo.

**Conclusión de alcance:** el engine puede avanzar si el caller aporta todos los receipts requeridos, pero la secuencia documentada `verify -> debt-verify` no demuestra mediante un evaluator tipado que esos receipts procedan del informe real. Este evolutivo no corregirá el engine. Sus instrucciones deben describir la limitación, no fabricar receipts ni presentar esa garantía como ejecutable. La convergencia runtime pertenece a las fases 3-4 y 8 del roadmap canónico.

### 4.3 Verify: contrato fuerte, enforcement parcial

`verify.md` ya declara gates para:

- Comportamiento.
- Implementación real.
- Disciplina documental.
- Fuerza de tests.
- Build y regresión.
- Production readiness.
- Diseño y SOLID.
- Completitud de tareas.

La mayor parte depende de búsqueda y juicio del LLM. La automatización especializada real se concentra en `comments_check`.

#### Defectos confirmados de `comments_check`

| Defecto | Evidencia | Consecuencia |
|---|---|---|
| Diff limitado a `crates/` | `comments_check.rs:289-305` | El subgate `comments` puede obtener `PASS (no added lines)` sin analizar código Python, Go o TS; el comando global además falla o resulta inaplicable por su acoplamiento a Cargo |
| Rango del hunk desplazado | `comments_check.rs:317-375` | Escanea una línea adicional y puede incluir líneas no añadidas |
| Prefiltros incompletos | `comments-rules.yaml:230-263` | `FIXME`, `XXX`, `HACK`, `Ticket`, JIRA y varios handles no alcanzan el regex |
| `placeholder_macro` no existe | Prompt `verify.md:82` frente al contrato real | Drift entre documentación y scanner |
| Comentarios multilínea sin estado | `comments_check.rs:459-485` | No inspecciona de forma fiable interiores Javadoc/JSDoc/docstrings |
| Errores de lectura fail-open | `comments_check.rs:251-256` | Un archivo ilegible se omite con warning |
| JSON sin findings | `dev/check.rs:126-133` | La automatización solo recibe `passed`, sin evidencia estructurada |
| Cargo siempre activo | `dev/check.rs:6-48` | El comando que aparenta ser genérico está acoplado al workspace Rust |

Además, el prompt permite contextualizar como válida una doc con un REQ adjunto (`verify.md:84`), mientras el scanner devuelve no-cero y el mismo prompt prohíbe reinterpretar un fallo determinista (`verify.md:99`).

Estos defectos son evidencia para calibrar los agentes y crear casos de evaluación. Corregir `comments_check`, `dev check` o su JSON queda fuera de este evolutivo. Hasta que el roadmap activo los resuelva, `verify` debe registrar sus límites y complementar la inspección con herramientas existentes sin convertir una ausencia de findings en prueba de ausencia.

### 4.4 Reportes y persistencia

SDDK ya dispone de buenos bloques:

- Rutas XDG separadas para CAS, artefactos legibles, generados y ledger (`crates/sddk-engine/src/paths.rs:25-61`, `122-136`).
- Store content-addressed con metadata (`crates/sddk-cli/src/artifact.rs:89-119`).
- `ArtifactRecord` y puerto para listar artefactos por proyecto, aunque el CLI solo expone `store|get`.
- Explorer y dashboard de telemetría derivados de estado.
- Control plane SQLite diseñado como proyección reconstruible (`docs/control-plane/SPEC.md:21-30`); la spec sigue en estado `draft` y el determinismo debe probarse en la implementación.

Los gaps son:

- No existe catálogo CLI de artefactos/reportes por proyecto, ciclo, fase o tiempo.
- Guardar un artefacto no produce un evento de dominio `artifact.created` claramente consultable en la timeline.
- El HTML final se genera por prompt, no por renderer determinista.
- `HTML-REPORT.md` llama al HTML "canonical artefact", aunque la autoridad debe ser ledger + evidencia + manifests.
- El closing report se declara autocontenido, pero usa Google Fonts, Tailwind y Mermaid desde CDN y `securityLevel: "loose"` (`HTML-REPORT.md:59-72`).
- Conviven layouts antiguos y actuales para `cierre.html`.
- El control plane actual indexa métricas de ciclos, no el catálogo completo de artefactos.

**Dependencia externa:** el roadmap activo ya asigna el grafo y `sddk why` a la fase 10, el Cockpit a la fase 11 y el lifecycle de artefactos a la fase 14. Este evolutivo no modifica el control plane ni añade persistencia; solo evita que prompts y plantillas llamen "canónico" a un render HTML.

### 4.5 C4 y LikeC4

El closing report contiene ejemplos C4 Mermaid, pero se infieren desde prosa y templates. No preservan necesariamente:

- IDs estables.
- Revisión del grafo.
- Evidencia por elemento/relación.
- Distinción entre observado y planificado.
- Cobertura de evidencia.
- Validación contra código.

Por tanto, el C4 actual es comunicación visual, no una proyección semántica verificable.

La arquitectura del proyecto ya decidió no acoplar C4/archctl al kernel (`docs/sddk-2.0-architecture-consolidation/roadmap/ROADMAP.md:301-307`). Esa decisión sigue siendo correcta.

LikeC4 `1.59.2`, versión viva consultada en npm el 2026-08-24, aporta:

- Modelo jerárquico y relaciones.
- Vistas como proyecciones del modelo.
- `likec4 validate` para sintaxis y layout drift.
- Build relocatable y `--output-single-file`.
- Export JSON, PNG/JPEG, Mermaid, PlantUML, Dot, D2 y DrawIO.

LikeC4 mantiene, consulta, valida y renderiza su modelo, pero no sustituye el diff semántico requerido por este workflow. En este evolutivo solo puede aparecer como herramienta opcional invocada por una skill, con instrucciones de validación y fallback textual. Cualquier integración persistente pertenece al roadmap activo.

### 4.6 Idioma y audiencia

El estado actual es inconsistente:

- El closing report fuerza español (`HTML-REPORT.md:3`).
- UAT usa `<html lang="es">` hardcodeado.
- Explorer usa `<html lang="en">`.
- El resumen del orchestrator imita el idioma de la conversación (`orchestrator.md:261-262`).
- No existe una configuración común para reportes.

**Recomendación pendiente de decisión:** la política requerida es:

- Reportes persistentes: español por defecto.
- Idioma configurable explícitamente.
- Contratos máquina, IDs, enums, comandos, paths y schemas: estables y no traducidos.
- Chat: puede seguir el idioma de la conversación.

### 4.7 Uso de CLI por los agentes

La CLI fue inspeccionada como autoridad, pero las instrucciones actuales no forman todavía un contrato coherente y eficiente por fase. Hay defectos de corrección antes incluso de optimizar llamadas:

| Hallazgo | Evidencia | Consecuencia |
|---|---|---|
| Consulta global de lease inexistente | `CycleLockStatusArgs` exige `--cycle` (`cycle.rs:400-409`), pero MCW y `sddk-cycle-resume` lo omiten | El error se oculta como `{}`/lease ausente y puede autorizar un nuevo ciclo sobre una precondición no comprobada |
| Comando de identidad incompleto | La CLI declara `sddk project resolve` (`lib.rs:324-328`), pero `sddk-cycle-resume` usa `sddk project` | El state token no obtiene identidad desde el comando documentado |
| Flags de vault inexistentes | `vault validate|index` aceptan `--vault` y `--db` (`vault_cmd.rs:28-40`); la skill usa `--vault-path` y `--rebuild` | La validación de vault falla y el fallback puede convertir el fallo en drift cero |
| Campos esperados que la CLI no devuelve | `cycle status` devuelve cycle/status/phase/path/updated_at/artifact count/lease; lock status devuelve solo lease; `vault validate` devuelve nodes/backlinks/errors/warnings/inserted/updated/deleted/diagnostics | La skill intenta leer `branch`, `head_sha`, `cycle_id` del lease y `drift_count`; el envelope puede contener defaults sin evidencia |
| Archive invoca vault sin argumento requerido | `archive.md:59` ejecuta `vault validate` sin `--vault`, obligatorio en `VaultIndexArgs` (`vault_cmd.rs:28-40`) | El cierre puede bloquear por una invocación inválida, no por drift real del vault |
| Resolución duplicada | `knowledge status` ya devuelve `project_id`, `vault_path`, presencia de profile/vault y Engram (`knowledge_cmd.rs:172-185`), pero preflights ejecutan además `knowledge path` | Más round-trips y dos lugares de parsing para el mismo dato |
| Salida humana en automatización | Todos los comandos lifecycle relevantes admiten `--format json`, pero explore/spec/design/tasks/apply usan salida text por defecto | Parsing frágil y evidencia difícil de validar automáticamente |
| Receipts nominales | Explore/spec/design/tasks/apply llaman `evaluate-gate` con `{"checked": true}` | La CLI firma la evaluación aportada por el caller, pero el receipt no demuestra qué artefacto, subject o comprobación produjo el outcome |
| Verificación de ledger sin política por frontera | Propose y coherence ejecutan `ledger verify` aunque no aplican transición; propose almacena CAS/metadata, pero `artifact store` no emite un evento de ledger (`artifact.rs:89-119`) | La llamada puede aportar confianza aparente sin verificar la operación que acaba de realizarse |

El coste no es solo número de procesos. Cada comando lifecycle vuelve a resolver identidad, abrir storage y cargar workflow mediante `RuntimeContext::open` (`cycle.rs:63-109`). Reducir duplicaciones importa, pero nunca debe eliminar la lectura fresca necesaria justo antes de una mutación o debilitar fencing e integridad.

## 5. Propuesta recomendada: arquitectura de instrucciones mínima

Las secciones 6-12 detallan cómo aplicar esta propuesta a verify, documentación, evaluaciones, prompts de fase, C4 opcional y comunicación.

### 5.1 Separar cinco superficies

```text
Agent wrapper -> Skill -> Prompt/contrato de fase
                              |
                              v
                    Herramientas existentes
                              |
                              v
                 Evidencia + output estructurado
```

| Superficie | Contiene | No contiene |
|---|---|---|
| Agent wrapper | Rol host, permisos, herramientas y delegación | Procedimiento completo de fase |
| Skill | Trigger, adaptación y referencias bajo demanda | Copias del contrato compartido |
| Prompt de fase | Pasos, entradas, evidencia y completion criteria | Implementación del runtime |
| Contrato compartido | Reglas transversales con una autoridad única | Excepciones específicas de cada fase |
| Evaluación | Task, fixture, trace, outcome y grader | Cambios de producción para hacer pasar el caso |

### 5.2 No crear subsistemas de runtime

Este evolutivo no añade crates, comandos, eventos, tablas ni servicios. Reutiliza las herramientas ya disponibles desde instrucciones y deja explícitas sus limitaciones.

- Mantener LikeC4 y archctl como herramientas opcionales llamadas por skills, no dependencias del motor.
- Mantener el harness de evaluación externo al kernel.
- Usar formatos estructurados solo como contratos de salida de agentes o fixtures, no como nuevos schemas runtime.
- Convertir cualquier necesidad de persistencia o enforcement compilado en una dependencia documentada del roadmap activo.

### 5.3 Contrato compartido de uso eficiente de CLI

Las instrucciones deben centralizar un único contrato CLI consumido por orchestrator, skills y prompts de fase:

1. **Un owner por llamada.** El orchestrator posee bootstrap y legalidad de dispatch; el coordinator de fase posee gate/transition; workers y lenses no ejecutan comandos lifecycle.
2. **JSON para automatización.** Usar `--format json` siempre que exista y validar exit code + shape antes de consumir campos. Text queda para interacción humana.
3. **Resolver inmutables una vez.** Adoption, `project_id`, `vault_path` y subject base viajan en el launch packet. `cycle_artifacts_dir` se resuelve una vez solo después de disponer de un `cycle_id` válido; antes del primer start permanece `null`.
4. **Refrescar mutables en fronteras.** Cycle phase, status y lease se leen antes del dispatch y de nuevo justo antes de gate/transition solo si hubo espera, mutación externa posible o riesgo de expiración.
5. **No consultar lease sin cycle ID.** Cuando el ID es conocido, `cycle status --format json` ya incluye lease. Cuando no existe un ID fiable, la baseline no ofrece discovery global: no se puede interpretar un error como "no active cycle"; `cycle start` conserva su propio conflicto fail-closed.
6. **Renovar por expiración, no por costumbre.** Comparar `expires_at_ms` con el tiempo esperado hasta la siguiente mutación; `lock renew` conserva el fencing token y su JSON sustituye el lease anterior.
7. **Evidencia material.** `evaluate-gate` recibe subject SHA/diff, artifact path+hash, checks ejecutados, outcomes, commands/exit codes y output digest cuando aplique. `{"checked": true}` está prohibido.
8. **Una verificación por frontera con significado.** Tras una secuencia gate+transition, ejecutar una sola verificación de ledger. Archive mantiene pre y post verificación porque la primera fundamenta `ledger-valid` y la segunda comprueba el append de cierre. Una fase sin mutación de ledger no usa `ledger verify` como prueba de CAS, filesystem o calidad.
9. **Fallos visibles.** Prohibido `2>/dev/null || echo ...` sobre consultas autoritativas. Distinguir `not_found`, `invalid_invocation`, `corrupt_state`, `permission_denied` y `tool_unavailable`; solo un `not_found` documentado puede significar ausencia.
10. **Comandos validados contra la baseline.** Cada ejemplo de prompt/skill debe corresponder al árbol Clap y a su output JSON en la versión declarada del bundle.

El orchestrator pasa un snapshot compacto, no stdout libre, a cada coordinator:

```yaml
cli_context:
  cli_version: semver
  observed_at: RFC3339
  project: {root: absolute-path, project_id: string, adopted: bool}
  knowledge: {vault_path: absolute-path, profile_present: bool, engram_enabled: bool}
  cycle: {cycle_id: string|null, status: string|null, phase: string|null, path: string|null, updated_at: RFC3339|null}
  lease: {owner: string, fencing_token: integer, expires_at_ms: integer}|null
  cycle_artifacts_dir: absolute-path|null
  source_commands: [{argv: [], exit_code: integer, output_digest: sha256}]
```

Los coordinators pueden reutilizar campos inmutables. Los campos mutables se refrescan según la frontera de estado y dejan una nueva entrada en `source_commands`; nunca se actualizan por inferencia del agente.

El objetivo no es minimizar llamadas a cualquier precio. Es eliminar llamadas inválidas o redundantes y conservar exactamente las lecturas necesarias para identidad, fencing, transición e integridad.

## 6. Verify como contrato de validación de agentes

### 6.1 Subject correcto

"Lo implementado en la sesión" no es una identidad reproducible. El prompt debe solicitar y devolver, cuando el CLI actual los exponga, estos datos de scope:

```yaml
subject:
  cycle_id: string
  base_commit: sha
  head_commit: sha
  changed_paths: []
  production_roots: []
  test_roots: []
  generated_roots: []
```

La sesión puede aportar contexto, pero el agente debe ligar su veredicto a un par inmutable `base/head` y al ciclo. Este contrato de salida no crea ni modifica schemas del kernel.

### 6.2 Tubería por capas

| Capa | Tipo | Responsabilidad | Puede bloquear sola |
|---|---|---|---|
| L0 | Identidad | SHA, CWD, árbol limpio, herramientas | Sí |
| L1 | Determinista | Build, tests, lint, format, schema | Sí |
| L2 | Scanner | Candidatos de docs, placeholders, secrets, wiring | Solo reglas de alta precisión |
| L3 | Lenguaje/AST | Herramientas existentes para visibilidad, cuerpos, composición y API pública | Sí con regla calibrada |
| L4 | Comportamiento | Negative controls, contract/integration, mutación dirigida | Sí |
| L5 | Semántica LLM | Intención, diseño, legibilidad, falsos positivos | No puede anular L0-L4 |
| L6 | Adversarial | Casos retenidos, jueces independientes | Bloquea según path/política |

Los scanners y comandos existentes producen evidencia; el agente la normaliza como **candidatos** con `rule_id`, confianza y localización. El coordinador adjudica solo los casos ambiguos. Un comando no-cero no se transforma en PASS por opinión. Este evolutivo mejora ese procedimiento y sus graders, no implementa scanners nuevos.

### 6.3 Detección anti-placeholder

No debe mezclarse con el scanner de comentarios.

#### Casos de evaluación de alta precisión

| Familia | Ejemplos | Exenciones necesarias |
|---|---|---|
| Primitivas explícitas | Rust `todo!`, `unimplemented!`; Python `NotImplementedError`; JS/Java/C# `throw ...not implemented` | Código generado, API abstracta intencional, test fixture |
| Cuerpos vacíos | `pass`, método vacío, handler sin efecto | Hook opcional documentado, trait/interface, no-op intencional |
| Éxito constante | `Ok(())`, HTTP 200 fijo, `true`, lista vacía | Contrato legítimo demostrado por contexto |
| Wiring fake | In-memory/fake/mock en composition root productivo | Perfil demo/dev explícito y no publicable |
| Bypass | Rama siempre tomada, feature flag hardcodeada, código solo alcanzable desde tests | Compatibilidad temporal preaprobada y expirable |

#### Hardcodes

Un literal no es automáticamente un hardcode defectuoso. Debe bloquear cuando:

- Sustituye configuración, estado o una dependencia requerida.
- Solo satisface los ejemplos conocidos.
- Evita ejecutar el camino productivo.
- Contiene secret o identidad de entorno.
- Una mutación o escenario alternativo demuestra que el test sigue pasando incorrectamente.

Por eso la validación del agente debe combinar búsquedas y analizadores ya disponibles, inspección de composición y negative controls. Una lista de palabras es insuficiente y fácil de gaming.

### 6.4 Contrato de finding

```yaml
finding_id: sha256(rule_id + canonical subject + canonical location)
rule_id: quality.anti-placeholder.rust-unimplemented
subject: {base: sha|null, head: sha|null, diff_digest: sha256|null}
location: {path: string, start_line: integer, end_line: integer, symbol: string|null}
classification: blocking_defect | warning | suggestion | false_positive | insufficient_evidence
severity: critical | high | medium | low
confidence: high | medium | low
production_reachable: yes | no | unknown
evidence: []
exemption: {authority: string, reason: string, expires_at: string|null}|null
owner_phase: apply | verify | debt-verify | replan | human
```

Verify posee defectos de comportamiento y fidelidad productiva. Debt-verify posee mantenibilidad más amplia y deuda preexistente. Si observan el mismo problema deben compartir `rule_id` y fingerprint, no duplicar findings.

### 6.5 Frontera con debt-verify

El modelo actual declara debt-verify como capability/gate, no como `Phase`. Decidir o implementar su lifecycle requiere cambios de runtime y queda fuera de alcance. Las instrucciones sí deben:

- mantener separado el ownership de verify y debt-verify;
- no fabricar receipts ni afirmar que un informe nominal fue evaluado;
- conservar y citar la evidencia disponible;
- devolver `BLOCKED` o `INCONCLUSIVE` cuando el contrato runtime no permita demostrar la transición.

La resolución estructural corresponde a la fase 8 y al slice de deuda durable del roadmap canónico.

## 7. Documentación de código útil

### 7.1 Política positiva

El objetivo no es "más comentarios". El objetivo es que la superficie que otro developer consume explique su contrato sin depender de tickets o historia del proyecto.

Documentar cuando aporte al menos uno de estos valores:

- Propósito y contrato público.
- Invariantes, precondiciones y postcondiciones.
- Unidades, rangos y formatos.
- Errores, excepciones, panics o códigos de fallo relevantes.
- Efectos laterales, persistencia y concurrencia.
- Requisitos de seguridad o comentarios `SAFETY`.
- Algoritmo o decisión no obvia.
- Ejemplo de uso cuando reduzca ambigüedad.

Evitar:

- Repetir el nombre o la siguiente línea de código.
- Narrar commits, ciclos o estados pasados.
- Referenciar usuarios, reviewers o autores.
- Adjuntar IDs de ticket, issue, PR, task, requirement o ADR como sustituto de una explicación.
- Dejar código comentado.
- Usar TODO/FIXME/HACK para diferir trabajo obligatorio.

La trazabilidad válida vive en `evidence_refs`, manifests, ledger, ADRs y reportes; no en el comentario productivo.

### 7.2 Perfiles por lenguaje

| Lenguaje | Estándar/autoridad | Adaptación recomendada |
|---|---|---|
| Rust | rustdoc y lints `missing_docs` | `///`/`//!`, secciones de errors/panics/safety/examples cuando apliquen, doctests para ejemplos |
| Go | `go/doc/comment` | Comentario completo asociado al símbolo exportado; package docs y formato procesable por `go doc` |
| Python | PEP 257 | Docstring en módulo, clase, función o método; resumen imperativo y detalle cuando el contrato lo exija |
| TypeScript | TSDoc | `/** */` parseable, tags estandarizados y tipos no repetidos innecesariamente |
| Java | Javadoc Doc Comment Specification | Contrato público, `@param`, `@return`, `@throws` cuando aporten información; doclint |
| C# | XML documentation comments | `///`, `summary`, `param`, `returns`, `exception`; warning CS1591 según política |

Los estándares definen forma y semántica documental; las herramientas concretas deben seleccionarse por el perfil detectado del proyecto. No se debe imponer Rust/Cargo a otros stacks.

### 7.3 Ratchet seguro

1. Aplicar solo al diff del ciclo y a superficies públicas modificadas.
2. Empezar en modo advisory para reglas nuevas.
3. Medir precisión, falsos positivos y exenciones.
4. Convertir en bloqueo solo reglas calibradas.
5. Permitir overrides versionados por proyecto, con hash del contrato usado.

No se recomienda bloquear desde el primer día por ausencia de docs en todo símbolo público. Eso incentivaría documentación vacía y castigaría repositorios con políticas distintas.

## 8. Evaluación de prompts, agents y skills

### 8.1 Unidad evaluada

Un "agente" evaluable es la combinación de:

- Modelo y versión.
- Prompt/skill bundle y hash.
- Herramientas y permisos.
- Orchestrator/harness.
- Estado inicial del repositorio.
- Task y acceptance criteria.

Evaluar solo el texto del prompt oculta gran parte del comportamiento.

### 8.2 Suite mínima

| Suite | Pregunta | Meta |
|---|---|---|
| Capability | ¿Puede resolver casos difíciles nuevos? | Mejorar progresivamente |
| Regression | ¿Conserva comportamientos ya logrados? | Casi 100% |
| Adversarial | ¿Evita stubs, fake success y evidencia fabricada? | Cero escapes críticos |
| Routing | ¿Carga fase/skill/herramienta correctas? | Precisión alta, contexto mínimo |
| Contract | ¿Envelope, hashes y transiciones son válidos? | 100% determinista |
| CLI contract | ¿Comandos, flags, JSON, ownership y número de llamadas son correctos? | Cero invocaciones inválidas o errores ocultos |
| Communication | ¿El reporte es correcto, comprensible y accionable? | Calibración humana |

Cada caso debe conservar task, trials, transcript/tool trace, outcome final y graders. Para componentes estocásticos se necesitan varios trials; para gates importa consistencia (`pass^k`), no acertar una vez (`pass@k`).

### 8.3 Golden dataset

El dataset actual tiene pocos casos y el runner deja resultados `PENDING`; no ejecuta de extremo a extremo los agentes verificadores. Debe evolucionar así:

1. Hacer ejecutables los casos existentes.
2. Añadir un caso por bug real del workflow o verificador.
3. Mantener etiquetas expertas y un conjunto retenido.
4. Medir precision, recall, F1 y tasa de falso bloqueo por `rule_id`.
5. Probar bypasses: renombrados, cambios de casing, multiline comments, production/test wiring, hardcodes alternativos.
6. Añadir mutantes de contrato CLI: subcomando/flag/campo inexistente, falta de `--cycle`, text parsing, error oculto, lease stale y evidence booleana.
7. Establecer un budget esperado de llamadas por fase y fallar cuando wrappers/workers repitan consultas que pertenecen al orchestrator/coordinator.
8. Separar evaluación externa del kernel; SDDK registra resultados y hashes, pero no necesita convertirse en una plataforma estadística completa.

### 8.4 Refactor de instrucciones

Solo después de establecer el baseline de evals:

| Superficie | Debe contener |
|---|---|
| Agent wrapper | Rol host, modelo, permisos, herramientas y puntero al contrato |
| Skill | Trigger, adaptación y delegation; procedimiento mínimo |
| Phase prompt | Pasos ordenados, completion criteria y output contract |
| Shared contract | Regla transversal con una autoridad única |
| Reference | Material consultado solo por una rama |

Pruning recomendado:

- Eliminar reglas duplicadas entre wrapper, skill y phase prompt.
- Mover tablas por stack/lenguaje a references cargables bajo demanda.
- Sustituir prohibiciones repetidas por objetivos positivos, manteniendo solo guardrails duros.
- Generar tablas/documentación desde contratos máquina cuando sea posible.
- Eliminar instrucciones que solo repiten información consultable en el CLI o filesystem.
- Añadir completion criteria observables a cada step.

## 9. Evolución por fases SDDK

### 9.1 Propose

El proposal debe seguir siendo corto. No debe incorporar un informe técnico completo ni exigir C4 a todo cambio.

Añadir dos bloques compactos al contrato de salida del agente; son estructura de instrucciones, no schemas nuevos del kernel:

```yaml
quality_intent:
  production_surfaces: []
  changed_public_apis: []
  readiness_dimensions: []
  required_real_boundaries: []
architecture_impact:
  level: none | local | boundary | deployable
  evidence: []
```

Cuando `architecture_impact` sea `boundary|deployable`, producir además un `architecture-intent` observado/planificado y una preview. La preview puede verse durante propose, pero la elaboración detallada pertenece a design.

El developer ve desde el inicio:

- Qué caminos deben ser productivos.
- Qué dependencias reales se probarán.
- Qué documentación pública cambia.
- Qué dimensiones de readiness serán obligatorias.
- Qué arquitectura se espera modificar.

### 9.2 Spec

- Incluir negative controls y escenarios alternativos donde exista riesgo de hardcode.
- Declarar qué resultado no sería aceptable aunque los tests básicos pasen.
- Identificar límites que requieren contrato/integración real.
- Mantener comportamiento observable; no prescribir detalles de implementación innecesarios.

### 9.3 Design

- Convertir architecture intent en modelo estructurado.
- Definir composition roots y perfiles productivos/no productivos.
- Declarar error handling, migraciones, seguridad, observabilidad y rollback aplicables.
- Elegir perfil documental por stack.
- Generar vistas C4 solo desde modelo/evidencia con IDs estables.

### 9.4 Tasks

- Asociar cada work unit con escenarios, paths productivos y gates aplicables.
- Mantener hardening y documentación con el código que los necesita.
- Incluir explícitamente pruebas reales de boundaries y negative controls.
- No crear una tarea genérica final de "production ready" que permita diferir todo.

### 9.5 Apply

- Ejecutar con herramientas existentes el mismo checklist de verify sobre cada slice en modo rápido/advisory.
- No duplicar reglas en prose; consumir contratos versionados.
- Incluir resultados del slice, subject y evidencia en el handoff que ya produzca la fase.
- Exigir que mocks/fakes permanezcan en tests o perfiles no productivos explícitos.
- Promover un finding a bloqueo antes del commit cuando sea de alta precisión.

### 9.6 Verify

- Ejecutar una vez los gates deterministas sobre el subject final.
- Producir findings estructurados, no solo Markdown.
- Comparar matriz requirement -> production path -> test -> evidence.
- Ejecutar anti-placeholder, docs por lenguaje, boundary fidelity y readiness.
- Delegar a lentes solo juicio semántico; no repetir comandos.
- Mantener el ownership de debt-verify separado y declarar `INCONCLUSIVE` si la evidencia runtime disponible no demuestra los receipts requeridos.

### 9.7 Archive

- Consumir las salidas estructuradas disponibles; no volver a inventar hechos desde prosa.
- Renderizar Markdown/HTML mediante la plantilla de instrucciones vigente.
- Incorporar architecture baseline/intent/actual/delta cuando existan.
- Marcar como `unavailable` cualquier evidencia o vista que no exista; no fabricarla.
- Mantener `/tmp` como copia de presentación, nunca autoridad.

### 9.8 Matriz CLI por step

La secuencia objetivo se expresa en términos de comandos existentes en la baseline. Todos los outputs consumidos por agentes usan JSON y todos los comandos conservan `--root` y `--scope` explícitos.

| Step | Owner | Secuencia mínima | Optimización y guardas |
|---|---|---|---|
| Orchestrator / resume | Orchestrator | `adopt status` + `knowledge status`; con `cycle_id` conocido, `cycle status` y `cycle artifacts-dir` una vez | En arranque sin ciclo, dejar artifacts dir en `null` y pasar a triage; no repetir `knowledge path`, ni usar lock status sin cycle, ni fabricar active cycle desde memoria |
| Init | Init coordinator | Reutilizar preflight; `adopt apply` solo si status demuestra ausencia; refrescar `knowledge status` tras mutación | No ejecutar status/path varias veces; persistir el JSON observado en el envelope |
| Explore | Phase coordinator | Status fresco si el token envejeció -> evaluate `exploration-sufficient` -> transition -> ledger verify | Evidence incluye report hash, scope y criterios observados; workers no llaman CLI |
| Propose | Phase coordinator | `artifact store --format json` | Validar `artifact_id` y `sha256`; no usar ledger verify para demostrar una operación que no escribió ledger |
| Spec | Phase coordinator | Status fresco si procede -> evaluate `requirements-testable` -> transition -> ledger verify | Evidence enlaza proposal/spec hashes, coverage y subject; transición depende del path ya resuelto |
| Design | Phase coordinator | Status fresco si procede -> evaluate `architecture-consistent` -> transition -> ledger verify | Evidence enlaza spec/design hashes, invariants y checks; C4 renderer no añade transición |
| Tasks | Phase coordinator | Status fresco si procede -> evaluate `plan-executable` -> transition -> ledger verify | Evidence enlaza tasks/spec/design, dependencias y work-unit readiness |
| Apply | Phase coordinator | Renovar lease solo si expirará antes de mutar -> evaluate `implementation-complete` -> transition -> ledger verify | Evidence sustituye booleano por subject, commits/diff, tasks y receipt hash; no renovar en cada slice |
| Verify | Verify coordinator | Status JSON -> renovación condicional -> evaluate gates -> transition para PASS/FAIL -> ledger verify | Ya exige evidence rica; añadir JSON a todas las llamadas y evitar que lenses repitan lifecycle queries |
| Debt-verify | Debt coordinator | Sin transición legacy propia; `debt-verify.md` declara handoff `specification_only` | No fabricar receipts ni invocar debt CLI nominal como prueba; devolver `INCONCLUSIVE` si falta enforcement demostrable |
| Coherence | Coherence coordinator | Leer artifacts/hashes; ledger verify solo cuando integridad de ledger sea un input explícito del check | No usar ledger verify como prueba de coherencia semántica o persistencia Engram |
| Release | Release coordinator | Release local -> status JSON -> evaluate gates -> transition -> ledger verify | Reusar receipts reales de Git; no consultar CI cloud como gate ni repetir estado remoto sin cambio local |
| Archive | Archive coordinator | Status/paths -> vault validate JSON -> ledger verify pre -> evaluate gates -> transition -> ledger verify post | Las dos verificaciones tienen funciones distintas; usar `--vault`, no flags inventados; no reacquire lease para satisfacer templates |

Para fases largas, "status fresco si procede" significa: no ha habido compaction/restart, el `updated_at` coincide con el state token y el lease seguirá vivo hasta completar la mutación. Si cualquiera falla, se consulta de nuevo; si no, se reutiliza el token y el fencing protege la transición.

## 10. C4 + LikeC4 como skill opcional

### 10.1 Pipeline

```text
Grafo canónico observado
        +
Overlay planificado con evidence refs
        |
        v
Diff semántico del capability externo por IDs/relaciones
        |
        +--> tabla/SVG fallback
        |
        +--> adaptador a LikeC4 DSL
                 |
                 +--> likec4 validate
                 +--> single-file HTML
                 +--> PNG/JSON/Mermaid/etc.
```

### 10.2 Fases

| Fase | Salida |
|---|---|
| Propose | Baseline resumida + áreas planificadas, solo si hay impacto arquitectónico |
| Design | Modelo planificado y vistas C4 completas |
| Verify | Consume snapshot observado y delta producidos por el capability externo; verify no edita el modelo |
| Archive | Bundle navegable, SVG/tablas fallback y enlaces a evidencia |

### 10.3 Contrato de manifest

```yaml
schema_version: string
cycle_id: string
phase: propose | design | verify | archive
subject_sha: sha
baseline_sha: sha|null
graph_revision: string
semantic_status: valid | insufficient_evidence | invalid
render_status: rendered | unavailable | failed
elements_total: integer
relationships_total: integer
accepted_evidence_coverage: number
intent_ref: artifact-ref|null
delta_ref: artifact-ref|null
outputs:
  - {kind: likec4-html|svg|json|table-html, path: string, sha256: string}
tool_versions: {}
diagnostics: []
```

El fallo del renderer no debe cambiar el veredicto semántico. La ausencia de evidencia arquitectónica requerida sí puede bloquear design/verify. Este manifest es un contrato de salida de la skill; esta iniciativa no añade registro, schemas ni persistencia al kernel.

### 10.4 Disponibilidad

- LikeC4/archctl como capability/pack opcional, no dependencia de `sddk-domain` o `sddk-engine`.
- Fallback Markdown/HTML tabular generado por la skill cuando LikeC4 no esté disponible.
- No abrir navegador durante automatización salvo petición explícita del usuario.
- Build LikeC4 recomendado: relocatable, hash routing y single-file.
- Preflight explícito de toolchain: LikeC4 `1.59.2` declara Node `>=22.22.3`; su ausencia o incompatibilidad degrada al fallback.
- Nunca reconstruir semántica convirtiendo Mermaid de vuelta a un modelo.

## 11. Frontera con el roadmap activo

El roadmap canónico confirma que runtime y control plane ya tienen una línea de trabajo propia. Este evolutivo no debe adelantarse ni competir con ella.

| Necesidad detectada | Roadmap propietario | Regla para esta iniciativa |
|---|---|---|
| WorkflowIR, gates y ejecución dinámica | Fases 3-4 | Solo documentar la limitación; no tocar runtime ni schemas |
| Debt report y verdict tipados | Fase 8 + slice DEBT | No decidir lifecycle ni crear evaluator; no fabricar receipts |
| Evaluación A-full/adaptive | Fase 9 | Aportar golden tasks y graders de agentes reutilizables |
| Catálogo causal y `sddk why` | Fase 10 | No crear eventos, CLI ni proyecciones |
| Cockpit y vistas | Fase 11 | Mejorar únicamente contenido y plantilla de reportes |
| Lifecycle/provenance de artefactos | Fase 14 | No modificar CAS, ledger, storage ni control plane |

El worktree parte de `v1.41.0`; `main` ya publicó `v1.42.0` en el tag `fd55295` y cerró su indexado documental en `a4f2476`. Los archivos CLI/instrucciones citados en la auditoría (`cycle.rs`, `vault_cmd.rs`, `lib.rs`, `sddk-cycle-resume` y MCW) no cambiaron entre ambas revisiones, por lo que los hallazgos siguen vigentes. Esta rama no modifica `crates/sddk-engine/`, ADRs de esos ciclos ni su superficie de pruebas.

### 11.1 Mejoras transferibles al roadmap propietario

El contraste con `SPEC-024`, `ADR-035`, `ADR-039`, `SPEC-029`, `SPEC-036`, `TEST-STRATEGY.md` e `IMPLEMENTATION-BACKLOG.md` identifica seis deltas que mejoran el roadmap sin cambiar su arquitectura:

| Destino | Delta propuesto | Evidencia de que falta o está incompleto | Criterio de aceptación sugerido |
|---|---|---|---|
| Fases 3-4 / M3-M3b | Añadir una consulta agregada y autoritativa de reanudación que, sin requerir conocimiento previo fabricado, devuelva active cycle, phase/path/status, subject, artifact dir y lease | La baseline exige `--cycle` para status/lock, mientras MCW necesita descubrir el ciclo activo; los prompts compensan con fallbacks y campos que la CLI no devuelve | Restart/compaction reconstruye un state token completo con una sola respuesta versionada; ausencia, error de invocación y corrupción son estados distintos y fail-closed |
| Fase 0 / M0 y fase 9 / M12 | Ampliar la identidad de cada evaluación con hash del bundle de prompts/skills, modelo/versión, tools/permisos, estado inicial, task, trial, trace y grader; separar suites capability, regression, adversarial, routing, contract y communication; medir `pass^k` | `SPEC-024` define niveles, golden tasks y métricas, pero no fija identidad completa, múltiples trials, graders separados ni estas seis suites | Dos bundles comparados sobre el mismo fixture son reproducibles y una degradación intencional aparece en regression/adversarial sin quedar oculta por un único trial afortunado |
| Fase 8 / ADR-039 | Añadir un contrato de evidencia multi-stack para anti-placeholder, documentación útil, boundary fidelity, production wiring y negative controls; los detectores producen candidates y el juicio semántico no anula fallos deterministas | ADR-039 enumera señales de riesgo y checks deterministas, pero no define findings, localización, confianza, exenciones ni calibración por stack | Fixtures Rust, Go, Python y TypeScript demuestran detección, exenciones y falso bloqueo por `rule_id`; una herramienta no aplicable produce `INCONCLUSIVE`, no falso `PASS` |
| Fase 2 + fase 14 / M13 | Generalizar el catálogo de artefactos más allá de supply chain: manifest de reportes, evento genérico versionado por decidir y consulta por proyecto/ciclo/fase/kind; mantener bytes en CAS y catálogo como proyección reconstruible | El event catalog contiene `artifact.built|promoted|deployed|lifecycle.changed` y M13 incluye lifecycle, pero no cubre claramente reportes de fases ni su descubrimiento | Borrar y reconstruir la proyección produce el mismo catálogo; todo reporte localizable conserva hash, subject, producer, locale y evidence refs |
| Fase 10-11 / M8-M9 | Completar el contrato C4/UML con IDs estables, `observed|planned|actual`, delta semántico, evidence refs y coverage; el renderer rico degrada a tabla sin alterar el veredicto | `SPEC-036` admite metadata C4/UML y fallback, y ADR-039 menciona graph delta, pero no define la semántica intent/actual ni la cobertura de evidencia | Una relación planificada ausente aparece como `planned_but_missing` con o sin LikeC4; fallo del renderer no cambia el resultado semántico |
| Fase 11 / M9 | Añadir presentación localizada al Cockpit/reportes: BCP 47, español por defecto, progressive disclosure, perfiles `novice|standard|expert` y acciones de recuperación; mantener campos máquina sin traducir | `SPEC-029` ya exige HTML estático, offline y sin CDN, pero no define locale, audiencia ni comprensión progresiva | El mismo snapshot genera español por defecto y otro locale solicitado sin cambiar IDs/veredictos; el resumen permite localizar verdict, impacto y siguiente acción |

Estas son propuestas para refinar las specs o backlog propietarios cuando sus fases se planifiquen. No son work units de `feat/evolutivo-prompts-agents-skills` y no autorizan cambios Rust desde esta rama.

### 11.2 Hallazgos ya cubiertos que no deben duplicarse

- `DebtReportV2`, validator Rust, CAS binding y receipt ligado a report/subject/baseline/policy/evaluator ya están en M13b, ADR-040 y SPEC-041.
- Cockpit estático, self-contained y sin CDN ya está en SPEC-029.
- Golden capability fixtures, comparación A-full/adaptive y ablation runner ya están en M12/M11b.
- C4/UML como renderer de pack y fallback tabular ya están en SPEC-036; solo falta precisar semántica y evidencia.
- Active Graph, `sddk why` y proyecciones reconstruibles ya pertenecen a M8/fase 10.

## 12. Idioma y comunicación para developers novatos

### 12.1 Resolución de locale

Orden aplicable dentro de la capa de instrucciones:

1. `report_locale` explícito en la entrada del agente o prompt de fase.
2. Preferencia explícita disponible en el contexto del proyecto.
3. Fallback fijo `es`.

No inferir automáticamente el locale persistente desde `$LANG` ni desde una frase aislada del chat. Las instrucciones deben aceptar language tags BCP 47 y aplicar fallback predecible. Añadir flags CLI, variables de entorno o configuración runtime queda fuera de alcance.

### 12.2 Un solo hecho, varias profundidades

El informe HTML debe usar progressive disclosure, no generar verdades diferentes por audiencia:

1. **Resumen:** qué cambió, por qué importa y verdict.
2. **Guía:** cómo funciona, ejemplo y próximos pasos.
3. **Detalle técnico:** arquitectura, contratos y trade-offs.
4. **Evidencia:** comandos, hashes, findings, logs y receipts.

`audience: novice|standard|expert` decide qué panel aparece expandido, nunca qué evidencia existe.

### 12.3 Reglas de lenguaje

- Español profesional y claro por defecto.
- Identificadores, comandos, paths, APIs y código sin traducir.
- Acrónimo explicado en su primera aparición.
- Cada fallo incluye impacto y acción concreta de recuperación.
- Cada `N/A` incluye por qué no aplica.
- Glosario derivado del domain language del ciclo.
- Evitar métricas sin interpretación y tablas sin decisión asociada.
- Enlaces de evidencia disponibles, pero fuera del resumen principal.

## 13. Roadmap recomendado

Todos los ciclos de esta sección son deliberadamente independientes del roadmap Rust. Si una work unit requiere tocar una superficie fuera del alcance definido en 2.1, queda bloqueada y se transfiere al roadmap propietario.

### Ciclo 0: harness de evaluación de instrucciones

**Objetivo:** poder cambiar prompts/skills/agentes sin volar a ciegas.

- Ejecutar el golden dataset de extremo a extremo.
- Registrar bundle hash, modelo, tools, task, trials, trace y outcome.
- Añadir casos para contradicciones y escapes ya encontrados.
- Añadir casos retenidos para evidencia fabricada, tools no ejecutadas, handoffs contradictorios y falso éxito.
- Capturar trazas y budgets CLI por step; añadir mutantes para subcomandos, flags, campos JSON, leases y fallbacks inválidos.
- Publicar precision/recall/F1 y falsos bloqueos.

**Criterio de cierre:** una modificación intencionalmente mala de verify o del contrato CLI es detectada por la suite.

### Ciclo 1: autoridad y contratos de instrucciones

**Objetivo:** eliminar duplicación y drift entre wrapper, skill, prompt, contrato compartido y uso de CLI.

- Inventariar la autoridad de cada regla y moverla a una sola superficie.
- Dejar wrappers como host/configuración y skills como trigger/adaptación/delegación.
- Añadir entradas, pasos, evidencia requerida, completion criteria y salida observable a cada prompt de fase.
- Mover matrices por stack y lenguaje a referencias bajo demanda.
- Marcar explícitamente limitaciones runtime que el agente no puede resolver ni reinterpretar.
- Crear una autoridad compartida de uso CLI con owner, secuencia, JSON, freshness, lease y error policy por step.
- Corregir ejemplos incompatibles de MCW y `sddk-cycle-resume`; prohibir fallbacks que conviertan errores en ausencia.
- Sustituir evidence booleana por subject, artifact hashes, checks y resultados observados.

**Criterio de cierre:** contract tests y evals detectan duplicación de autoridad, referencias stale, criterios no observables y cualquier comando/flag/campo incompatible con la CLI declarada.

### Ciclo 2: validaciones de agentes IA

**Objetivo:** mejorar detección, adjudicación y explicación sin añadir scanners compilados.

- Incorporar al prompt de verify la tubería L0-L6 y la regla de no anular fallos deterministas.
- Añadir checklists y golden tasks multi-stack para placeholders, hardcodes, docs útiles, wiring fake y negative controls.
- Exigir findings con localización, evidencia, confianza, clasificación y exención justificada.
- Separar generador, evaluador y juez adversarial en el harness.
- Medir escapes críticos, falsos bloqueos y consistencia `pass^k`.
- Verificar que cada phase coordinator respeta su budget de llamadas, no repite queries de workers y conserva fencing/ledger safety.

**Criterio de cierre:** fixtures multi-stack demuestran que los agentes detectan y explican defectos reales sin afirmar capacidades que las herramientas existentes no poseen.

### Ciclo 3: reporting localizado y arquitectura opcional

**Objetivo:** mejorar la comunicación sin crear renderer, catálogo ni persistencia runtime.

- Añadir `report_locale` a contratos de entrada/salida de prompts, con `es` por defecto.
- Refactorizar plantillas con progressive disclosure y acciones concretas para developers novatos.
- Corregir la denominación de HTML como render, no artefacto canónico.
- Definir una skill C4/LikeC4 opcional con preflight, validación y fallback Markdown/HTML.
- Distinguir siempre baseline observada, intención planificada, estado actual y evidencia ausente.

**Criterio de cierre:** el mismo conjunto de hechos produce reportes comprensibles en español por defecto y en otro locale solicitado; la ausencia de LikeC4 conserva un fallback útil y explícito.

### Ciclo 4: pruning de prompts, skills y agentes

**Objetivo:** reducir contexto y drift con seguridad.

- Inventario de autoridades y duplicaciones.
- Progressive disclosure por branch.
- Generación de proyecciones desde contratos.
- Retirada de no-ops y referencias stale.
- Retirada de queries CLI redundantes demostradas por trace, sin eliminar lecturas frescas de seguridad.
- Una concernencia por work unit/commit.
- Comparación contra baseline de evals.

**Criterio de cierre:** menor carga de contexto sin degradación estadísticamente visible en regression/adversarial suites.

## 14. Decisiones recomendadas

| Decisión | Recomendación | Motivo |
|---|---|---|
| Añadir reglas duplicadas a verify | No | Primero clarificar autoridad y evidencia observable |
| Scope "sesión" | No como autoridad | Usar cycle + base/head reproducible |
| Debt dentro de verify | No | Ownership y lifecycle distintos |
| LLM puede anular scanner | No | Rompe reproducibilidad |
| Bloquear toda ausencia de docs | No inicialmente | Incentiva comentarios vacíos y falsos positivos |
| Reportes cada N minutos | No | Coste, ruido y Goodhart; usar phase events y on-demand |
| Español por defecto | Sí | Requisito de comunicación |
| Idioma configurable en prompts | Sí | Sin traducir contratos máquina ni tocar CLI/runtime |
| Cambiar Rust/runtime desde este evolutivo | No | Ya tiene roadmap y trabajo activo propios |
| Añadir catálogo, eventos o control plane | No | Fases 10, 11 y 14 del roadmap canónico |
| LikeC4 en kernel | No | Skill opcional externa |
| LikeC4 como autoridad del diff | No | Solo presenta un modelo/evidencia producidos fuera del renderer |
| C4 obligatorio para todo proposal | No | Activación por impacto arquitectónico |
| Plataforma A/B dentro del CLI | No | Harness externo de evaluación de agentes |
| Text output para decisiones de agentes | No | JSON evita parsing frágil y permite validar shape |
| `{"checked": true}` como gate evidence | No | Un receipt firmado no prueba una comprobación que no se describe |
| Una query lifecycle por worker/lens | No | Orchestrator y coordinator poseen estado y mutaciones |
| Minimizar llamadas sacrificando freshness | No | Fencing, phase legality e integridad siguen siendo gates |

## 15. Riesgos de la evolución

| Riesgo | Mitigación |
|---|---|
| Interferir con el trabajo Rust activo | Allowlist de paths y gate que rechace cambios fuera de prompts/agentes/skills/evals |
| Mega-refactor de instrucciones | Ciclos pequeños, eval baseline primero |
| Duplicar gates/rules | Registry y rule IDs únicos; ownership explícito |
| Falsos positivos de documentación | Advisory + ratchet + corpus por lenguaje |
| LikeC4 decorativo | IDs/evidence refs/graph revision obligatorios |
| Reportes que nadie lee | Progressive disclosure y resumen accionable |
| Instrucciones prometen enforcement inexistente | Marcar límites y devolver `INCONCLUSIVE` en vez de fabricar evidencia |
| Receipt firmado se interpreta como evidencia semántica | La firma cubre receipt/gate/transition/plan hash, no demuestra el contenido de `evidence`; exigir subject, artifact hash y checks antes de pedir `passed` |
| Error CLI ocultado como estado ausente | Clasificar errores y prohibir fallbacks como los de MCW lock status y `sddk-cycle-resume` project/vault sobre consultas autoritativas |
| Optimización elimina una lectura de seguridad | Budget por frontera, no un mínimo global de comandos |
| Locale altera contratos máquina | Localizar solo texto de presentación; mantener IDs y campos estables |
| Novato confunde warning con fallo | Explicar impacto, confianza y acción |
| Agente aprende a evitar palabras prohibidas | Outcome, reachability y negative controls |

## 16. Métricas útiles

### Verify

- Escape rate de placeholders confirmados.
- Precision/recall por `rule_id`.
- Tasa de falsos bloqueos y exenciones.
- Porcentaje de boundaries modificados con prueba real.
- Mutantes relevantes supervivientes.
- Gates `INCONCLUSIVE` por falta de evidencia.

### Prompts/agentes

- `pass^k` por suite y path.
- Regresiones por bundle/model version.
- Tool-selection accuracy.
- Evidencia fabricada o comando afirmado/no ejecutado.
- Tokens/contexto por fase.
- Correcciones por contradicción de handoff.

### Uso de CLI

- Invocaciones inválidas por bundle: objetivo 0.
- Porcentaje de outputs máquina consumidos como JSON: objetivo 100% cuando exista `--format json`.
- Queries duplicadas por phase trace y llamadas lifecycle ejecutadas por workers/lenses.
- Gate receipts con evidence booleana o sin subject/artifact hash: objetivo 0.
- Errores autoritativos convertidos por fallback en estado ausente: objetivo 0.
- Renovaciones de lease innecesarias y transiciones rechazadas por lease stale.
- Ledger verifies por frontera, distinguiendo pre/post archive de duplicación real.

### Comunicación

- Tiempo hasta encontrar verdict, riesgo y siguiente acción.
- Secciones abiertas por audiencia.
- Links de evidencia rotos o afirmaciones sin referencia.
- Reportes con locale incorrecto o términos máquina traducidos.
- Feedback humano de comprensión, separado de satisfacción visual.

## 17. Fuentes primarias y oficiales

### Agentes y evaluaciones

- Anthropic, **Effective context engineering for AI agents** (2025-09-29): https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- Anthropic, **Demystifying evals for AI agents** (2026-01-09): https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
- Anthropic, **Equipping agents for the real world with Agent Skills** (2025-10-16): https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills
- OpenAI, **Evaluation best practices**: https://developers.openai.com/api/docs/guides/evaluation-best-practices
- OpenAI, **Graders**: https://developers.openai.com/api/docs/guides/graders
- Google Engineering Practices, **What to look for in a code review**: https://google.github.io/eng-practices/review/reviewer/looking-for.html
- Google, **Software Engineering at Google: Test Doubles**: https://abseil.io/resources/swe-book/html/ch13.html

### Seguridad, evidencia y provenance

- NIST SP 800-218, **Secure Software Development Framework 1.1**: https://csrc.nist.gov/pubs/sp/800/218/final
- OWASP, **Application Security Verification Standard 5.0**: https://owasp.org/www-project-application-security-verification-standard/
- OWASP GenAI, **LLM Top 10 2025**: https://genai.owasp.org/llm-top-10/
- SLSA v1.2, **Verifying artifacts**: https://slsa.dev/spec/v1.2/verifying-artifacts
- W3C, **Trace Context**: https://www.w3.org/TR/trace-context/
- OpenTelemetry, **Semantic conventions for events**: https://opentelemetry.io/docs/specs/semconv/general/events/
- IETF, **RFC 5646 - Tags for Identifying Languages**: https://www.rfc-editor.org/rfc/rfc5646
- IETF, **RFC 4647 - Matching of Language Tags**: https://www.rfc-editor.org/rfc/rfc4647

### Documentación por lenguaje

- Rust Project, **The rustdoc book**: https://doc.rust-lang.org/rustdoc/
- Go Project, **Go Doc Comments**: https://go.dev/doc/comment
- Python, **PEP 257 - Docstring Conventions**: https://peps.python.org/pep-0257/
- TSDoc: https://tsdoc.org/
- Oracle, **Javadoc Documentation Comment Specification**: https://docs.oracle.com/en/java/javase/26/docs/specs/javadoc/doc-comment-spec.html
- Microsoft, **XML documentation comments**: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/xmldoc/

### Arquitectura

- Simon Brown, **The C4 model**: https://c4model.com/
- LikeC4, **Views**: https://likec4.dev/dsl/views/
- LikeC4, **CLI**: https://likec4.dev/tooling/cli/
- npm Registry, **LikeC4 latest package metadata**: https://registry.npmjs.org/likec4/latest

## 18. Siguiente paso

Iniciar el **Ciclo 0** capturando un baseline ejecutable del bundle actual sobre golden tasks de capability, regresión, adversarial, routing, contrato, CLI y comunicación. La traza debe registrar owner, argv, output format, exit code, campos consumidos y frontera de estado. El primer cambio de instrucciones solo comienza cuando la suite puede detectar una degradación intencional de `verify` o una invocación CLI incompatible.

Después se refactorizan wrapper, skill y prompt de fase en slices pequeños, con una allowlist que impida modificar `crates/`, CLI, runtime, schemas, storage, ledger o control plane. Cualquier bloqueo en esas superficies se entrega como dependencia al roadmap canónico, no se resuelve en esta rama.
