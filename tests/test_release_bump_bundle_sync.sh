#!/usr/bin/env bash
# tests/test_release_bump_bundle_sync.sh
#
# Por que este guard existe, y que NO cubre:
#
# `test_release_bump_derivation.sh` ya fija que release-bump.sh CALCULA bien la
# siguiente version. Lo que no fijaba era que la APLIQUE a todos los ficheros
# que tienen que moverse con ella. Medido en session-70: el bump 2.5.3 -> 2.5.4
# movio Cargo.toml y Cargo.lock y se dejo `BUNDLE.toml` en 2.5.3, y la release
# murio en el paso 1b con
#   `BUNDLE.toml version 2.5.3 != workspace 2.5.4 (fosil: regenerar…)`
#
# El mensaje culpa a un fosil y no menciona la causa —que el bump se dejo un
# fichero trackeado por delante sin avisar—, asi que el que lea el fallo no sabe
# que hacer. Y lo que si sabia era lo de siempre: arreglarlo a mano. Un paso
# manual que solo lo caza un gate, en el momento de publicar, es un defecto del
# bump, no del gate.
#
# La regla que ata: **un bump es una operacion atomica sobre TODA la superficie
# versionada.** Si un fichero trackeado declara la version del bundle, el bump
# lo mueve; si no lo mueve, alguien lo esta moviendo a mano y el proximo bump
# vuelve a morir en 1b.
#
# Tres invariantes, y la tercera es la que no es obvio:
#   (1) Las TRES claves del rango se mueven (version, min, max). Mover solo
#       `version` deja un rango que excluye el binario que el release acaba de
#       construir, y `dev install --source` lo rechaza.
#   (2) Si el rango se abre a mano (min != max) el bump lo cierra en la nueva
#       version, porque un rango abierto que no incluye el propio bundle es
#       un bundle que no se puede instalar.
#   (3) `schema_version` NO se mueve: empieza por otro ancla y shares prefix
#       con `version`. Un `sed` que no ancla a linea deja `schema_version`
#       reescrito cuando `version = "2"` casa dentro de `schema_version = "2"`.
#       Ese es el mismo literal-suelto que ya costo una sesion en este repo.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUMP="${SDDK_BUMP_SH:-$REPO/scripts/release-bump.sh}"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

[ -f "$BUMP" ] || { echo "FALLO: no existe $BUMP"; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ── Extraccion del bloque REAL ─────────────────────────────────────────────
# Se ejecuta el bloque del producto, no una reescritura. Si el bloque se
# degrada a "esto es lo que hace", el guard mediria la copia.
awk '
    # Se extrae el `if [ -f BUNDLE.toml ]` COMPLETO: desde su linea de apertura
    # hasta el primer `fi` en columna 0, que es su cierre. Dos extrusion
    # fallidas quedaron documentadas aqui porque las dos parecian razonables:
    #
    #  · Parar en el primer `^}$` se lleva por delante el resto del script —el
    #    bloque termina en `fi`, no en `}`— y se vio imprimir "applied: ->
    #    2.5.4" y "changed files:", que son del resto del producto. Un guard
    #    que ejecuta de mas no mide solo lo que dice medir.
    #  · Contar anidamiento con `^(if|for|...)` falla porque los `for`/`done`
    #    van indentados y `^` no los alcanza, luego el `for` nunca suma y su
    #    `fi` de cierre queda sin emparejar.
    #
    # El criterio que si funciona es el literal: el bloque abre y cierra en
    # columna 0, y ningun otro `fi` sin sangrar cae dentro.
    /^if \[ -f BUNDLE\.toml \]; then$/ { inblock = 1 }
    inblock { print }
    inblock && /^fi$/ { exit }
' "$BUMP" > "$WORK/sync.sh"

if ! grep -q 'BUNDLE.toml' "$WORK/sync.sh"; then
    bad "release-bump.sh no mueve BUNDLE.toml — toda release futura muere en 1b en el primer bump"
    echo
    echo "PASS=$PASS FAIL=$((FAIL + 1)) SKIP=0"
    exit 1
fi
ok "el bloque de sincronizacion de BUNDLE.toml existe en el producto"

# ── Arnes ──────────────────────────────────────────────────────────────────
# El bloque hace `sed -i` sobre BUNDLE.toml del directorio de trabajo y lee el
# resultado, asi que se ejecuta con cwd en la arena.
run_sync() {
    local next="$1" dir="$2"
    ( cd "$dir" && NEXT="$next" bash "$WORK/sync.sh" ) 2>"$dir/stderr"
    return $?
}

make_bundle() {
    local dir="$1" ver="$2" min="$3" max="$4"
    mkdir -p "$dir"
    cat > "$dir/BUNDLE.toml" <<EOF
[bundle]
schema_version = 2
version = "$ver"
binary_min_version = "$min"
binary_max_version = "$max"

[contents]
agents_count = 72
manifest_sha256 = "sha256:deadbeef"
EOF
}

field() {
    sed -n "s/^$1 = *\"\([^\"]*\)\".*/\1/p" "$2/BUNDLE.toml" | head -1
}

# ══════════════════════════════════════════════════════════════════════════
# S1 — bump normal: las tres claves se mueven juntas.
# ══════════════════════════════════════════════════════════════════════════
D="$WORK/s1"; make_bundle "$D" "2.5.3" "2.5.3" "2.5.3"
if run_sync "2.5.4" "$D"; then
    if [ "$(field version "$D")" = "2.5.4" ] \
    && [ "$(field binary_min_version "$D")" = "2.5.4" ] \
    && [ "$(field binary_max_version "$D")" = "2.5.4" ]; then
        ok "S1 bump normal: version, min y max quedan las tres en 2.5.4"
    else
        bad "S1 bump normal: quedo version=$(field version "$D") min=$(field binary_min_version "$D") max=$(field binary_max_version "$D") — un rango que excluye el binario recien construido hace que dev install --source lo rechace"
    fi
else
    bad "S1 el bloque de sincronizacion fallo: $(cat "$D/stderr")"
fi

# ══════════════════════════════════════════════════════════════════════════
# S2 — rango abierto a mano: el bump lo cierra en la nueva version.
# ══════════════════════════════════════════════════════════════════════════
D="$WORK/s2"; make_bundle "$D" "2.5.3" "2.5.0" "2.9.9"
if run_sync "2.5.4" "$D"; then
    if [ "$(field binary_min_version "$D")" = "2.5.4" ] \
    && [ "$(field binary_max_version "$D")" = "2.5.4" ]; then
        ok "S2 rango abierto: el bump lo cierra en 2.5.4 en vez de dejarlo apuntando a otro binario"
    else
        bad "S2 rango abierto: quedo min=$(field binary_min_version "$D") max=$(field binary_max_version "$D")"
    fi
else
    bad "S2 el bloque de sincronizacion fallo: $(cat "$D/stderr")"
fi

# ══════════════════════════════════════════════════════════════════════════
# S3 — schema_version NO se mueve. `version = "2"` casa dentro de
# `schema_version = "2"`, asi que un sed sin ancla de linea lo reescribe, y un
# schema_version corrupto hace que `sddk dev install` rechace el bundle entero
# por un motivo que no menciona el schema.
# ══════════════════════════════════════════════════════════════════════════
D="$WORK/s3"; make_bundle "$D" "2.5.3" "2.5.3" "2.5.3"
run_sync "2.5.4" "$D" >/dev/null 2>&1
SCHEMA="$(sed -n 's/^schema_version = *\([0-9]*\).*/\1/p' "$D/BUNDLE.toml" | head -1)"
if [ "$SCHEMA" = "2" ]; then
    ok "S3 schema_version intacto en 2 tras el bump"
else
    bad "S3 schema_version quedo en '$SCHEMA' — el sed no ancló a línea y 'version = \"2\"' casa dentro de 'schema_version = \"2\"'"
fi

# ══════════════════════════════════════════════════════════════════════════
# S4 — CONTROL DE NO-VACUIDAD. Un bloque que no escribiera nada pasaria S2
# (que empieza ya en el valor buscado) sin medir nada. Se exige que el
# contenido REALMENTE cambie, y que cambie en las tres claves.
# ══════════════════════════════════════════════════════════════════════════
D="$WORK/s4"; make_bundle "$D" "9.9.9" "9.9.9" "9.9.9"
BEFORE="$(cat "$D/BUNDLE.toml")"
run_sync "2.5.4" "$D" >/dev/null 2>&1
AFTER="$(cat "$D/BUNDLE.toml")"
if [ "$BEFORE" = "$AFTER" ]; then
    bad "control de no-vacuidad: el bump a 2.5.4 no cambio NADA en un bundle en 9.9.9 — el guard no mide nada"
elif [ "$(field version "$D")" = "2.5.4" ] && [ "$(field binary_min_version "$D")" = "2.5.4" ] && [ "$(field binary_max_version "$D")" = "2.5.4" ]; then
    ok "control de no-vacuidad: el bump SI reescribe las tres claves cuando el valor es otro"
else
    bad "control de no-vacuidad: el fichero cambio pero no a 2.5.4 en las tres claves"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=0"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — un bump mueve BUNDLE.toml entero, y schema_version queda intacto."
    exit 0
fi
echo "RESULT: FAIL — el bump deja superficie versionada por delante."
exit 1
