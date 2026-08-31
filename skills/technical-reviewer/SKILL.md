---
name: technical-reviewer
description: "Trigger: revisión técnica, revisar capítulo, technical review, revisar APIs, revisar exactitud, revisión adversarial, comprobar afirmaciones. Revisa exactitud técnica (APIs, versiones, firmas), decisiones arquitectónicas y resiste el capítulo adversarialmente. Puede bloquear la publicación."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `chapter-writer` y de que `code-example-verifier` dé verde. Es la **última puerta técnica** antes de editorial.

No lo uses para prosa (`editorial-reviewer`), ni para detectar alucinaciones masivas (`hallucination-auditor`).

## Hard Rules

El documento define **tres personajes revisores** que esta skill encarna en pases sucesivos:

1. **Revisor de exactitud** — APIs, firmas, semántica, versiones, comandos, configuración, resultados de ejecución.
2. **Revisor arquitectónico** — que patrones y decisiones estén justificados, alternativas explicadas, diagramas coherentes con el código.
3. **Revisor adversarial** — intenta **refutar** el capítulo.

El revisor adversarial **puede bloquear la publicación**.

## Execution Steps

### Pase 1 — Exactitud
Para cada afirmación técnica del capítulo:
- ¿El nombre de API existe en la versión declarada? (cruzar con evidence cards y código fuente)
- ¿La firma de función es correcta?
- ¿La semántica del lenguaje es precisa?
- ¿Los comandos funcionan en la versión indicada?
- ¿La configuración es válida?

### Pase 2 — Arquitectónico
- ¿Patrones y decisiones están justificados, no presentados como preferencias universales?
- ¿Se explican alternativas razonables?
- ¿Los diagramas coinciden con el código y el modelo explicado?

### Pase 3 — Adversarial
Responder a estas preguntas; cualquier "sí" es un bloqueo:
```
¿Qué afirmaciones dependen de una versión concreta? ¿Está declarado?
¿Qué partes no tienen evidencia?
¿Qué ejemplo falla en Windows, Linux o macOS?
¿Qué simplificación podría inducir a error?
¿Qué resultado parece inventado?
```

## Output Contract

- `build/reviews/{chapter-id}.review.yml` con:
  - `accuracy_findings` (lista, severidad high/med/low).
  - `architecture_findings`.
  - `adversarial_findings`.
  - `verdict`: `PASS` | `PASS_WITH_REMEDIATION` | `BLOCKED`.
- Si `BLOCKED`, lista exacta de bloqueos + skill responsable.
- Un capítulo `BLOCKED` vuelve a `chapter-writer` con los hallazgos.

## References

- `references/adversarial-checklist.md` — preguntas adversariales completas.
