# rust-systems-reasoning

Skill portable basada en el estándar **Agent Skills** para diseñar y revisar software de sistemas en Rust con énfasis en:

- invariantes expresadas mediante tipos;
- núcleo pequeño y trazable;
- separación core/adapters;
- async y concurrencia colocados deliberadamente;
- zero-copy y parsing seguros;
- `unsafe` con fronteras auditables;
- presupuestos de rendimiento;
- property testing, fuzzing y verificación formal.

## Instalación en OpenCode

### Global

```bash
mkdir -p ~/.config/opencode/skills
cp -R rust-systems-reasoning ~/.config/opencode/skills/
```

### Sólo para un proyecto

```bash
mkdir -p .opencode/skills
cp -R rust-systems-reasoning .opencode/skills/
```

OpenCode descubrirá `SKILL.md` y cargará el cuerpo únicamente cuando la descripción coincida con la tarea.

## Ejemplos de prompts que deberían activarla

- "Revisa esta arquitectura Rust y dime dónde debería estar async."
- "Diseña un parser binario zero-copy seguro."
- "¿Cómo hago que estos estados inválidos no se puedan representar?"
- "Revisa este unsafe y sus invariantes de memoria."
- "Diseña el core de este motor separándolo de Tokio y del filesystem."
- "Propón estrategia de fuzzing/Kani para este parser."
- "Analiza copias, allocations y locks de este hot path."

## Diseño de la skill

`SKILL.md` contiene sólo el flujo que debe ejecutarse siempre. Las reglas especializadas viven en `references/` y se cargan únicamente cuando la rama de trabajo las necesita.

Esto mantiene baja la carga de contexto y hace más predecible el proceso del agente.
