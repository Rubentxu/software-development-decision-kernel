# Progressive disclosure

El agente no debe abrir código por defecto.

```text
L0 Project posture
L1 Bounded context / architectural area
L2 Package/module summary
L3 File Knowledge Card
L4 Symbol/graph slice
L5 AST/source excerpt
L6 Full source
L7 Runtime evidence/trace slice
```

Cada escalado debe estar motivado por `Unknown`, contradiction, risk, stale knowledge o insuficiente evidence coverage.

El ContextCompiler mantiene budgets por tokens/items, no copia todos los workbooks al prompt.
