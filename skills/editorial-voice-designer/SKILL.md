---
name: editorial-voice-designer
description: "Trigger: voz editorial, tono del libro, estilo, libro con humor, cero a experto, para dummies, libro técnico ameno, arquetipo editorial, voice profile, personalidad del libro. Define el contrato de voz editorial del libro: arquetipo (con humor / de cero a experto / para dummies / referencia formal), tono, persona narrativa, humor y ritmo. Es lo que hace que el libro tenga personalidad y no suene a manual genérico."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **inmediatamente después** de `audience-profiler` (A2) y **antes** de `curriculum-designer` (A3). Define el **contrato de voz** que todo el libro respeta: `chapter-writer` lo sigue, `editorial-reviewer` lo valida.

No lo uses para definir el contenido (`curriculum-designer`), ni para corregir muletillas puntuales (`editorial-reviewer`).

## Hard Rules

- El `voice-profile` es **único por libro** y vinculante: `chapter-writer` y `editorial-reviewer` lo cargan.
- El arquetipo se elige **explícitamente** y declara su contract (qué hace y qué NO hace).
- El humor (si lo hay) es **funcional**: ilustra, no distrae. Se calibra al `audience-profile`.
- La persona narrativa, el registro y el léxico son coherentes de principio a fin.

## Arquetipos editoriales

Cada arquetipo es un preset de voice-profile. Se pueden mezclar (ej. "cero a experto con humor").

| Arquetipo | Contract | Humor | Ritmo | Ejemplo real |
|-----------|----------|-------|-------|--------------|
| `con-humor` | Técnico pero ameno; la gracia ayuda a recordar, no a reírse de ti. | funcional, metafórico, nunca cruel ni infantil | medio | "Head First", "Don't Make Me Think" |
| `cero-a-experto` | Empieza asumiendo cero conocimiento, termina dominando. Progresión gradual y visible. | opcional, para aliviar fricción | creciente | "The Rust Programming Language", "You Don't Know JS" |
| `para-dummies` | Simplifica sin mentir; analogías primero, rigor después. Advertencias claras cuando se simplifica. | ligero, tranquilizador | lento al inicio, acelera | "For Dummies", "A Common-Sense Guide" |
| `referencia-formal` | Densa, precisa, sin florituras. El lector busca datos, no entretenimiento. | ninguno | uniforme | K&R "The C Programming Language", ECMA spec |
| `narrativo` | Enseña con una historia/caso de estudio continuo. | integrado en la trama | medio | "The Phoenix Project" |

## Execution Steps

1. Leer `planning/audience-profile.yml` (nivel, tolerancia a teoría, objetivo).
2. Elegir arquetipo(s) con el autor (preguntar si no está claro).
3. Calibrar al lector: ¿este público aguanta humor? ¿qué tipo?
4. Generar `planning/voice-profile.yml` (esquema en `assets/voice-profile.schema.yml`):
   - `archetype`, `tone`, `person`, `humor` (tipo/intensidad/forbidden), `register`, `pacing`.
   - Ejemplos canónicos de "así suena bien" y "así no".
5. Generar `editorial/glossary.yml` inicial (vacío; se llena en redacción).
6. Alimentar a `curriculum-designer` (el ritmo afecta el orden pedagógico) y a `chapter-writer`/`editorial-reviewer`.

## Humor funcional — reglas

- **Ilustra, no distrae**: la metáfora debe aclarar el concepto.
- **Nunca a expensas del lector**: el humor se ríe *con* el lector de la complejidad, no *de* él por no saber.
- **Calibrado al tema**: humor en "qué es un puntero", sí; humor en "cómo no borrar producción", mínimo y con aviso.
- **Marca de zona**: las notas humorísticas se delimitan (`[nota] ... [/nota]` o callout) para que quien quiera lo pase de largo.
- `analogy-auditor` valida que el humor/analogía no induzca error.

## Esquema de voice-profile (resumen)

```yaml
voice:
  archetype: [cero-a-experto, con-humor]
  tone: "cercano, directito, autoconsciente de su propia complejidad"
  person: segunda-del-plural     # veis, comprobáis
  humor:
    style: metaforico            # metaforico|ironico|absurdo|ninguno
    intensity: media             # baja|media|alta
    forbidden: [chistes-a-expensas-del-lector, sarcasmo-hiriente]
  register: informal-estandar    # formal|informal-estandar|coloquial
  pacing: creciente              # lento|uniforme|creciente
  examples_good:
    - "Pensar en el borrow checker como un portero de discoteca muy estricto pero justo."
  examples_bad:
    - "Si no entiendes esto a la primera, mejor dedícate a otra cosa."
```

## Decision Gates

| Necesidad | Acción |
|-----------|--------|
| Público muy novato + humor | `para-dummies` o `cero-a-experto` con humor `baja` |
| Público experto, referencia | `referencia-formal`, humor `ninguno` |
| Tema de seguridad/crítico | Humor `baja` independientemente del arquetipo |
| Autor quiere mezclar | Permitido, pero el contract debe declarar la mezcla |

## Output Contract

- `planning/voice-profile.yml`.
- `editorial/glossary.yml` inicial.
- `chapter-writer` y `editorial-reviewer` cargan el voice-profile; el workflow lo requiere antes de redactar.

## References

- `references/archetype-recipes.md` — presets detallados por arquetipo con ejemplos.
- `assets/voice-profile.schema.yml` — esquema validable.
