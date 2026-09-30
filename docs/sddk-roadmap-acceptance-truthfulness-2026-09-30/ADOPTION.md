# Adopción en el roadmap actual

## Cambios mínimos

### `docs/roadmap/ROADMAP.md`

Insertar después de C3k/C3j, sin renumerar los hitos existentes:

```text
C3l — Acceptance Truthfulness & False-Green Elimination
C3m — Semantic & Boundary Convergence
C3n — Production Boundary Certification
```

Añadir la regla:

> C3j puede continuar en paralelo. C4/C6 sólo pueden reclamar como certificadas las capacidades afectadas por C3l/C3m después de C3n.

### `docs/roadmap/CURRENT.md`

Mantener el siguiente paso C3j, pero añadir un segundo frente activo:

```text
parallel_required:
  - C3j
  - C3l.0
```

Recomendación operativa inmediata:

```text
1. abrir C3l.0
2. re-clasificar sólo claims afectados
3. ejecutar C3l.1 y C3l.2 primero
4. continuar C3j sin mezclar concerns
```

### `docs/roadmap/STATE.yaml`

Añadir:

```yaml
acceptance_truthfulness:
  baseline_sha: 6fbe1990ac40826a1f52ddf5b52c145fbea07f13
  status: PROPOSED
  active_milestone: C3l
  active_slice: C3l.0
  blocks:
    - AIW_FULL_CERTIFICATION
    - CONTEXT_FIRST_FULL_CERTIFICATION
    - ENHANCED_RUNTIME_CERTIFICATION
  does_not_block:
    - C3j
    - BASE_PROFILE_WHEN_CAPABILITY_NOT_CLAIMED
```

### `docs/roadmap/UAT-MATRIX.md`

Añadir las filas `AT-UAT-001..026` del fichero adjunto.

## Primer ciclo ejecutable

**ID:** `c3l-0-acceptance-baseline`

### Scope

- sólo documentación/estado;
- no tocar código;
- re-clasificar los claims demostrablemente sobrecertificados;
- registrar evidencia del audit baseline.

### Salida

- matrix AIW-S0..S8;
- matrix R0..R11;
- boundary class por UAT;
- lista exacta de slices reabiertas.

Después:

```text
C3l.1 DebVerify
C3l.2 Producer→L0
```

pueden desarrollarse en paralelo con commits atómicos separados.
