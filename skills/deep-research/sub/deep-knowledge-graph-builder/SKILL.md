---
name: deep-knowledge-graph-builder
description: "Trigger: knowledge graph del libro, mapa de conceptos, relaciones entre conceptos, autores influyentes, papers seminales, grafo de conocimiento. Construye grafos de conocimiento para temas complejos: entidades (conceptos, autores, papers, librerías), relaciones (citó_a, influyó_en, depende_de, contradice_a), propiedades. Produce RDF/Turtle, JSON-LD, o grafos Mermaid. Permite visualizar y razonar sobre la estructura del campo."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: domain-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
  consumers: [book-orchestrator, orchestrator]
---

## Activation Contract

Úsalo cuando el tema requiere **visualizar/razonar sobre relaciones** entre muchas entidades: autores que se citan, papers seminales, conceptos que se influencian, librerías que dependen de otras, etc. Produce grafos navegables y consultables.

No lo uses para: modelar un solo dominio con pocas entidades (`deep-domain-modeler`), trazar la evolución temporal (`deep-historical-lineage-tracer`).

## Hard Rules

- **Cada nodo y arista tiene fuente verificable**: un paper seminal (L1), doc oficial (L2), o agregación de evidencia (L3).
- **Tipos de relación explícitos**: vocabularios controlados (`cito:cites`, `dct:creator`, `skos:broader`, etc.).
- **Cardinalidad cuando aplique**: 1 autor → N papers; 1 paper → N concepts.
- **Versionado**: el grafo evoluciona; cada versión se numera.
- **Consultable**: el grafo debe responder a preguntas como "¿quién influyó en X?" o "¿qué papers citan a Y?".

## Execution Steps

1. Activar pipeline R para el dominio.
2. Identificar los **tipos de entidades** relevantes:
   - Personas (autores, mantenedores, reviewers).
   - Documentos (papers, libros, RFCs, issues).
   - Conceptos (algoritmos, patrones, teorías).
   - Cosas concretas (frameworks, librerías, datasets).
3. Identificar los **tipos de relaciones** entre ellas:
   - `author_of`, `cites`, `cites_as_foundation`, `influences`, `contradicts`, `extends`, `depends_on`, `replaces`.
4. Construir el grafo:
   - **Modo LIBRO**: Mermaid graph (visualización) + JSON-LD (intercambio).
   - **Modo SOFTWARE**: RDF/Turtle (razonamiento) + JSON-LD (consumo por API).
5. Anotar cada nodo y arista con sus fuentes.
6. Validar: el grafo debe ser navegable (no disconnected components sin razón).

## Vocabularios comunes

| Tipo | Vocabulario | URL |
|------|-------------|-----|
| Citación | CITO | `http://purl.org/spar/cito/` |
| Persona | FOAF | `http://xmlns.com/foaf/0.1/` |
| Documento | BIBO, DC, DCTERMS | `http://purl.org/ontology/bibo/`, `http://purl.org/dc/terms/` |
| Concepto | SKOS | `http://www.w3.org/2004/02/skos/core#` |
| Software | DOAP, SCHEMAORG SoftwareSourceCode | `http://usefulinc.com/ns/doap#` |

## Ejemplo (Turtle / RDF)

```turtle
@prefix cito: <http://purl.org/spar/cito/> .
@prefix dct: <http://purl.org/dc/terms/> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .

<https://donellameadows.org/archives/leverage-points-places-to-intervene-in-a-system/> a cito:AcademicArticle ;
    dct:title "Leverage Points: Places to Intervene in a System" ;
    dct:creator <https://example.org/foaf/donella-meadows> ;
    cito:cites <https://mitpress.mit.edu/9780262001436/world-dynamics/> .

<https://example.org/foaf/donella-meadows> a foaf:Person ;
    foaf:name "Donella H. Meadows" ;
    foaf:workplaceHomepage <https://www.academyforchange.org/> .
```

## Output según modo

### Modo LIBRO
- `research/knowledge-graphs/{topic}.jsonld` (intercambio).
- `research/diagrams/{topic}-graph.mmd` (visualización Mermaid).
- `research/drafts/{topic}-graph-section.md` (borrador explicando el grafo).

### Modo SOFTWARE
- `research/knowledge-graphs/{topic}.ttl` (RDF/Turtle).
- `research/blueprints/kg-query-api.yml` (API para consultar el grafo).

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Nodo sin fuente | STOP: investigar origen (R2) |
| Relación sin evidencia | NO incluir; o marcar `confidence_score: low` |
| Grafo con > 1000 nodos sin clustering | Dividir por sub-tema; o usar vistas |
| Grafo disconnected components | Investigar por qué; o son dominios separados |

## Output Contract

- `research/knowledge-graphs/{topic}.{ttl,jsonld}`.
- `research/diagrams/{topic}-graph.mmd` (LIBRO).
- `research/blueprints/kg-query-api.yml` (SOFTWARE).

## References

- Vocabularios: ver tabla arriba.
- Herramientas:
  - `rdflib` (Python): para construir/consumir RDF.
  - `graphviz` + `dot`: para visualizar.
  - Mermaid `graph`/`flowchart`: para grafos simples en markdown.
- `references/rdf-vocabularies.md` — vocabularios recomendados por dominio.
