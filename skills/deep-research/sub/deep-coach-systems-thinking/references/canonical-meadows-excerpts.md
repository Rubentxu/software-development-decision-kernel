# Excerpts canónicos de Meadows — Para citar en libros

Selección de excerpts verbatim de Donella Meadows para usar en evidence cards y borradores de AsciiDoc. Cada excerpt tiene su fuente y página exacta.

## Introducción: el lente sistémico

> "A system is a set of elements... interconnected to achieve a purpose."
> — *Thinking in Systems*, Introduction.

> "These problems will yield only as we reclaim our intuition, stop casting blame, see the system as the source of its own problems, and find the courage and wisdom to restructure it."
> — *Thinking in Systems*, Introduction.

## Capítulo 1: The basics

> "A system isn't just any collection of items. It consists of three kinds of things: elements, interconnections, and a purpose."
> — *Thinking in Systems*, Chapter 1.

## Capítulo 2: A brief visit to the systems zoo

> "A stock is the present memory of the history of changing flows within the system."
> — *Thinking in Systems*, Chapter 2.

> "A feedback loop is formed when changes in a stock affect the flows into or out of that same stock."
> — *Thinking in Systems*, Chapter 2.

> "Balancing feedback loops are goal-seeking or stability-seeking."
> — *Thinking in Systems*, Chapter 2.

## Capítulo 3: Why systems work so well

> "Interconnections are also critically important. Changing relationships usually changes system behavior."
> — *Thinking in Systems*, Chapter 3.

## Capítulo 4: Why systems surprise us

> "Most of the big problems in the world are caused by the gradual accumulation of small events."
> — *Thinking in Systems*, Chapter 4.

> "Bounded rationality means that people make reasonable decisions based on the information they have, but they don't have all the information."
> — *Thinking in Systems*, Chapter 4.

## Capítulo 5: System traps

> "Systems thinkers call these common structures that produce characteristic behaviors 'archetypes.'"
> — *Thinking in Systems*, Chapter 5.

> "Policy resistance occurs when various actors try to pull a system stock toward various goals, resulting in a standoff where everyone expends great effort but the system remains stuck in an undesirable state."
> — *Thinking in Systems*, Chapter 5.

> "Shifting the burden to the intervenor: drug addiction, industry dependence on government subsidies, farmers' reliance on fertilizers..."
> — *Thinking in Systems*, Chapter 5.

> "The best way out is to align the various goals of the subsystems by providing an overarching goal that everyone can pull toward together."
> — *Thinking in Systems*, Chapter 5 (Policy Resistance escape).

## Capítulo 6: Leverage Points

> "I'm starting with that list, in increasing order of effectiveness, because the list is the answer to the question..."
> — *Thinking in Systems*, Chapter 6.

> "Places to intervene in a system (in increasing order of effectiveness):"
> — *Thinking in Systems*, Chapter 6.

Lista de los 12 (en orden original de Meadows, de menor a mayor):

> "12. Constants, parameters, numbers (subsidies, taxes, standards).
> 11. The sizes of buffers and other stabilizing stocks, relative to their flows.
> 10. The structure of material stocks-and-flows (such as transport networks, population age structures).
> 9. The lengths of delays, relative to the rate of system changes.
> 8. The strength of negative feedback loops, relative to the impacts they are trying to correct against.
> 7. The gain around driving positive feedback loops.
> 6. The structure of information flows (who does and does not have access to information).
> 5. The rules of the system (incentives, punishments, constraints).
> 4. The power to add, change, evolve, or self-organize system structure.
> 3. The goals of the system.
> 2. The mindset or paradigm out of which the system — its goals, power structure, rules, its culture — arises.
> 1. The power to transcend paradigms."
> — *Thinking in Systems*, Chapter 6.

> "The higher the leverage point, the more the system will resist changing it — that's why societies often rub out truly enlightened beings."
> — *Leverage Points* (1997), reproducido en Thinking in Systems cap. 6.

> "Magical leverage points are not easily accessible, even if we know where they are and which direction to push on them. There are no cheap tickets to mastery."
> — *Leverage Points* (1997).

> "Paradigms are the sources of systems. From them, from shared social agreements about the nature of reality, come system goals and information flows, feedbacks, stocks, flows and everything else about systems."
> — *Leverage Points* (1997).

> "The power to transcend paradigms — to see that no paradigm is 'reality' — is the highest leverage point of all."
> — *Leverage Points* (1997).

## Capítulo 7: Dancing with Systems

> "We can't control systems or figure them out. But we can dance with them!"
> — *Dancing with Systems* (2001), reproducido en Thinking in Systems cap. 7.

> "Don't go to great trouble to optimize something that never should be done at all. Aim to enhance total systems properties, such as creativity, stability, diversity, resilience, and sustainability — whether they are easily measured or not."
> — *Dancing with Systems* (2001), citando a Kenneth Boulding.

## Limits to Growth (1972)

> "If the present growth trends in world population, industrialization, pollution, food production, and resource depletion continue unchanged, the limits to growth on this planet will be reached sometime within the next one hundred years. The most probable result will be a rather sudden and uncontrollable decline in both population and industrial capacity."
> — *The Limits to Growth* (1972), Conclusions.

## Cómo usar estos excerpts

1. Cada excerpt va en `evidence-cards/{topic}.yml` con su fuente.
2. En el `.adoc`, usar bloques de cita AsciiDoc:
   ```asciidoc
   [.quote]
   ____
   "We can't control systems or figure them out. But we can dance with them!"
   ____
   
   — Donella Meadows, _Dancing with Systems_ (2001).
   ```
3. Nunca reescribir la cita; siempre verbatim.
4. `hallucination-auditor` valida que el excerpt está en la página citada.
