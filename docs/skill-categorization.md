# Skill Categorization — Taxonomía lógica

## Problema

El framework SDDK distribuye skills en `skills/<name>/` (un nivel). Cuando un dominio crece a 20+ skills relacionadas, el listado plano se vuelve difícil de navegar. 22 directorios `deep-*` en la raíz de `skills/` generaban demasiado ruido.

## Solución adoptada: Master + sub-skills (jerárquica)

Para bundles grandes (≥ 15 skills relacionadas), adoptamos el patrón **"hierarchical skills"** de LangChain + RFC-318 (collection-based namespacing):

```
skills/
└── deep-research/                ← MAESTRA (1 entry point)
    ├── SKILL.md                  ← índice + activación
    ├── references/               ← docs compartidos
    │   ├── index.md
    │   └── pipeline-r0-r6.md
    └── sub/                       ← 22 sub-skills especializadas
        ├── deep-research-orchestrator/   (gate)
        ├── deep-research-strategist/      (R1)
        ├── deep-source-discovery-specialist/  (R2)
        └── ...
```

**Ventajas**:
- ✅ Una sola entrada raíz (la maestra) en lugar de 22.
- ✅ Granularidad preservada (cada sub-skill es invocable independientemente).
- ✅ Compatible con la spec oficial de Agent Skills (1 skill = 1 directorio + SKILL.md).
- ✅ Las sub-skills siguen siendo directorios independientes (progressive disclosure funciona).
- ✅ Las sub-skills pueden referenciar la maestra (`category: deep-research`).

**Limitación reconocida con el CLI actual** (SDDK 1.13.0):
- El CLI no escanea subdirectorios en `skills/` (asume 1 nivel).
- Hasta SDDK2-411 (modificar CLI), la maestra existe pero el orchestrator debe leer `references/index.md` manualmente para descubrir las sub.
- Las sub-skills no se cargarán automáticamente — pero esta es la dirección correcta para sddk-2.0.

## Taxonomía actual (categoría: `deep-research`)

| Subcategoría | Sub-skills |
|--------------|-----------|
| **gate** | `sub/deep-research-orchestrator` |
| **methodology-hub** | `sub/deep-research-methodology-hub` |
| **r-pipeline** | 7 sub-skills (R1-R6) |
| **domain-pipeline** | 6 sub-skills |
| **systems-thinking** | 7 sub-skills (incluyendo la maestra coach) |

Ver `skills/deep-research/references/index.md` para el mapa completo.

## Estado

- **Versión**: 1.1 (cambio de patrón estructural)
- **Fecha**: 2026-08-16
- **Aplica a**: bundle `deep-research` (22 skills)
- **Cambios requeridos en CLI**: SDDK2-411 (sddk-2.0) para descubrimiento automático de sub-skills
