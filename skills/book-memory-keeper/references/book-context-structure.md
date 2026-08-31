# Estructura estándar de `book-context/` (tech-agnostic)

Cada proyecto de libro tiene un directorio `book-context/` que es su **memoria viva**. No depende de la tecnología del libro (Rust, Python, JS...). Lo mantiene `book-memory-keeper`.

```
book-context/
├── LEDGER.md                  # estado del pipeline (la "pantalla de control")
├── SESSION-LOG.md             # último resumen ejecutivo + "qué hacer mañana"
├── VOICE.md                   # voice-profile + glosario de estilo (vivo)
├── GLOSSARY.md                # glosario canónico de términos (vivo)
├── DECISIONS.md               # índice de ADRs
├── adr/
│   ├── 01-stack-asciidoc.md
│   ├── 02-arquetipo-cero-a-experto-humor.md
│   └── NN-{slug}.md
├── SNAPSHOT-CORPUS.md         # resumen legible del corpus (de research-knowledge-curator)
└── CONVENTIONS.md             # convenciones del libro concreto (naming, formato)
```

## Contenido de cada documento

### LEDGER.md — estado del pipeline
```yaml
book: "Título del libro"
stack: "Rust/Bevy 0.19"          # o Python, Go, JS... (tech-agnostic)
voice_archetype: [cero-a-experto, con-humor]
current_macro_phase: C
last_session: 2026-07-23
chapters:
  - id: cap-04
    state: DONE
  - id: cap-12
    state: BLOCKED
    blocked_on: hallucination-auditor
    remediation_target: chapter-writer
    cycle: 2
next_action: "Re-redactar cap-12 con sintaxis BSN verificada del corpus"
```

### SESSION-LOG.md — contexto ejecutivo
```markdown
# Sesión 2026-07-23

## Qué hicimos
- Completamos Macro-fase R para el tema "BSN".
- Descubrimos que la auditoría original estaba equivocada: bsn! sí existe.

## Dónde quedamos
- cap-12 reescrito, pendiente re-verificar código.

## Qué hacer mañana
1. Ejecutar code-example-verifier sobre chapters/chapter-12-scenes.
2. Empezar Macro-fase R para "scheduling".
```

### VOICE.md — voz editorial viva
Copia del `voice-profile.yml` más anotaciones que surgen al redactar (ej. "decidimos evitar la palabra 'simplemente'").

### GLOSSARY.md — glosario canónico
Término → traducción/decisión. Vivo: cada término nuevo al redactar se añade.

### adr/ — decisiones de diseño
Por qué elegimos este stack, este arquetipo, esta simplificación pedagógica. Plantilla en `adr-template.md`.

## Principio
Estos documentos son la **fuente de verdad legible**. Engram es el índice semántico que los recupera. Nunca hay decisión importante que no aterrice aquí.
