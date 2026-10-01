---
id: ADR-0150-SURFACE-BREVITY
status: accepted
proposed_at: 2026-10-01
accepted_at: 2026-10-01
accepted_by_cycle: session-65d
date: 2026-10-01
deciders: ["operador (ratificación explícita de 300/150/200, session-65d)", "orchestrator"]
related:
  - ADR-0106-TYPED-INSTRUCTION-COMPILATION
  - ADR-0108-SKILL-IS-NOT-CAPABILITY
  - ADR-0047-DURABLE-DEBT-REMEDIATION
  - INC-DEBT-054-DOCTOR-STRICT-MEASURES-NOTHING
---

# Brevedad de superficies: el presupuesto, su medición y su vía de waiver

## Contexto

Las superficies `agents/*.md`, `skills/*/SKILL.md` y `prompts/sddk/*.md` se cargan en
el contexto del agente cuando se invocan. Su tamaño es un peaje que se paga en cada
uso, no una vez al escribirlas.

Durante años ese peaje no tuvo contrato. Peor: tuvo **una cita que no resolví­a**.
`CHANGELOG.md:2308` registra `docs(adr): ADR-016 surface-brevity` como trabajo hecho, y
`crates/sddk-cli/src/dev/doctor.rs` implementa los tres números citando ese ADR-016.
Rastreados el repositorio entero y todo el historial de git, **ese ADR nunca fue
commiteado**. Los ADR-016 que existen son otros: *outcomes-events-errors*,
*provider-independent-agent-profiles*, *universal-evidence-model*.

El resultado es un concepto con un punto de aplicación y una cita, y ninguna autoridad
canónica. Es la violación de «una autoridad por concepto» que el propio AGENTS.md §2.7
prohíbe, y tiene una consecuencia práctica: nadie podía consultar **qué estructura
produce un corte**, ni **qué hacer cuando una superficie no se puede partir**.

A eso se suma [INC-DEBT-054](./../../../debt/INC-DEBT-054-DOCTOR-STRICT-MEASURES-NOTHING.md):
el gate que debía gravitar ese peaje se anclaba a `current_dir()` con cada
enumeración dentro de `if let Ok(read_dir(..))`, de modo que desde un directorio sin
superficies emitía **0 checks y salía con exit 0**. El instrumento no medía. Medido
contra el binario publicado v2.5.2: `all_present: true`, exit 0, cero checks.

## Medición (session-65d, sobre el árbol y sobre el bundle publicado)

Los números que este ADR ratifica nunca tuvieron una derivación. Esta es la primera.

| medida | valor |
|---|---|
| skills que cumplen el techo de 150 | 78 |
| media de las que cumplen | **3 034 bytes ≈ 758 tokens ≈ 119 líneas** |
| superficies fuera de presupuesto | **19** (2 agents, 14 skills, 3 prompts) |
| tokens de las 19 incumplidoras, sumados | **≈ 68 400** |
| peor caso individual | `prompts/sddk/HTML-REPORT.md`, 1 328 líneas ≈ **14 900 tokens** |
| skills que adoptaron `references/` antes de que existiera regla | **2** (`playwright-cli`, `test-pyramid`) |

El techo de 150 para skills queda a **1,26× la mediana** de lo que ya se cumple. No es
un número arbitrario: es un_backstop_ que casi nada toca hoy.

El dato que más pesa a favor de la regla no está en esta tabla: `playwright-cli` y
`test-pyramid` adoptaron el patrón de `references/` **sin que nadie se lo mandara**, y
lo hicieron partiendo por temas reales (`rust-testing`, `edge-case-grill`,
`session-management`). El patrón se encontró útil antes de existir la regla.

## Decisión

1. **Se ratifican los tres presupuestos**, con la medición de arriba como su
   justificación y no una constante sin razón:

   | superficie | techo |
   |---|---|
   | `agents/*.md` | 300 líneas |
   | `skills/*/SKILL.md` | 150 líneas |
   | `prompts/sddk/*.md` | 200 líneas |

2. **Existe una vía de waiver.** El CHANGELOG afirmaba «sin excepciones nominales» y
   eso **no se ratifica**. Un gate duro sin salida fuerza *cumplir o `--no-verify`*, y
   un gate al que se recurre a saltarse no gobierna nada. El waiver se registra como
   entrada en `docs/debt/` con severidad y prioridad según
   [ADR-0047](./../../adr/ADR-0047-durable-debt-remediation.md), se nombra el
   presupuesto que excede y la fecha de revisión. **`--strict` sigue fallando**: el
   waiver documenta la deuda, no la esconde ni la legaliza.

3. **Se declara la imprecisión conocida.** El gate cuenta **líneas**, y lo que se
   quiere gravar son **tokens**. La media real es 78 bytes por línea, no los ~60 que
   cabe suponer, y una tabla de 150 líneas cuesta bastante menos que 150 de prosa.
   La dirección es medir tokens; hasta entonces, línea es el proxy acordado y su
   error favours que se partiá de más, no de menos.

4. **Un corte tiene una forma definida**, que es la parte que antes no existía:

   **Se queda en la superficie de invocación** el frontmatter; una línea de *gate* si
   la superficie delega en otra; `## Purpose`; el contrato o flujo como lista corta y
   verificable; **un** ejemplo mínimo; y un índice `## References` con el nombre de
   cada fichero de referencia.

   **Se mueve a `references/`** la profundidad por temas. Nombre en minúsculas y
   guiones, un fichero por tema, tal como ya hacen `playwright-cli` (10 ficheros) y
   `test-pyramid` (10 ficheros, 1 287 líneas repartidas).

   Cada fichero de referencia abre diciendo **qué cubre y qué no**, y entrega lo
   excluded a la skill hermana. Es el patrón que ya usa
   `test-pyramid/references/rust-testing.md`, y sin esa línea el corte produce
   superficies que nadie sabe cuándo abrir.

5. **Las referencias se leen bajo demanda.** Un `references/` que se carga entero
   reproduce el problema dentro del archivo y no lo arregla: el presupuesto existe
   para que la profundidad no se pague al invocar.

6. **Una superficie que ya es especificación se parte igual, y por lo que ella misma
   prescribe.** `prompts/sddk/HTML-REPORT.md` describe en su propio texto
   *«Progressive Disclosure (20 sections, 4 layers)»* y son 1 328 líneas en un solo
   fichero: un documento que prescribe disclosure progresivo y no lo ejerce sobre sí
   mismo. Se parte según su propia especificación —el contrato arriba, las 20
   secciones abajo— y **no** por la vía de waiver. Waiver es para lo que no se puede
   partir; aquí sí se puede, y el propio documento dice cómo.

7. **El enforzado no vuelve a ser un gate que no mide.** `sddk dev doctor` es advisory
   por defecto y `--strict` promueve. Cuando **no hay nada que medir** —ni superficies
   en el cwd ni en el framework root activo— se emite `surface.briefness.root` con
   `present: false` y `--strict` sale 1. Una medición vacía no es una medición aprobada.

8. **El remedio es proporcional al exceso: primero se adelgaza, después se parte, y el
   waiver es el último peldaño.** Este punto se añadió por enmienda el 2026-10-01,
   después de intentar ejecutar los puntos 1–7 sobre `skills/uat-discovery/SKILL.md`
   (164 líneas, 14 por encima). Los puntos 1–7 ofrecían **solo dos salidas**: partir
   el fichero o registrar un waiver. Ninguna de las dos sirve para un exceso pequeño y
   real.

   | peldaño | cuándo | qué cuesta |
   |---|---|---|
   | **1. Adelgazar** | el exceso se puede cubrir con contenido redundante | ninguno: se quita duplicación, se pierde nada |
   | **2. Partir** | el exceso es de fondo | un fichero por tema, con su «qué cubre y qué no» |
   | **3. Waiver** | tras 1 y 2 sigue excediendo, y exceder está justificado | una entrada en `docs/debt/` con revisión fechada |

   La medición que motiva el peldaño 1: en `uat-discovery` el `curl` de health check
   estaba escrito dos veces (en Prerequisites y en Phase 1) y el contrato de salidas
   también (en Phase 2 y en la tabla Output Files). Eliminar **solo** esa duplicación
   genuina baja el fichero de 164 a **160**: se recuperan **4 de las 14 líneas** y
   quedan 10 que llevan información. Con el contrato anterior esas 10 obligaban a
   crear ficheros de referencia para ahorrarlas, o a documentar deuda por 14 líneas.
   Ninguna de las dos es una respuesta honesta.

   El peldaño 3 es lo que hace coherente el 2: un exceso marginal tiene salida legítima
   —un waiver documentado y fechado— **porque la decisión 2 abrió esa vía**. Sin
   waiver, un presupuesto con forma de acantilado (149 conforme, 151 incumplido) solo
   puede cumplirse distorsionando el contenido.

## Consecuencias

- **19 superficies incumplirán `--strict` desde ya**, y eso es correcto: es el aviso que
  el gate silencioso reprimía. Reducirlas es trabajo de contenido, en su propio ciclo.
- **Volver a estar bajo el techo deja de ser una pelea**: con el presupuesto y su
  waiver, partir una superficie es el camino barato y saltar el gate el caro.
- **Aparece trabajo nuevo**: el waiver exige escribir la entrada de deuda con la misma
  disciplina que el resto, incluida la teeth que demuestre que el waiver está vivo.

## Lo que este ADR NO hace

- **No reduce las 19 superficies.** Da el contrato contra el cual reducirlas; el corte
  es otro ciclo.
- **No cambia a tokens el enforzado.** Queda declarado como imprecisión conocida con
  dirección, no como deuda cerrada.
- **No toca los presupuestos.** El operador los ratifica como están, con la medición que
  los hace defendibles añadida a su historia.
- **No reabre INC-DEBT-054.** Ese INC cierra el defecto del gate, que ya está corregido
  y validado contra el binario instalado.
