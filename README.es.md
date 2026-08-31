# SDDK Framework

> **Software Development Decision Kernel** — un kernel agéntico de decisiones y workflows con grafo de conocimiento, efectos Git gobernados y verificación basada en evidencia.

[![Licencia: MIT](https://img.shields.io/badge/Licencia-MIT-yellow.svg)](LICENSE)
[![OKF Compatible](https://img.shields.io/badge/OKF-v0.2-blue.svg)](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
[![Obsidian Compatible](https://img.shields.io/badge/Obsidian-Properties_v1.4+-purple.svg)](https://obsidian.md/)

[English](README.md) | **[Español](README.es.md)**

---

## ¿Qué es SDDK?

SDDK es un framework completo de orquestación de agentes para desarrollo de software asistido por IA. Coordina agentes IA a través de un pipeline estructurado — desde la exploración hasta el release — con puertas de calidad integradas, auditoría de deuda técnica y un grafo de conocimiento que rastrea cada decisión, requisito e incidencia a lo largo de los ciclos.

### Diferenciadores clave

| Característica | Qué hace |
|----------------|----------|
| **Gobernado por decisiones** | El contexto y el riesgo seleccionan el workflow; evidencia y recibos explícitos gobiernan cada handoff. Las specs son un artefacto de aceptación, no la identidad del producto. |
| **Verificación multi-lente** | 6 lentes paralelos (compliance de spec, arquitectura, calidad de tests, coherencia de diseño, 2 jueces adversariales) + síntesis |
| **Auditoría de deuda técnica** | 5 agentes cluster (arquitectura, smells, duplicación, coupling, over-engineering) auditan deuda antes del merge a main |
| **Grafo de conocimiento** | Cada milestone, ADR, requisito, ciclo e incidencia es un nodo en un grafo de wikilinks compatible con Obsidian. Trazabilidad bidireccional completa |
| **Garantía trunk-based** | Un ciclo no puede declarar `success` hasta que los cambios estén mergeados a `main` + tag semver + trunk sincronizado. Sin abortos silenciosos |
| **Lock de serialización** | Un ciclo a la vez. El lock sobrevive caidas de sesión |
| **Agnóstico al editor** | Funciona con ZCode y OpenCode (extensible a cualquier runner de agentes) |

## Arquitectura

```
┌─────────────────────────────────────────────────────┐
│   ~/Proyectos/agentesIA/sddk-framework/    │
│           (este repositorio — framework)             │
│                                                      │
│  ┌──────────┐  ┌─────────┐  ┌────────────────────┐  │
│  │  agents/  │  │ skills/ │  │   prompts/sddk    │  │
│  │ (prompts) │  │ (tools)  │  │ (phase specs, MCW) │  │
│  └────┬─────┘  └────┬────┘  └─────────┬──────────┘  │
│       │              │                  │             │
│  ┌────┴──────────────┴──────────────────┴──────────┐ │
│  │       knowledge-template/ (plantilla vault)      │ │
│  │  milestones · adrs · specs · cycles · incidences │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─────────────┐  ┌──────────────────┐               │
│  │golden-dataset│  │ bootstrap.sh     │               │
│  │(meta-testing)│  │ (instalador)     │               │
│  └─────────────┘  └──────────────────┘               │
└─────────────────────────────────────────────────────┘
         │                                    │
    ┌────┴────┐                         ┌─────┴─────┐
    │ ZCode   │                         │ OpenCode  │
    │(symlinks)│                        │(symlinks) │
    └─────────┘                         └───────────┘
         │
    ┌────┴──────────────┐
    │ ~/.sddk-knowledge/{project}/ │  (vault por proyecto,
    │   (committed to git)       │   creado por sddk-adopt)
    └───────────────────┘
```

Las rutas actuales de agentes y skills se mantienen en el [inventario generado del repositorio](docs/generated/inventory.md).

## Inicio rápido

### Instalar

```bash
git clone https://github.com/Rubentxu/software-development-decision-kernel.git ~/Proyectos/agentesIA/sddk-framework
~/Proyectos/agentesIA/sddk-framework/bootstrap.sh --all
```

El script de bootstrap detecta automáticamente los editores instalados (ZCode, OpenCode) y crea los symlinks. Tus repos de proyecto quedan limpios — **cero archivos de documentación en tus repos de código**.

### Ejecutar un ciclo

**¿Primera vez en un proyecto?** Adóptalo primero:

```bash
cd tu-proyecto
/sddk-adopt         # una vez: auditar proyecto, plantar artefactos SDDK, crear vault de conocimiento
/sddk-init          # una vez: detectar stack, testing, modo TDD
/sddk-new add-auth  # iniciar un ciclo SDDK completo
```

**Ciclos posteriores** (proyecto ya adoptado):

```bash
cd tu-proyecto
/sddk-new <change-name>  # el vault ~/.sddk-knowledge/{project}/ ya existe; init se omite
```

El directorio `~/.sddk-knowledge/{project}/` es el marcador de adopción — su existencia significa que el proyecto está adoptado. `sddk-init` lo verifica con un simple `test -d`.

El orchestrator ejecutará:
1. **Planificación** — explore → propose → spec → design → tasks (con checkpoints interactivos)
2. **Construcción** — apply (con Strict TDD si está activado) → verify (multi-lente) → debt-verify según el path
3. **Release y archive** — publicar main → tag semver → receipts → archive manifest → sincronizar trunk

Ningún ciclo se cierra hasta que tu código está en `main`.

## Paths del workflow

| Path | Cuándo | Profundidad |
|------|--------|-------------|
| **B-direct** | Hotfix, tarea acotada | Cargar skill → ejecutar → verify ligero → release → archive |
| **A-min** | Cambio simple, contexto C2 | spec → apply → verify → debt-verify (smoke, 2 clusters) → release → archive |
| **A-lite** | Trabajo acotado, contexto C1 | propose → spec → apply → verify → debt-verify (standard, 4 clusters) → release → archive |
| **A-full** | Arquitectura, dominio nuevo, C0 | explore → propose → spec ∥ design → tasks → apply → verify (6 lentes) → debt-verify (deep, 5 clusters) → release → archive |

La profundidad de debt-verify queda fijada cuando el triage selecciona el path.
La reversibilidad influye en esa decisión inicial, pero no permite saltar el
gate después. Se acepta el coste de análisis para obtener un gate predecible;
si falta cobertura requerida, el resultado es `INCONCLUSIVE` y release se bloquea.

## Grafo de conocimiento

Cada ciclo puebla un vault de conocimiento en `~/.sddk-knowledge/{project}/` (fuera del repo):

```
mi-app/~/.sddk-knowledge/{project}/
├── _index.md              ← MOC con queries Dataview
├── milestones/
│   ├── _active.md         ← lock de serialización
│   └── M-001-auth.md      ← [[ADR-003]], [[REQ-Session]]
├── adrs/
│   └── ADR-003-jwt.md     ← [[REQ-Session]], log de implementación
├── specs/auth/
│   └── REQ-Session.md     ← [[ADR-003]], tested_by, verified_in_cycle
├── cycles/
│   └── CYC-2026-08-03.md  ← hub de trazabilidad (linkea todo)
├── incidences/
│   └── INC-001-lag.md     ← [[ADR-003]], afecta [[REQ-Session]]
└── terms/
    └── TERM-JWT.md
```

Ábrelo en [Obsidian](https://obsidian.md) para graph view, backlinks y queries Dataview. Basado en el [spec OKF de Google](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) con changelogs bi-temporales.

## Sistema de verificación

### Verificación funcional (`sddk-verify`)

La **Behavioral Compliance Matrix** mapea cada escenario de spec a un test que pasó en runtime. El análisis estático por sí solo nunca es verificación.

| Lente | Qué verifica |
|-------|-------------|
| Spec Compliance | Cada escenario → test cubridor → PASS en runtime |
| Architecture + Connascence | Calidad de diseño, coupling, SOLID |
| Test Quality | Assertions prohibidas, ratios de mock, triangulación |
| Design Coherence | Decisiones de diseño vs implementación |
| Adversarial Judge A | Detección ciega de deficiencias |
| Adversarial Judge B | Detección ciega de deficiencias |

### Auditoría de deuda técnica (`sddk-debt-verify`)

Hasta 5 agentes cluster ejecutan en paralelo, según el path, en modo read-only.
Generan `debt-report.json` como autoridad machine-readable y
`debt-report.md` como proyección humana; el handoff al CLI actual sigue siendo
solo especificación.

| Cluster | Dimensión |
|---------|-----------|
| Architecture | Connascence, SOLID, críticas Matsumoto + Khononov |
| Smells | 12 Fowler smells con señales grep-verificables → mapeo SOLID |
| Duplication | Estructural/literal/semántica + dead code |
| Coupling | Dependencias ocultas, estado global, imports circulares |
| Over-engineering | YAGNI, ledger de deuda ponytail, trayectoria de bloat |

## Estructura del proyecto

```
sddk-framework/
├── agents/                 # Prompts de agentes; ver docs/generated/inventory.md
├── skills/                 # Skills; ver docs/generated/inventory.md
├── prompts/sddk/            # Phase specs, MCW, git-contract, decision-model, plantillas ADR/roadmap
├── knowledge-template/     # Plantilla de vault (6 tipos de nodo, MOCs, lock de serialización)
├── golden-dataset/         # Casos de meta-verificación (5 casos iniciales + runner)
├── bootstrap.sh            # Instalador para ZCode/OpenCode
├── README.md               # Documentación en inglés
├── README.es.md            # Esta documentación
└── LICENSE                 # MIT
```

## Conceptos clave

- **MCW (Mandatory Complete Workflow)** — la ley. 5 fases, pasos numerados, gates duros. Fuente de verdad: `prompts/sddk/mcw.md`.
- **Lock de serialización** — un ciclo a la vez. Lock file: `milestones/_active.md`. Sobrevive crashes de sesión.
- **Release Completion Guard** — el orchestrator no puede emitir `status: success` sin `HEAD == origin/main` + tag semver confirmado en remoto.
- **Zero docs en repo** — todo el conocimiento del proyecto vive en el vault, nunca en el repo git del proyecto.
- **Changelog bi-temporal** — cada nodo registra `valid_from` / `valid_to`, permitiendo queries de time-travel.

## Compatibilidad

- **Editores**: ZCode, OpenCode (extensible a cualquier runner de agentes que lea prompts markdown)
- **Formato de conocimiento**: [OKF v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md), [Obsidian Properties v1.4+](https://obsidian.md/)
- **MCPs** (opcional): CogniCode (análisis de arquitectura), Chronos (debugging time-travel), Engram (memoria cross-session)

## Contribuir

Las contribuciones son bienvenidas. Por favor lee la arquitectura en `prompts/sddk/mcw.md` antes de proponer cambios.

## Licencia

[MIT](LICENSE) © 2026 Rubentxu
