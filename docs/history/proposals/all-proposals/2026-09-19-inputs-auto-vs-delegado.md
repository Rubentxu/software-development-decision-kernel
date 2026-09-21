# Arquitectura de producción de inputs: auto-ejecutables vs delegados a CLI

**Estado:** propuesta. Extiende [`2026-09-19-inputs-origen-y-adopcion-auditoria.md`](2026-09-19-inputs-origen-y-adopcion-auditoria.md).
**Fecha:** 2026-09-19. Restricción arquitectónica que lo gobierna: **AIS-004 de
arch-spec-030** — «A permanently running SDDK daemon SHALL NOT be required». Toda la
propuesta respeta esa ley: nada aquí exige daemon.

## 1. El criterio de reparto

La pregunta correcta no es «¿proceso residente o LLM?». Es **quién debe ejecutar
cada captura** según tres propiedades del input:

| Propiedad | Decide |
|---|---|
| **Determinismo** — ¿dos ejecuciones sobre el mismo estado dan el mismo resultado? | Si sí, no debe haber LLM en el camino de captura. |
| **Momento** — ¿el valor caduca si no se captura en el instante del evento? | Si sí, necesita un disparador automático (hook/ciclo), no una petición del agente. |
| **Interpretación** — ¿requiere juzgar (elegir, clasificar, decidir relevancia)? | Si sí, es legítimo delegarlo al LLM — pero su salida debe volverse tipada. |

Y una ley de cerradura: **todo input que persista en el ledger debe entrar por un
comando SDDK** (`plan evidence attach`, gate receipt, cycle transition, knowledge
import). El LLM nunca escribe evidencia directamente: invoca el comando que la
escribe. Eso da auditoría, schema y fail-closed gratis.

## 2. Lo que SDDK debería auto-ejecutar (procesos internos, sin daemon)

Cuatro mecanismos, todos acoplados a eventos o a invocaciones existentes:

### 2.1 Captura atada a gates de ciclo (ya existe al 80%)

Cuando `apply` evalúa un gate ya construye `{evidence_json}` con path/SHA-256/
diff digest. El salto: **que cada comando determinista que un agente ejecuta para
un gate deje el receipt automáticamente**, no que el LLM lo redacte después.

- **Coste bajo:** los gate receipts ya tienen schema e idempotencia.
- **Nueva pieza:** un wrapper interno que `apply.md` ordene usar
  (`sddk dev run-and-record -- <cmd>`) que ejecute el comando determinista, capture
  exit code + digest de salida + revisión, y emita el receipt en un paso.
- **UAT:** el mismo comando sobre el mismo estado produce el mismo receipt id; un
  exit≠0 nunca produce receipt PASS.

### 2.2 Hooks de git como productores de eventos (existe el patrón, falta uso)

El repo ya usa `githooks/pre-push` para disciplina de release. El mismo patrón
puede capturar inputs que **caducan en el momento**:

- `post-commit` / `pre-push`: registrar SHA, diff digest, dirty-state → como
  observación `observed_for` del WorkItem activo (Puente 2 del doc hermano).
- **Coste:** bajo. El productor de diff ya existe (`git_changed_paths`); solo falta
  que un hook lo invoque y persista.
- **Riesgo nombrado:** los hooks son opt-in por checkout (`core.hooksPath`); el
  UAT debe cubrir hook ausente = degradación silenciosa a captura bajo demanda,
  nunca a datos falsos.

### 2.3 Projector incremental de proyecciones (existe, falta puesta en marcha)

`rebuild.rs` + `projection_checkpoints_v1` ya implementan reconstrucción canónica
con checkpoints. Hoy nadie la dispara en el ciclo. Debería correr:

- **en cada transición de fase** (el `sddk cycle transition` ya es el punto de
  autoridad; añadir la reconstrucción de proyecciones afectadas como efecto
  secundario determinista), y
- **al inicio de cada consulta** (`plan roadmap *` verifica checkpoint freshness
  contra `list_events` sequence antes de responder).

Esto convierte el «snapshot quizás reconciliado» del reporte previo en un
invariante: **la proyección es siempre reconstruible y siempre fresca al leerla**,
sin daemon: se reconstruye cuando cambia el ledger que la alimenta.

### 2.4 Knowledge scan como fase de descubrimiento (existe, 0 adopción)

`sddk knowledge scan` detecta y clasifica conocimiento no registrado con hashes.
Es el único productor de «qué sabemos del repo» que SDDK tiene y nadie lo pide.
Debería ser un paso estándar de `sddk-explore` (la fase ya lee código; scan
formaliza lo que el LLM encontrará luego con grep).

## 3. Lo que se delega a CLI especializadas vía LLM (shell por el agente)

El LLM no captura: **invoca herramientas especializadas y reporta sus salidas
tipadas**. La división por tipos de input:

| Input | Herramienta CLI especializada | Por qué delegado |
|---|---|---|
| Resultado de tests | `nextest json` / `pytest --json-report` / `go test -json` | Los runners ya emiten JSON por testcase; SDDK no debe reimplementar parsers por familia. El agente ejecuta con `sddk dev run-and-record`, y el wrapper persiste el JSON crudo a CAS + referencia. La normalización por testcase es el Puente 2, y puede ser incremental: familia cargo primero. |
| Análisis estático | Proveedor CogniCode vía `CodeIntelligencePort` (CC-S1+) | El contrato del puerto ya define negotiate/analyze/cancel; el agente no debe intermediar (M7 del spike: sin LLM entre SDDK y el puerto). La delegación aquí es del *runtime*, no del agente. |
| Análisis semántico/profundidad de código | Agentes LLM con herramientas (ripgrep, LSP) | Interpretar «por qué está acoplado» es genuinamente generativo. Pero el informe debe ceñirse a un envelope (ya existe `AgentContributionEnvelope`) y el agente adjunta evidencia con refs, no citas sueltas. |
| Cobertura, benchmarks, seguridad | Herramientas nativas (`cargo llvm-cov`, `hyperfine`, `cargo-audit`) | Mismo patrón que tests: ejecutar + persistir salida cruda + normalizar después por demanda. |
| Estado del repo (dirty, ahead/behind) | `sddk dev doctor` / hooks | Determinista → auto, no LLM. |

La diferencia con el estado actual: hoy el LLM ejecuta `cargo test` y **narra** el
resultado. Con el wrapper, ejecuta y **queda registrado**; la narración pasa a ser
presentación de un receipt verificable, no la fuente.

## 4. Mapa final de responsabilidad

```text
AUTO (disparado por eventos/ciclo, código interno):
  transición de fase ──→ rebuild proyecciones ──→ checkpoints frescos
  gate evaluation ──→ receipt automático con digest de comando
  hooks git (opt-in) ──→ observación de cambio con revisión
  explore phase ──→ knowledge scan formal del repo

DELEGADO (LLM ejecuta herramienta CLI especializada, reporta tipado):
  test/coverage/bench/audit ──→ runners JSON nativos ──→ CAS + normalize
  análisis estático ──→ CodeIntelligencePort (runtime, sin LLM en medio)
  interpretación semántica ──→ agentes con envelope + evidencia con refs

PROHIBIDO:
  LLM redactando evidencia de un hecho determinista
  LLM escribiendo al ledger fuera de los comandos canónicos
  capturas auto que sobrescriban evidencia adjuntada por humano
```

## 5. Orden de adención (misma disciplina: no roadmap nuevo)

1. **`sddk dev run-and-record`** — el wrapper del 2.1. Es la pieza de menor coste y
   mayor cobertura: convierte en inputs tipados todos los comandos que el agente
   ya ejecuta. Un UAT pequeño.
2. **Reconstrucción de proyecciones en transición + freshness en lectura** (2.3).
   Conecta el backlog de proyecciones muertas con el ciclo vivo.
3. **Hook post-commit → observación de cambio** (2.2), tras el Puente 2 (que a su
   vez empieza por exponer test_runner o normalizar nextest JSON).
4. **`knowledge scan` en explore** (2.4) — barato, solo prompt + skill.
5. **CC-S1** (puerto real) sigue siendo la línea A6; este documento no lo reemplaza,
   lo rodea de inputs que valdrán más cuando exista.

Cada paso adopta capacidad ya construida (auditoría previa: test_runner, knowledge,
rebuild estaban muertos-productivos). Ninguno añade un modelo, tabla o daemon nuevo.

## 6. Evidencia y límites

**DOCUMENTED:** AIS-004 (sin daemon), M7 del spike CC-S0 (sin LLM entre SDDK y
puerto), contrato de gate receipts, `rebuild.rs` canónico.
**OBSERVED:** conteos de la auditoría previa; `apply.md:466-471` construye
evidence_json manual del LLM; githooks/pre-push activo en este repo.
**STRUCTURAL:** `projection_checkpoints_v1` + `delete_checkpoint` idempotente;
`knowledge scan|import` completos sin invocador; test_runner `pub(crate)` sin CLI.
**DERIVED:** los cuatro mecanismos, la tabla de delegación y el orden — pendientes
de admisión como ciclos.

No se implementó nada; no se validó ningún UAT propuesto; restricciones AIS-004 y
fail-closed del ledger se asumen vigentes en toda propuesta. Flake y release
SEC-1: intocados.
