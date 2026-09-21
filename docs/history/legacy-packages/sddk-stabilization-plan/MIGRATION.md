# Migración de SDDK v3.5 a v3.6

## Principio

No reescribir todos los agentes a la vez. Introducir una envolvente determinista alrededor del comportamiento existente y mover reglas una a una.

## Fase 0 — Congelación semántica

Antes de implementar Rust:

1. Inventariar agentes, skills, comandos, artefactos y paths.
2. Marcar cada regla con un identificador estable.
3. Resolver contradicciones de adopción, ramas, debt y release.
4. Eliminar referencias a plugins inexistentes o implementarlos.
5. Crear fixtures de comportamiento actual aceptado.

## Fase 1 — Linter y contratos

Añadir:

- `workflow.yaml`.
- Schemas de agente y capability.
- `sddk lint`.
- Generación de documentación.

Los agentes siguen funcionando, pero CI detecta inconsistencias.

## Fase 2 — Identidad y adopción

Sustituir toda lógica de rutas y adopción por:

```bash
sddk project resolve
sddk adopt plan
sddk plan apply
sddk adopt repair
```

Los prompts solo invocan comandos y explican sus resultados.

## Fase 3 — Ledger y ciclos

El CLI comienza a registrar ciclos y fases. Los agentes existentes se ejecutan mediante un adaptador de compatibilidad:

```text
legacy agent output
    → adapter
    → structured agent result
    → schema validation
```

## Fase 4 — Gateway local

Migrar primero:

- Filesystem.
- Git local.
- Tests.
- Artefactos.

Eliminar comandos Bash autoritativos de los agentes correspondientes.

## Fase 5 — Forge y release

Crear adaptador GitHub y sustituir la secuencia actual de release.

## Fase 6 — Vault e informes

Indexar el vault, validar relaciones y generar informes autocontenidos.

## Compatibilidad temporal

Cada agente tendrá un campo de migración:

```yaml
execution_mode: legacy | hybrid | deterministic
```

- `legacy`: documentado, no bloquea inicialmente.
- `hybrid`: resultado estructurado, pero algunas capacidades siguen fuera del gateway.
- `deterministic`: todas las operaciones pasan por el CLI.

CI debe impedir que un agente vuelva de `deterministic` a `hybrid` o `legacy`.

## Eliminación de compatibilidad

El modo legacy se elimina cuando:

- Todos los agentes críticos son deterministas.
- No quedan comandos shell autoritativos.
- Los fixtures cubren todos los hard gates.

## Estado: migración completada

Las fases 0-6 se entregaron entre `v0.1.0` y `v0.10.0`:

- Fase 0-1: canon, linter, inventario y CI (`v0.2.0`).
- Fase 2: identidad y adopción (`v0.2.0`-`v0.3.0`).
- Fase 3: ledger, ciclos, leases, rebuild y gates autorizados (`v0.3.0`-`v0.10.0`).
- Fase 4: gateway de capacidades, Git local y CAS (`v0.4.0`-`v0.5.0`).
- Fase 5: Forge y release reconciliable (`v0.7.0`).
- Fase 6: vault, índices y distribución (`v0.8.0`-`v0.9.0`).

El runtime Rust es la autoridad operativa; los prompts legacy quedan como contratos documentados.
