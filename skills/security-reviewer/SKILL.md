---
name: security-reviewer
description: "Trigger: revisión de seguridad, secretos incrustados, permisos excesivos, contenedores privilegiados, comandos destructivos, configuración insegura, ejemplos no aptos para producción. Revisa ejemplos y configuraciones buscando secretos, permisos excesivos, comandos destructivos y configuraciones inseguras, especialmente en libros de DevOps/Kubernetes/redes/backend."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo como sub-pase de `technical-reviewer` cuando el libro toca **infraestructura, redes, backend o cualquier comando ejecutable**. Especialmente necesario en libros de DevOps, Kubernetes, redes o backend.

No la uses para prosa (`editorial-reviewer`).

## Hard Rules

- **Cero** secretos reales en el libro (API keys, tokens, contraseñas).
- Los permisos/privilegios de los ejemplos deben ser los **mínimos** necesarios.
- Los comandos destructivos deben llevar **aviso explícito**.
- Las configuraciones deben marcarse si **no son aptas para producción**.

## Checklist

| Categoría | Qué buscar |
|-----------|------------|
| Secretos | API keys, tokens, contraseñas, connection strings reales |
| Permisos | `chmod 777`, `--privileged`, RBAC excesivo, `sudo` innecesario |
| Contenedores | `--privileged`, `hostPath`, `hostNetwork`, root user |
| Comandos destructivos | `rm -rf`, `dd`, `mkfs`, `DROP TABLE`, `--force` |
| Configuración | TLS desactivado, CORS `*`, debug en prod, sin rate-limit |
| Supply chain | `curl | bash`, pip install sin hash, imágenes sin tag |

## Execution Steps

1. Escanear ejemplos, snippets y bloques de código del capítulo.
2. Para cada hallazgo, clasificar severidad (`critical`/`high`/`med`).
3. Proponer remediación: usar placeholder, reducir permisos, añadir aviso.
4. Emitir `build/reviews/{chapter-id}.security.yml`.

## Output Contract

- `build/reviews/{chapter-id}.security.yml`.
- `verdict`: `PASS` | `PASS_WITH_WARNINGS` | `BLOCKED`.
- `critical` (secreto real, comando destructivo sin aviso) → `BLOCKED`.

## References

- `references/security-checklist.md` — checklist exhaustiva por categoría.
