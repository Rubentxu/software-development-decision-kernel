# Knowledge Merkle Tree — diseño definitivo

## Árbol de identidad/freshness

```text
Repository
  Context/Area
    Package/Crate
      Module/Directory
        File
          Symbol (opt-in)
```

Un nodo contiene **fingerprint vector**, no sólo hash de bytes:

```text
source_hash
structure_hash
symbols_hash
dependency_hash
behavior_hash?
knowledge_hash
intent_hash?
decision_refs_hash
analyzer_set_hash
```

## Merkle composition

`node_digest = H(unit_identity, local_fingerprint_vector, ordered_child_digests, schema_version)`.

Los roots por dimensión pueden calcularse por separado para evitar que un comentario invalide el mismo conocimiento que un import.

## Semantic Impact Overlay

El KMT no intenta ser grafo de dependencias. El SemanticGraph mantiene edges transversales:

```text
FILE_DEPENDS_ON_FILE
MODULE_DEPENDS_ON_MODULE
ASSERTION_DERIVED_FROM
ASSERTION_DEPENDS_ON
DECISION_ASSUMES
TRADEOFF_REVISIT_WHEN
RUNTIME_OBSERVED_EDGE
```

## Invalidation algorithm

1. comparar fingerprint vector old/new;
2. invalidar assertions que declaran dependencia de las dimensiones cambiadas;
3. invalidar parents jerárquicos como `INVALIDATED`;
4. propagar por overlay como `POSSIBLY_STALE`, nunca como falso definitivo;
5. priorizar reevaluación por risk/impact/uncertainty;
6. confirmar o reemplazar assertions;
7. recalcular roots.

## Ejemplo

Cambio de comentario:

```text
source_hash      changed
structure_hash   same
dependency_hash  same
```

Puede invalidar resumen textual, pero no obliga a reevaluar Hexagonal.

Nuevo import:

```text
source_hash      changed
structure_hash   changed
dependency_hash  changed
```

Invalida dependency assertions y vuelve `POSSIBLY_STALE` Alignment assessments de Hexagonal/DIP/Connascence relacionados.

## Persistencia

- nodes/current state: projection reconstruible;
- snapshot de un `ProjectKnowledgeBaseline`: objeto CAS opcional + receipt;
- raw provider caches: responsabilidad del provider, no de SDDK.
