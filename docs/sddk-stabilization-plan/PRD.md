# PRD — SDDK v3.6 Stabilization

**Estado:** aceptada — entregado entre `v0.1.0` y `v0.14.0`
**Versión:** 0.1
**Fecha:** 2026-08-03
**Producto:** SDDK Framework

## 1. Visión

SDDK debe evolucionar desde una colección de agentes, skills, prompts, convenciones y scripts parcialmente solapados hacia una plataforma local de desarrollo dirigido por especificaciones en la que:

- Los agentes realizan razonamiento, investigación y generación de contenido.
- Un CLI Rust controla estados, transiciones, gates, permisos y efectos externos.
- Cada operación importante es trazable, idempotente y reconciliable.
- El conocimiento permanece en un vault Markdown compatible con Obsidian.
- El sistema puede reconstruir sus índices e informes a partir de fuentes canónicas.

## 2. Problema

El workflow actual contiene lógica autoritativa distribuida entre documentos y agentes. Esto provoca:

- Reglas duplicadas y contradictorias.
- Paths y marcadores de adopción inconsistentes.
- Comandos shell incrustados en prompts sin garantías de atomicidad.
- Estados difíciles de reconstruir tras una interrupción.
- Dependencia excesiva de convenciones no verificables.
- Referencias a plugins o artefactos que pueden no existir.
- Acoplamiento directo a GitHub, `main`, `origin` y `gh`.
- Escasa capacidad para explicar causalmente por qué ocurrió una operación.

## 3. Objetivos

### O1. Fuente única del workflow

Definir estados, fases, transiciones, gates, artefactos y políticas en contratos estructurados versionados.

### O2. Capa determinista

Implementar un CLI Rust que sea el único propietario de:

- Estado de los ciclos.
- Locks.
- Transiciones.
- Gates.
- Capacidades externas.
- Receipts.
- Trazabilidad.

### O3. Conocimiento portable

Mantener el conocimiento canónico en Markdown con frontmatter, IDs estables y relaciones tipadas.

### O4. Recuperación y reconciliación

Permitir reanudar el workflow después de fallos, cierres del proceso o respuestas externas desconocidas.

### O5. Migración incremental

Adoptar la nueva arquitectura sin reescribir simultáneamente todos los agentes.

## 4. No objetivos de v3.6

- Sustituir el vault por una base de datos de grafos.
- Cargar plugins nativos de terceros sin confianza.
- Crear un daemon permanente obligatorio.
- Implementar una interfaz web compleja.
- Autoactualizar silenciosamente el CLI.
- Permitir autoaprobación de acciones destructivas.
- Incorporar nuevos agentes antes de estabilizar los existentes.

## 5. Usuarios

### Desarrollador individual

Quiere iniciar y ejecutar ciclos SDDK con una trazabilidad clara sin contaminar el repositorio.

### Arquitecto o responsable técnico

Quiere revisar requisitos, decisiones, riesgos, implementación y evidencias de verificación.

### Agente LLM

Necesita contratos explícitos para conocer qué puede proponer, qué artefactos debe entregar y qué capacidades puede solicitar.

### Mantenedor de SDDK

Necesita evolucionar workflows y packs sin introducir contradicciones entre documentación e implementación.

## 6. Casos de uso principales

### UC-01 Adopción de un proyecto

```bash
sddk adopt plan --project .
sddk plan apply <plan-id>
sddk adopt status
```

La adopción debe ser atómica, reparable e independiente del nombre del directorio local.

### UC-02 Inicio de ciclo

```bash
sddk cycle start --change add-oauth
```

El CLI resuelve la identidad, comprueba locks, crea el manifiesto del ciclo y registra el frame inicial.

### UC-03 Finalización de fase por un agente

```bash
sddk phase complete design --result agent-result.json
```

El resultado se valida contra schema, se verifican artefactos y solo entonces se evalúa la transición.

### UC-04 Ejecución de una capacidad

```bash
sddk capability plan git.create_branch --args args.json
sddk plan apply <plan-id>
```

La operación queda registrada antes y después del efecto.

### UC-05 Recuperación

```bash
sddk reconcile
```

El CLI resuelve efectos en estado desconocido consultando la realidad externa.

### UC-06 Consulta del conocimiento

```bash
sddk vault search "release policy"
sddk vault backlinks adr:sddk:0002
sddk vault validate
```

## 7. Requisitos funcionales

### RF-001 Identidad estable

El sistema debe separar:

- `project_id`: identidad lógica estable.
- `workspace_id`: checkout o worktree concreto.
- `display_name`: nombre humano.

El basename no puede ser la identidad.

### RF-002 Paths XDG

Por defecto:

```text
$XDG_DATA_HOME/sddk/projects/<project-id>/
$XDG_STATE_HOME/sddk/projects/<project-id>/
$XDG_CACHE_HOME/sddk/
```

### RF-003 Ledger

Cada comando debe producir eventos secuenciales con:

- `event_id`.
- `sequence`.
- `frame_id`.
- `command_id`.
- `actor`.
- estado anterior y posterior.
- hash del evento anterior.
- hash del evento actual.

### RF-004 Máquina de estados

El sistema debe validar estado y fase por separado.

Estados mínimos:

```text
OPEN, BLOCKED, REMEDIATING, RELEASE_PENDING,
RELEASED, CLOSED, ABANDONED, RECOVERING
```

Fases mínimas:

```text
explore, specify, design, plan, build,
verify, review, release, archive
```

### RF-005 Plan y aplicación

Toda operación con efectos debe implementar:

```text
validate → plan → authorize → apply → verify → commit
```

### RF-006 Idempotencia

Una operación repetida con la misma `idempotency_key` no debe duplicar efectos.

### RF-007 Gateway de capacidades

Los agentes no pueden ejecutar directamente Git, shell, forge, filesystem de escritura o publicación.

### RF-008 Vault estructurado

Cada nota canónica debe declarar:

- `schema_version`.
- `id` estable.
- `type`.
- `title`.
- `status`.
- `relations` tipadas.
- `provenance`.

### RF-009 Índice del vault

SQLite debe indexar notas, propiedades, relaciones, backlinks, tags y texto completo.

### RF-010 Artefactos por contenido

Los artefactos grandes deben almacenarse mediante SHA-256 y referenciarse desde el ledger.

### RF-011 Linter

El CLI debe detectar como mínimo:

- Referencias rotas.
- Placeholders sin resolver.
- Agentes o skills inexistentes.
- Transiciones inválidas.
- Gates sin implementación.
- Artefactos sin productor o consumidor.
- Paths no canónicos.
- Documentación generada obsoleta.

### RF-012 Packs

Cada pack debe declarar manifiesto, dependencias, superficie, riesgo, clase de consecuencia y fixtures.

### RF-013 Resultados estructurados de agentes

Los agentes deben devolver JSON validable. El texto libre no puede cambiar el estado del workflow.

### RF-014 Forge abstracto

GitHub será el primer adaptador, pero el dominio debe usar una interfaz neutral.

### RF-015 Release reconciliable

El cierre del ciclo no puede ocurrir con efectos pendientes o desconocidos.

### RF-016 Control plane local de telemetría

El CLI debe agregar la telemetría de todos los proyectos adoptados del host en un store SQLite central reconstruible (`sddk telemetry ingest|aggregate|status`), permitiendo análisis cross-proyecto de ciclos, costos, lead time, bottlenecks y señales F3.

### RF-017 Dashboard HTML autocontenido

El CLI debe generar un dashboard HTML estático, sin dependencias CDN ni red, que presente KPIs, tendencias y distribuciones agregados desde el control plane (`sddk telemetry dashboard`).

### RF-018 No intrusión en repos de proyectos

El framework no escribe ficheros dentro de los repositorios git de los proyectos que lo usan. Todo el estado operativo (artefactos de ciclo, docs generados, estado de adopción, vault, ledger) vive en directorios de usuario XDG o `~/.sddk-knowledge/`.

## 8. Requisitos no funcionales

### RNF-001 Determinismo

Los fixtures del núcleo deben ejecutarse sin red, claves ni reloj real no controlado.

### RNF-002 Portabilidad

El vault debe seguir siendo utilizable sin SDDK.

### RNF-003 Seguridad

- Los secretos no se almacenan en el vault ni en el ledger.
- El shell arbitrario está deshabilitado por defecto.
- Acciones R3 y R4 requieren políticas explícitas.

### RNF-004 Rendimiento inicial

- Inicio de comandos locales: objetivo inferior a 300 ms sin tareas externas.
- Indexación incremental: solo notas cuyo hash haya cambiado.
- Consulta de backlinks: objetivo inferior a 100 ms en vaults de hasta 20 000 notas.

### RNF-005 Compatibilidad

Los cambios de schema deben incluir migración o compatibilidad de lectura.

### RNF-006 Observabilidad

Todo error debe incluir:

- Código estable.
- Contexto.
- Causa.
- Acción de recuperación sugerida.

### RNF-007 Privacidad y localidad de la telemetría

La telemetría agregada se almacena y analiza localmente. No se transmite telemetría a servicios remotos salvo decisión explícita documentada en un ADR.

### RNF-008 Versionado de bundles (modelo asdf)

El framework debe soportar múltiples versiones de bundle instaladas en `$SDDK_DATA_DIR/framework/<version>/` y resolver la activa mediante `dev use` (symlink `current`) y `.sddk-versions` por proyecto, sin que el editor dependa del repo de desarrollo.

### RNF-009 Portabilidad de paths por SO

La resolución de directorios debe funcionar en Linux (XDG), macOS (`~/Library/Application Support`, `~/Library/Caches`) y Windows (`%APPDATA%`, `%LOCALAPPDATA%`) mediante el crate `dirs`, con overrides explícitos (`SDDK_DATA_DIR`, `XDG_*`) prioritarios y sin depender de `HOME` cuando no exista.

## 9. Arquitectura lógica

```text
Agente / Usuario
      │
      ▼
   sddk-cli
      │
      ▼
 sddk-engine ────── workflow.yaml / policies
      │
      ├── sddk-storage ── SQLite
      ├── capability gateway
      ├── vault adapter ── Markdown
      └── artifact store ── SHA-256 filesystem
```

## 10. Modelo de almacenamiento

### Fuente canónica de conocimiento

Vault Markdown.

### Fuente canónica operativa

SQLite ledger y tablas de estado.

### Proyecciones reconstruibles

- Índice FTS.
- Backlinks.
- Informes HTML.
- Grafos en memoria con `petgraph`.

### Aplazado

LadybugDB queda como backend analítico opcional futuro.

## 11. Métricas de éxito

- Cero reglas operativas contradictorias detectadas por `sddk lint`.
- Cero referencias rotas en `main`.
- 100 % de transiciones cubiertas por fixtures.
- 100 % de capacidades con riesgo y consecuencia declarados.
- Replay reconstruye el mismo estado lógico.
- Un ciclo interrumpido puede recuperarse sin editar manualmente SQLite.
- Dos repositorios con igual basename no colisionan.
- El informe HTML se genera sin dependencias CDN.
- La telemetría de todos los proyectos adoptados es consultable en un solo store local.
- El dashboard se regenera determinista desde el store central sin red.
- `git status` de un proyecto adoptado es idéntico antes y después de un ciclo completo.
- Dos versiones de bundle pueden convivir y alternarse con `dev use` sin re-link del editor.
- Los paths de almacenamiento se resuelven por SO (Linux/macOS/Windows) sin depender de convenciones Unix.

## 12. Riesgos

### R-01 Sobrediseño inicial

Mitigación: comenzar con cinco crates y packs compilados.

### R-02 Migración demasiado grande

Mitigación: modo compatibilidad que envuelva agentes existentes y valide sus resultados gradualmente.

### R-03 Divergencia vault/índice

Mitigación: el índice es reconstruible y cada nota se identifica por hash.

### R-04 Acoplamiento a GitHub

Mitigación: interfaz `Forge` desde la primera implementación.

### R-05 Ledger incompleto por efectos externos

Mitigación: estados `started`, `succeeded`, `failed`, `unknown` y reconciliación.

## 13. Criterio de salida de v3.6

SDDK v3.6 estará listo cuando el flujo de adopción, inicio de ciclo, finalización de fases, ejecución Git local, verificación, trazabilidad y recuperación esté controlado por el CLI Rust y no por lógica Bash duplicada en prompts.
