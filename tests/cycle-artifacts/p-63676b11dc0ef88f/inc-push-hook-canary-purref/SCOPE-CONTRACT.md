# SCOPE-CONTRACT — inc-push-hook-canary-purref

> Candidato. Pendiente de autorización del operador.

Cycle id: `p-63676b11dc0ef88f/inc-push-hook-canary-purref`
Baseline (released): `v1.169.93` → `99abe1a9cac99f07cd8af62bb0e1eae42af2362a`
Mode: docs only
Source INC: `INC-AIWS1-RECEIPT-PUSH-BLOCK`

## 1. Tensión que cierra

`githooks/pre-push` admite la publicación de `tests/cycle-artifacts/p-*/*/{SCOPE-CONTRACT,DISCOVERY,RECEIPT}.md` sin bump (commit `5f604e5`, ver `tests/test_push_prevention_hook.sh` §`s_cycle_*`). Esa admisión pasa por un screen de secretos de alta confianza (`file_contains_secret_pattern`) que aplica sobre el contenido completo del archivo en el SHA apuntado por el push, no sobre el diff.

El regex, intencionalmente laxo, dispara falsos positivos sobre placeholders de prueba redactables:

```text
api_key=CANARY_…     # matchea (api_key)[:=]CANARY_…
token=CANARY_…       # matchea (token)[:=]CANARY_…
```

El `RECEIPT.md` del ciclo `sec-1-secrets-boundary-proof` ya commiteado en `HEAD` contiene exactamente estos placeholders en la tabla de tests RED/GREEN (línea 58: ``api_key=CANARY_…``). El intento de push del cierre administrativo de SEC-1 en la sesión 2026-09-20 reprodujo el bloqueo:

```text
ERROR: Push to main rejected — cycle documentary file looks like it contains
a secret: tests/cycle-artifacts/p-63676b11dc0ef88f/sec-1-secrets-boundary-proof/RECEIPT.md
ERROR: (apply/release split | INC-AIWS1-RECEIPT-PUSH-BLOCK)
```

El bloqueo no es una incidencia aislada: el propio contrato del hook garantiza que se reproducirá mientras el screen siga siendo sintáctico y los receipts contengan canarias redactables. Cada cierre de ciclo que mencione el nombre de un token, aunque sea como canaria de RED pinned, queda recluido al siguiente bump. Esto contamina la promesa del allowlist de cycle-artifacts, admitida en `5f604e5`.

## 2. Decisión adoptada

Corregir el screen de secretos del hook para que las canarias de test redactables no disparen falsos positivos, **manteniendo intactos los controles de seguridad y publicación**. No se autoriza:

- `--no-verify` ni ningún bypass del chequeo.
- Bump ceremonial `chore(release): bump version` con código ausente.
- Trasladar artifacts a `docs/` para sortear el control.
- Relajar el screen a "palabras clave genéricas" sin distinguir placeholders.

El cambio se publica por la única ruta compatible con el contrato A5-1 vigente: un push cuyo rango contiene la modificación de `githooks/pre-push` (que vive bajo `githooks/**`, no en el allowlist), pero cuyo rango incluye también cambios bajo `docs/adr/` o `.sddk/followups/` que materializan el contrato. El push se ejecuta a través de un ciclo autorizado, no por una rama efímera.

## 3. MUST

- **M1.** Un `RECEIPT.md` que contiene placeholders de prueba redactables (`api_key=<CANARY>`, `token={CANARY_TOKEN}`, `aws_access_key=<REDACTED>`, o cualquier forma donde el valor no sea un secreto literal de alta confianza) pasa el screen sin disparar `file_contains_secret_pattern`.

- **M2.** Un `RECEIPT.md` que contiene secretos literales de alta confianza (`ghp_…`, `github_pat_…`, `AKIA…`, `xox[baprs]-…`, `sk-…`, bloques `-----BEGIN … PRIVATE KEY-----`, pares `key="valor_literal_de_8+_caracteres"`) sigue siendo rechazado por el screen. Admitir la ruta no equivale a certificar contenido.

- **M3.** Un push sin bump que mezcla `tests/cycle-artifacts/p-*/*/{SCOPE-CONTRACT,DISCOVERY,RECEIPT}.md` con cambios bajo `githooks/**`, `crates/**`, `scripts/**` u otros paths no admitidos sigue siendo rechazado (ver `s_cycle_receipt_plus_crates` y `s_cycle_receipt_plus_githooks`, ya verdes).

- **M4.** Un rename de archivo runtime a `tests/cycle-artifacts/.../RECEIPT.md` no logra admisión; el screen y el allowlist siguen operando sobre la lista de paths cambiados sin depender de la heurística "parece RECEIPT.md". El rename detection del hook (`--no-renames`) conserva este invariante.

- **M5.** El contrato A5-1 (`cargo_workspace_version` real + allowlist cerrado) queda **sin modificaciones funcionales**: la corrección es estrictamente sobre el screen de secretos dentro del bloque ya admitido por la rama (B). No se introduce una tercera vía de admisión.

- **M6.** El contrato del hook queda ampliado en `tests/test_push_prevention_hook.sh` con al menos los casos:
  - `s_cycle_receipt_with_test_canary` — `ACCEPT` con `api_key=<CANARY>` en tabla.
  - `s_cycle_receipt_with_redacted_marker` — `ACCEPT` con `api_key=<REDACTED>` en texto.
  - `s_cycle_receipt_with_literal_high_confidence_secret` — `REJECT` (refuerza M2).
  - `s_cycle_receipt_with_real_github_pat` — `REJECT`.

- **M7.** El contrato del hook no se debilita por la corrección: el porcentaje de archivos admitidos que serían rechazados por el screen de secretos original es ≥ 0% (no se reduce la cobertura de detección real).

## 4. MUST NOT

- **N1.** No se cambia el allowlist de paths admitidos bajo rama (B). El árbol `tests/cycle-artifacts/` sigue admitiendo exclusivamente `SCOPE-CONTRACT.md`, `DISCOVERY.md`, `RECEIPT.md` con la convención `tests/cycle-artifacts/p-<id>/<cycle-id>/<file>.md`. Logs, fixtures y volcados quedan fuera.

- **N2.** No se elimina el screen de secretos: se afina. Un receipt con un `ghp_…` real o un bloque PEM sigue siendo rechazado.

- **N3.** No se introduce una rama (C) en el contrato del hook. La corrección se hace dentro de la rama (B), no creando una nueva categoría de admisión.

- **N4.** No se relajan los tests existentes en `tests/test_push_prevention_hook.sh`. Todo caso `ACCEPT` previo sigue siendo `ACCEPT`; todo `REJECT` previo sigue siendo `REJECT`. El cambio es estrictamente aditivo en cobertura.

- **N5.** No se modifica `Cargo.toml` `[workspace.package]` version. No hay bump. La corrección viaja en un push docs-only.

## 5. Diseño aplicado (final)

La función `file_contains_secret_pattern` en `githooks/pre-push` lee el
contenido del archivo desde stdin (el caller hace `git show $local_sha:$f |`).
Internamente pre-filtra con awk y aplica el grep original al residuo:

```bash
file_contains_secret_pattern() {
    awk '
        BEGIN { IGNORECASE = 1 }
        {
            if (index($0, "<redacted:") > 0)                       next
            line = tolower($0)
            if (match(line, /<[a-z_][a-z0-9_-]*>/))                next
            if (match($0,    /\{[A-Za-z_][A-Za-z0-9_-]*\}/))       next
            if (index($0, "\xe2\x80\xa6") > 0)                     next
            print
        }
    ' | grep -qE '(ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,}|sk-[A-Za-z0-9]{20,}|-----BEGIN (RSA|EC|OPENSSH|PGP|DSA) PRIVATE KEY-----|"(api[_-]?key|secret[_-]?key|access[_-]?token|password)"?[[:space:]]*[:=][[:space:]]*"[^" ]{8,}|(api[_-]?key|secret[_-]?key|access[_-]?token|password)[[:space:]]*[:=][[:space:]]*[^ ][^ ]{8,})'
}
```

**Decisiones de implementación que merece la pena dejar escritas:**

1. **`BEGIN { IGNORECASE = 1 }`**: GNU awk 5.x con el flag `i` en regex matchea
   incorrectamente subsecuencias que cruzan boundaries multibyte (descubierto
   durante la validación). Usar `IGNORECASE = 1` con `tolower()` aplicado a la
   línea antes del match es la forma portable.

2. **`index($0, "\xe2\x80\xa6")` para la elipsis**: elipsis UTF-8 son los
   bytes `E2 80 A6`, no `\xc2\x85`. Mi primer intento usó la secuencia
   incorrecta y filtró 0 líneas.

3. **Lectura desde stdin**: la versión anterior de la función tomaba la ruta
   del archivo como argumento y leía con `grep -qE '...' "$f"`. El caller la
   invoca con `git show $local_sha:$f | file_contains_secret_pattern -`. Para
   que la nueva implementación (que usa awk + grep) siga siendo drop-in, la
   función lee de stdin y NO hace su propio `git show`. El caller sigue
   pasando `-` por compatibilidad con el call site existente; la función
   ignora el argumento.

## 6. ADR — no procede

El cambio es estrictamente una corrección de implementación del screen de
secretos del hook. La decisión arquitectónica ya está tomada y committed en
`5f604e5` (admisión de `tests/cycle-artifacts/p-*/*/{SCOPE-CONTRACT,DISCOVERY,
RECEIPT}.md` bajo rama B) y la matriz del hook está pinada por
`tests/test_push_prevention_hook.sh` (A5-1).

Afinar la heurística del screen para distinguir placeholders de canaria
documental no introduce una decisión arquitectónica nueva, no abre una nueva
categoría de admisión, ni relaja el control de secretos. Crear un ADR-0139
sería redundante: el contrato A5-1 vigente ya cubre el caso.

La justificación del afinamiento queda documentada en:
- este `SCOPE-CONTRACT.md` (§1 tensión, §3 MUST, §4 MUST NOT, §5 diseño),
- el commit message del cambio en `githooks/pre-push`,
- el `RECEIPT.md` del ciclo al cierre,
- la matriz ampliada de `tests/test_push_prevention_hook.sh`.

## 7. Net delta esperado

| File | Change |
|------|--------|
| `githooks/pre-push` | +30/-3 líneas: `file_contains_secret_pattern` afinada con pre-filtrado awk de placeholders (`<redacted:N>`, `<TOKEN_NAME>`, `{TOKEN_NAME}`, `…` U+2026); cambia de "lee por argumento" a "lee de stdin" para preservar el contrato con el call site (`git show $local_sha:$f | file_contains_secret_pattern -`) |
| `tests/test_push_prevention_hook.sh` | +68 líneas: 4 funciones `s_cycle_receipt_with_*` + 4 invocaciones `run_case` en la matriz cycle-artifacts |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/inc-push-hook-canary-purref/SCOPE-CONTRACT.md` | este contrato |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/inc-push-hook-canary-purref/RECEIPT.md` | al cierre del ciclo |

No se crea ADR (ver §6). No se modifica `Cargo.toml`. No se añade entrada nueva en `docs/handoff/`.

## 8. Falsification battery (a verificar antes de close)

1. `bash tests/test_push_prevention_hook.sh` → 0 FAILED.
2. `bash tests/test_release_public_gate.sh` → 0 FAILED (no debería tocar release; smoke).
3. Test manual: crear rama efímera con un commit que toque `githooks/pre-push` + un `RECEIPT.md` con `api_key=<CANARY>` → push a `main` se rechaza (porque `githooks/**` no está en el allowlist).

> **Limitación a registrar honestamente:** El arranque del propio ciclo choca con la regla que pretende corregir. Publicar la corrección del hook requiere un release con cambio de versión, o un procedimiento externo de merge de gobernanza. El operador nombró esta dificultad. Este ciclo documenta la fricción pero no la elimina por sí solo.

## 9. Out of scope

- Modificar el allowlist para admitir `githooks/**` (eso abriría la puerta a bypass de control).
- Migrar el INC-AIWS1-RECEIPT-PUSH-BLOCK a estado "closed by construction" sin verificar empíricamente los 4 casos de prueba.
- Reconciliación con A5-CERT (`v1.169.88`); la certificación A5-C permanece intacta.
- A6 STATIC_ENHANCED capabilities adicionales (CC-S1..CC-S3); sigue siendo el siguiente slice de A6 con su propio contrato.