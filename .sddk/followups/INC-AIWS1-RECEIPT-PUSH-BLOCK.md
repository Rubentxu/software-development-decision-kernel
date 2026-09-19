# INC-AIWS1-RECEIPT-PUSH-BLOCK (2026-09-19)

## Tensión
- `githooks/pre-push` §(B): allowlist cerrado = `docs/*`, `.sddk/followups/*`.
- Contrato SEC: artifacts de ciclo en `tests/cycle-artifacts/p-<id>/<cycle>/`.
- Resultado: el receipt final de AIW-S1 (`5af4faa`, bajo
  `tests/cycle-artifacts/.../aiw-s1-cognicode-real/`) no puede pushearse
  sin un nuevo bump. El release v1.169.91 ya está publicado; el receipt
  queda en local hasta el próximo release legítimo.

## Estado
- v1.169.91 publicada y verificada (tag = HEAD = origin/main = 22e4446).
- Commit local pendiente de push: `5af4faa` (receipt final, docs-only
  en espíritu pero fuera del allowlist por su ruta).

## Opciones de remediación (decisión del operador, ninguna automática)
1. Añadir `tests/cycle-artifacts/*` al allowlist del hook (amplía
   superficie documental del hook; requiere decisión explícita).
2. Registrar el receipt en `docs/` por referencia (documento espejo
   que cite el artifact; duplicación parcial).
3. Mantener la fricción tal cual: el receipt viaja con el próximo bump.
   Coste: ventana en que el receipt público no acompaña al release.
