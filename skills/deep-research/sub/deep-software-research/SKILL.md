---
name: deep-software-research
description: "Trigger: investigar tecnología, framework X, lenguaje Y, cómo funciona Z, ¿cuál es la mejor práctica para W?. Investiga CUALQUIER tecnología/framework/lenguaje para producir evidencia rigurosa sobre APIs, versiones, comportamiento, performance, best-practices, patrones arquitectónicos. Núcleo para capítulos técnicos de libros (LIBRO) y para blueprints de aplicaciones (SOFTWARE). Aplica marco Meadows: el ecosistema tecnológico ES un sistema con loops, leverage points, traps."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: domain-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research, software
  consumers: [book-orchestrator, orchestrator]
---

## Activation Contract

Úsalo cuando el tema del capítulo o el componente de software es una **tecnología concreta** (framework, lenguaje, librería, runtime, base de datos). Combina las skills R del pipeline general con conocimiento específico del ecosistema software.

No lo uses para: conceptos abstractos (eso es `deep-domain-modeler`), comparativas de mercado (eso es `deep-knowledge-graph-builder`), historia del software (`deep-historical-lineage-tracer`).

## Hard Rules (extendidas para software)

- **Versión siempre explícita**: ninguna afirmación sobre una API sin `version` específica.
- **Código fuente es L1**: para comportamiento de una API, el código fuente del repo oficial es la fuente primaria.
- **Release notes son L1**: para cambios entre versiones.
- **Documentación oficial es L2**: para "cómo usar" (no para "cómo funciona internamente").
- **Benchmarks requieren reproducibilidad**: cualquier claim de performance debe tener metodología + datos accesibles.
- **Ecosistemas como sistemas**: cuando analizas un framework, identifica el `system-map` (actores, reglas, propósito, loops dominantes). Esto evita caer en Shifting the Burden (cambiar de librería sin resolver el problema real).

## Execution Steps

1. Activar el pipeline R general: R0 (definir el sistema del ecosistema tecnológico) → R1-R6.
2. **R0 específico para software**:
   - **Propósito** del framework/lenguaje (ej: "Bevy es un game engine ECS-first para Rust").
   - **Actores**: maintainers, contributors, sponsors, usuarios objetivo.
   - **Reglas**: convenciones de código, governance, RFC process.
   - **Feedback loops dominantes**: ¿qué incentivos tiene el maintainer? ¿hay network effects? ¿lock-in?
   - **Leverage points**: ¿dónde un cambio pequeño en el framework tendría impacto grande? (documentación, ejemplos, breaking changes).
   - **Traps comunes**: ¿qué errores cometen los usuarios? (e.g., Bevy: usar `Query` con `&mut` dos veces).
3. **R1 específico**: agenda con `claim_type` de software:
   - `api-existence`, `version`, `behavior`, `performance`, `security`, `best-practice`, `architectural-pattern`, `dependency`.
4. **R2 específico**: búsqueda priorizando:
   - Código fuente oficial (GitHub, GitLab).
   - Release notes / changelog.
   - RFCs / design docs.
   - Documentación oficial.
   - Crates.io / npm / PyPI / Maven (para versiones publicadas).
5. **R3-R4** estándar.
6. **R5 específico**: `corpus.yml` con `decay_date` agresivo (tech caduca rápido).
7. **R6 específico**:
   - Modo LIBRO: evidence cards con ejemplos de código.
   - Modo SOFTWARE: blueprints con interfaces + tests reproducibles.

## Anti-patrones de investigación de software

- ❌ **Shifting the Burden a documentación oficial sin verificar código**: la doc puede estar desactualizada. Siempre verificar contra código fuente cuando hay duda.
- � **Policy Resistance entre fuentes**: blogs diciendo cosas contradictorias. Triangular con código fuente.
- � **Drift to Low Performance**: aceptar "esto siempre ha sido así" como verdad. Buscar issues/PRs recientes que cambien el comportamiento.
- ❌ **Tragedy of la Commons**: tutoriales copy-pasteados sin verificar. Buscar fuentes originales.
- ❌ **Escalation entre frameworks**: defender Rust vs. Go con passion. Investigar, no advocacy.
- ❌ **Sucess to the Successful**: recomendar el framework más popular sin evaluar el leverage.

## Salidas específicas

### Modo LIBRO
- `research/evidence-cards/{framework}.yml` con ejemplos de código (vía `include::` desde el workspace, no inline).
- Diagramas Mermaid de arquitectura.
- Tabla de versiones con fechas.

### Modo SOFTWARE
- `research/blueprints/{component}.yml` con interfaz exacta.
- `research/test-fixtures/{api}-expected.json` con valores de referencia.
- `research/code-patterns/{pattern}.{py,rs}` con snippets verificados contra tests.

## References

- Padre: `deep-research-orchestrator` (sub-pipeline `software-research`).
- Skills relacionadas: `deep-pattern-extractor`, `deep-domain-modeler`.
- `references/source-types-software.md` — URLs y patrones de búsqueda por tecnología.
