# INC-DEBT-038 — `dev install --source` instala bundle sin versionar y sin symlink `current`

**Fecha de registro:** 2026-09-29 (session-37)
**Severidad:** medium / **Prioridad:** P2
**Estado:** OPEN (mitigado para coherencia de superficies; el recibo sigue rojo)

## Síntoma observado (evidencia real, session-36/37)

`sddk dev install --prefix <P> --source .` desde el checkout:

1. Valida el bundle del checkout (manifest, compat, ancla) — **correcto**.
2. Copia las superficies (`agents/`, `skills/`, `prompts/sddk/`, `assets/`) **directamente
   a `<P>/`** (install.rs:169-171) y `MANIFEST.sha256` + `BUNDLE.toml` a la raíz del
   prefix (install.rs:174-181). **No crea** `framework/<version>/` y **no crea** el
   symlink `<P>/framework/current`.
3. El recibo (`sddk-install.json`) queda en schema v2 con
   `bundle_version: "2.2.32"`, `bundle: true`.

Resultado: `sddk dev doctor --prefix <P>` reporta
**`binary.bundle_coherence: missing`** y `all_present: false`, porque:

- `resolve_active_framework_root` (`dev/paths.rs:59`) resuelve `<P>/framework/current`
  (no existe → fallback "highest version dir" → no hay dirs de versión → error), y
- la comparación de coherencia del doctor (doctor.rs:359-395) requiere
  `receipt.bundle_version == <versión del bundle activo>`, y el bundle activo nunca
  se instaló como versión.

El mismo síntoma dejó el prefix real del operador incoherente en session-36
(recibo 2.2.32 vs `framework/current` → 2.2.27). La única vía que instala coherente
es `scripts/install.sh` (desde tarball, crea `framework/<v>/` + `current`) o
`dev update` (tarball, extrae versionado).

## Causa raíz

Dos consumers con contratos distintos sobre la misma operación:

- **install.rs `--source`** está pensado para dogfooding: "pon mis superficies
  frescas en el prefix AHORA" — copia plana, sin versionar.
- **doctor** asume el layout de `install.sh`: `framework/<v>/` + `current`, y
  contrasta el recibo contra ESE layout.

Ninguno está "mal" por separado; el bug es que `--source` escribe un recibo v2 con
`bundle_version` que el layout que él mismo produce no puede satisfacer. El recibo
afirma una coherencia que la instalación no crea.

## Opciones de resolución (pendiente de decidir en ciclo propio)

1. **`--source` versiona como install.sh:** crear `framework/<bundle_version>/`,
   copiar superficies + BUNDLE.toml + MANIFEST allí, mover `current` al nuevo dir.
   Más coherente, pero cambia el flujo de dogfooding (cada install crea una versión).
2. **Recibo honesto para layout plano:** cuando `--source` instala plano, escribir el
   recibo con `bundle_version: null` (equivalente pre-v2) o un campo nuevo
   `layout: "flat"`, y que el doctor trate ese caso como "coherencia no aplicable"
   en vez de `missing`. Mínimo invasivo; el doctor deja de mentir.
3. **Doctor tolera layout plano:** si `framework/current` no existe pero las
   superficies del prefix verifican contra el MANIFEST del prefix, reportar
   `bundle_coherence: flat-install (present, sin versionado)` en vez de missing.

La opción 2+3 juntas son las más baratas y honestas; la 1 es la "correcta" a largo
plazo pero toca el contrato del dogfooding.

## Evidencia de reproducción (session-37, 2026-09-29)

```bash
TMP=$(mktemp -d)
sddk dev install --prefix $TMP/prefix --source .   # exit 0
sddk dev doctor --prefix $TMP/prefix               # binary.bundle_coherence: missing
ls $TMP/prefix/framework                           # no existe
cat $TMP/sddk-install.json | jq .bundle_version    # "2.2.32"
rm -rf $TMP
```

## Relacionado

- INC-DEBT-025 (ancla del manifest, cerrada) — el ancla ya se valida en `--source`.
- Guard nuevo `tests/test_dev_install_source_guard.sh` (commit `edf148cb`): evita la
  reincidencia del fósil que motivó el descubrimiento; NO cubre este bug de layout.
