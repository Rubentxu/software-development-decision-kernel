# Ejemplo — Workflow `content.book.create`

## Objetivo

Falsar que el core sea software-specific.

```yaml
apiVersion: sddk.dev/v1alpha1
kind: Workflow
metadata:
  name: content.book.create
  version: 0.1.0

spec:
  augmentation_profiles:
    - core.context
    - core.evidence
    - research.sources
    - content.consistency

  inputs:
    topic:
      type: string
    audience:
      type: AudienceProfile
    constraints:
      type: ContentConstraints

  steps:
    - id: brief
      use: content.book-brief@v1

    - id: research
      use: core.research@v1
      needs: [brief]
      with:
        question_set: $steps.brief.research_questions

    - id: outline
      use: content.outline@v1
      needs: [research]

    - id: write_chapters
      use: content.write-section@v1
      needs: [outline, research]
      map:
        over: $steps.outline.sections
        max_concurrency: 3

    - id: consistency
      use: content.consistency-review@v1
      needs: [write_chapters]

    - id: revise
      use: content.revise@v1
      needs: [consistency]
      loop:
        max_iterations: 3
        until: $.outputs.accepted == true

    - id: publish
      use: content.publish@v1
      needs: [revise]

  outputs:
    manuscript: $steps.publish.artifact
```

## Cómo usa el mismo kernel

| Necesidad de authoring | Primitive SDDK |
|---|---|
| investigación y fuentes | EvidenceRef + provider Observation |
| decisiones de tono/estructura | DecisionRecord |
| capítulos paralelos | Map + StepRun children |
| memoria entre capítulos | ContextCapsule + expandable refs |
| consistency findings | Observation/Evidence |
| revisión iterativa | Loop + convergence budget |
| restart | durable WorkflowRun/StepRun |
| handoff | hypermedia resource/affordances |
| publicación | Artifact + governed capability |

## `content.write-section` reusable

Puede aparecer también en:

```text
manual técnico
curso
whitepaper
informe largo
documentación de producto
```

porque consume contracts:

```text
SectionBrief
ResearchBundleRef
StyleConstraints
ContextCapsuleRef
```

no `book/chapter-3.md` como interfaz primaria.

## Falsadores de generalidad

C7 falla si para ejecutar este workflow hay que:

- crear un segundo runtime;
- añadir Git como dependencia core obligatoria;
- crear un Evidence store nuevo;
- codificar `chapter` dentro de WorkflowRuntime;
- cambiar ContextCapsule a un modelo específico de authoring;
- duplicar Authority/Gateway.
