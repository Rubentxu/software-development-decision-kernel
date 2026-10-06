# SDDK PRE-FLIGHT — `cl-release-step1-memory-guard`

> Emitido **antes** de modificar código, conforme al protocolo global
> (`/home/rubentxu/AGENTS.md` §2). Con `Readiness != READY` está prohibido tocar.

| Campo | Valor |
|---|---|
| **Project** | `p-63676b11dc0ef88f` (`sddk project resolve`, `adopt status: complete`) |
| **Goal** | Que el pipeline de release **nunca muera sin decir por qué**. Es el camino de entrega de todo lo demás: si esto muere en silencio, no hay artefacto y no hay recibo. |
| **Milestone** | M7 / release pipeline (`agent-session start` → `authority=sddk`, `head=36462c10`) |
| **WorkItem** | Backlog `bl-bl-01M42JGYG4000388551BF9NZ40` — **P1, Triaged**, `origin_cycle: c3n-production-boundary-certification`, `origin_phase: verify`. Es el **único ítem vivo** del backlog (`sddk backlog list`). |
| **HEAD** | `36462c10` |
| **Objective** | Que el paso 1 mida los recursos **en el punto donde se consumen**, no solo en el preflight, y que un fallo del paso 1 **nombre** el estado de memoria y disco de ese instante. |
| **Definition of Done** | 1) `release_check_resources` se invoca inmediatamente antes del paso 1 y aborta con causa nombrada. 2) Los tres `cargo` del paso 1, al fallar, imprimen el estado de recursos **de ese momento**. 3) Guard que lo ejerce + autofalsación que le quita los dientes uno a uno. 4) `shellcheck` limpio **sin filtro de severidad** (el 1b corre así). 5) Cierre registrado en el ítem del backlog vía SDDK. |
| **Required inputs** | `scripts/lib/release_diagnostics.sh` (autoridad ya existente de diagnóstico), `scripts/release.sh` paso 1, `tests/test_release_diagnostics*.sh` (patrón de guard y falsador). |
| **Known decisions** | No se fabrican claves ni se altera la postura de firma. No se bumpea la versión. El diagnóstico se **extiende** sobre `release_diagnostics.sh`, que ya es la autoridad única, en vez de crear un segundo juez de recursos. |
| **Known blockers** | **Escritor concurrente activo en el mismo worktree** (§ abajo). `scripts/release.sh` estaba modificado por otro proceso a las 11:27:05. Mitigación: 716 s de silencio medidos antes de editar, y verificación de que la edición sobrevive antes de commitear. |
| **Missing information** | Cuánta memoria **de verdad** consume `cargo test --workspace` en el pico: **no está medido**. Consecuencia: el umbral **no se toca**; sólo se mueve *dónde* y *cuándo* se mide. Declarado, no supuesto. |
| **Next executable task** | Añadir `release_resources_now` a la lib y cablear el paso 1. |
| **Readiness** | **READY** |

---

## El hallazgo que abre el bloque, y que no estaba en la descripción del ítem

El ítem dice «el paso 1 no tiene cota de memoria». Leyendo el código sale otra
cosa, y es más incómoda:

```
scripts/release.sh:312   release_check_resources "$RELEASE_SCRATCH"   ← UNA vez, en el 0
scripts/release.sh:389   step "1/15 — cargo fmt + clippy + test"
```

**La comprobación de memoria existe y es fail-closed. Lo que no existe es medirla
donde se consume.** El preflight mide la máquina *antes*; el paso 1 es el pico de
consumo *después*, y entre ambos la máquina la carga lo que sea. Eso es
exactamente el caso medido: el fallo ocurrió con **2.4 GiB libres y swap al 100 %**
mientras corrían **~313 clones de otro proyecto** sobre un tmpfs de 48 G y
PipelineK ejecutando su CI. Nada de eso estaba en el preflight.

Y hay un segundo dato que desarma la solución obvia: **el umbral declarado
(`SDDK_RELEASE_MIN_AVAIL_MB=2048` MB) está por debajo del punto en el que se
falló** (2.4 GiB ≈ 2457 MB). Subir el umbral parecería el arreglo y sería
adivinar la magnitud del pico de `cargo test --workspace`, que **nadie ha
medido**. Por eso este bloque **no toca el umbral**: mueve la medición, que es
lo que está medido que falla.

## Conocimiento negativo (lo que este bloque NO va a hacer)

- **No va a poner una cota dura al `cargo`.** `ulimit -v` está descartado: Rust
  reserva espacio virtual muy por encima de su RSS y el proceso moriría por un
  motivo inventado. `systemd-run --scope -p MemoryMax=` necesita un bus de
  sesión que no existe en todos los entornos donde corre este release. Añadirlo
  al camino de entrega sin haberlo medido en este entorno sería cambiar el
  pipeline a ciegas.
- **No va a arreglar el caso en que mata al `release.sh` entero.** Si el OOM
  killer se lleva el proceso del release, el `trap` no corre y no hay `EXIT=`
  que imprimir: eso es detección post-mortem y es otro bloque.
- **No va a tocar `c3n`** (está en `approval-waiting` por
  `surface.cycle_state#cycle_supersede`) ni `INC-DEBT-050`/`061`, que son
  decisiones del operador.

## Riesgo asumido, escrito antes de que ocurra

Dos agentes escriben este árbol. Si mientras trabajo en `scripts/release.sh` el
otro proceso reescribe el fichero, mi edición se pierde **o** la suya. Mitigación
declarada: medir el silencio del fichero inmediatamente antes de editar, y
**comprobar que mi diff sigue ahí** antes de commitear. Si no está, se para el
commit y no se reintenta a ciegas.
