# Naming — SDDK

## Decisión

Conservar las siglas **SDDK** y cambiar su expansión oficial a:

# Software Development Decision Kernel

## Por qué este nombre

### Software
El producto sigue centrado en ingeniería de software, no en automatización genérica.

### Development
Cubre el ciclo completo: exploración, diseño, implementación, verificación, UAT, release, incidentes, seguridad, upgrades y mantenimiento.

### Decision
Es la pieza que mejor explica la evolución del sistema:

- el supervisor decide estrategias;
- las políticas autorizan o rechazan efectos;
- el router decide proveedor/modelo;
- los humanos aceptan/rechazan UAT;
- el sistema registra decisiones y su evidencia;
- forks y replay permiten comparar decisiones alternativas;
- el conocimiento operacional mejora decisiones futuras.

### Kernel
El núcleo debe ser pequeño, estable y agnóstico de dominio. SDD, UAT, incident response, seguridad o release son packs sobre ese kernel.

## Tagline recomendada

> **SDDK — The decision kernel for agentic software engineering.**

Alternativa más descriptiva:

> **An event-sourced reactive control plane for human + agent software delivery.**

## Qué deja de significar

`Specification-Driven Development Kernel` pasa a considerarse el **origen histórico** del nombre y el nombre del pack SDD:

```text
SDDK product
└── sddk-sdd pack
    └── Specification-Driven Development workflow
```

Así preservamos reconocimiento y continuidad sin obligar a que todos los workflows sean SDD.

## Alternativas evaluadas

| Nombre | Ventaja | Problema |
|---|---|---|
| Software Delivery Decision Kernel | Refuerza delivery | Reduce la parte de research/architecture/knowledge |
| Software Development Dynamics Kernel | Refleja reactividad | “Dynamics” es menos claro para usuarios |
| Software Development Distributed Kernel | Encaja con agentes | “Distributed” describe implementación, no propósito |
| Software Development Decision Kernel | Explica gobierno y control | **Elegido** |

## Regla de marca

Usar siempre:

```text
SDDK
Software Development Decision Kernel
```

y describir SDD como un pack, no como el producto completo.
