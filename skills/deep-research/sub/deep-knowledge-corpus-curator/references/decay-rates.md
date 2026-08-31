# Decaimiento de evidencia — Por dominio

La evidencia tecnológica y científica caduca. Cada claim tiene `decay_date` que indica cuándo debe re-verificarse. Esta tabla guía el cálculo.

## Tasas por dominio

| Dominio | Tasa de decaimiento | Notas |
|---------|---------------------|-------|
| **Frontend web frameworks** | 6-12 meses | UI patterns y best-practices cambian rápido |
| **Backend frameworks** | 1-2 años | APIs y patrones más estables |
| **Lenguajes de programación** | 2-3 años | Features nuevos caducan; sintaxis básica no |
| **Librerías específicas** | 1-2 años | APIs cambian; verificar versión |
| **AI/ML state-of-the-art** | 6-12 meses | SOTA cambia muy rápido |
| **AI/ML foundational** | 2-5 años | Backprop, transformers, etc. |
| **Cloud / DevOps** | 1-2 años | Servicios cambian; verificar version |
| **Systems Thinking foundational** | No caduca | Meadows/Forrester/Senge no caducan |
| **Systems Thinking re-tests** | 2-5 años | World3 re-tests cada pocos años |
| **Ciencia (peer-reviewed)** | 3-5 años | Salvo refutación |
| **Medicina (guidelines)** | 1-3 años | Guidelines se actualizan |
| **Medicina (clinical trials)** | 2-5 años | Salvo nueva evidencia |
| **Economía (datos)** | 1-2 años | Datos cambian |
| **Economía (análisis)** | 3-5 años | Análisis más estable |
| **Historia** | No caduca | Contexto estable (con revisión historiográfica) |
| **Filosofía** | No caduca | Conceptos perennes |

## Cómo calcular decay_date

```python
# Pseudo-código
def compute_decay_date(claim, current_date):
    domain = claim.domain
    claim_type = claim.claim_type
    decay_rate = DOMAIN_DECAY[domain][claim_type]
    if decay_rate is None:  # no caduca
        return None
    return current_date + decay_rate
```

## Política de re-check

- `decay_date` vencida → la claim pasa a `status: needs_recheck`.
- `version-drift-detector` dispara re-check cuando hay nueva versión.
- Un capítulo no puede citar claims `needs_recheck` sin re-verificar.
- Las foundational (Meadows, Forrester, Senge, Kim) NO tienen `decay_date`.

## Re-check workflow

1. Curator detecta `needs_recheck`.
2. Dispara R-incremental solo para esa claim (R2-R4 focal).
3. Re-verifica fuentes (HEAD request, búsqueda de papers posteriores).
4. Si sigue válida → actualiza `confidence_score` y `decay_date`.
5. Si refutada → marca `deprecated` con link al paper de refutación.
6. Si obsoleta pero no refutada → actualiza `text` con la versión actual.

## Anti-patrones

- ❌ Asignar `decay_date` a foundational (Meadows). NO caducan.
- ❌ No asignar `decay_date` a tech. SIEMPRE caducan.
- ❌ Citar claim `needs_recheck` sin re-verificar.
- ❌ Reemplazar fuente sin actualizar `confidence_score`.
