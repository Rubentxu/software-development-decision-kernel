---
name: editorial-reviewer
description: "Trigger: revisión editorial, estilo, consistencia terminológica, glosario, muletillas LLM, Vale, lint editorial, voz narrativa, castellano de España. Mantiene terminología, voz, estilo, longitud y estructura consistentes usando reglas automáticas (Vale) sobre AsciiDoc/Markdown."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `technical-reviewer` y `pedagogical-reviewer`, como última pasada de estilo antes de `book-builder`.

No la uses para corrección técnica (`technical-reviewer`) ni didáctica (`pedagogical-reviewer`).

## Hard Rules

- **Carga `planning/voice-profile.yml`** (de `editorial-voice-designer`); es vinculante para la voz y el humor.
- Carga `book-context/GLOSSARY.md` (glosario vivo del libro); ningún sinónimo no autorizado.
- Voz narrativa, tono, registro y humor **deben coincidir con el voice-profile** del libro.
- Castellano por defecto (variante según `voice-profile.language_variant`).
- Uso **consistente** de términos ingleses (mismas reglas en todo el libro).
- Prohibición de **muletillas LLM** (lista cerrada en `references/llm-tells.md`).
- Si el voice-profile declara `humor.intensity: alta`, el review es más laxo con humor; si es `baja` o `ninguno`, mucho más estricto.
- Las reglas se automatizan con **Vale** (config en `assets/vale-config.ini`).

## Execution Steps

1. Cargar `editorial/glossary.yml` (canónico) y `editorial-style.yml`.
2. Ejecutar `vale src/` con la configuración del libro.
3. Revisar hallazgos y categorizar:
   - `terminology` — uso de término no canónico.
   - `voice` — cambio de persona narrativa.
   - `llm_tell` — muletilla detectada.
   - `length` — sección fuera de rango.
   - `format` — advertencias/ejemplos mal formateados.
4. Corregir lo automático; escalar lo ambiguo al autor.
5. Emitir `build/reviews/{chapter-id}.editorial.yml`.

## Config Vale (resumen)

```ini
StylesPath = styles
MinAlertLevel = suggestion

[*.adoc]
BasedOnStyles = LibroTecnico
LibroTecnico.TerminosCanonicos = YES
LibroTecnico.FrasesVacias = YES
LibroTecnico.SegundaPersona = YES
```

## Output Contract

- `build/reviews/{chapter-id}.editorial.yml` con hallazgos categorizados.
- `verdict`: `PASS` | `PASS_WITH_REMEDIATION` (las muletillas y términos no canónicos son remediación obligatoria).
- Resumen de consistencia global (términos que fluctúan entre capítulos).

## References

- `references/llm-tells.md` — lista cerrada de muletillas LLM.
- `assets/vale-config.ini` — configuración Vale lista.
