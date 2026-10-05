#!/usr/bin/env bash
# Guard del merge del changelog de `release-bump.sh`.
#
# ESTE TEST NO COPIA EL CODIGO QUE VIGILA
# ---------------------------------------
# La version anterior (session-30) pegaba el bloque del merge de
# `release-bump.sh` literalmente en el propio test, con el comentario
# "copied verbatim from release-bump.sh". Consecuencia medida: cambiar el
# merge **no movia el test**, luego el unico guard de ese bloque daba
# verde contra la copia antigua, y el arreglo correcto — deduplicar — era
# invisible para el guard que deberia exigirlo. Es la clase "una copia del
# codigo no vigila el codigo"; aqui era peor que en otros casos porque el
# objeto exclusivo del guard es justamente ese bloque.
#
# Aqui el test SOURCEA `scripts/lib/changelog_merge.sh` y LLAMA a
# `changelog_merge`. Si el release cambia el merge, este test cambia con
# el, porque es el mismo codigo.
#
# LO QUE ESTE TEST EXIGE
# ----------------------
# 1. La estructura del changelog sobrevive: una sola cabecera por version,
#    la seccion siguiente y su contenido intactos.
# 2. Un item NUEVO entra, y uno YA REPRESENTADO NO se duplica (INC-DEBT-074).
# 3. La prosa escrita a mano no se pierde al deduplicar.
# 4. Los group-headers no quedan vacios ni duplicados.
# 5. Un item no clasificable se CONSERVA y se declara (direccion del fallo).
# 6. La seccion destino puede ser la ULTIMA del changelog.
# 7. La huella clasifica igual un item y un subject de commit.
# 8. La disposicion se declara, y son cuatro y no se confunden.
#
# El punto 2 tiene su falsificador en
# `tests/test_changelog_merge_mutation.sh`, que quita el dedup y exige que
# este test caiga. Una asercion sin ver caer es decoracion.
#
# POR QUE NO HAY `A && ok || bad`
# ------------------------------
# El idiom `[ "$n" -eq 1 ] && ok "..." || bad "..."` dispara SC2015, y el
# gate del 1b (`tests/test_build_identity_policy.sh`) corre `shellcheck`
# SIN filtro de severidad sobre los `.sh` tocados, luego un aviso
# mataria el release. De ahi los helpers con `if`. Es INC-DEBT-073
# cobrando su primera vez en este mismo bloque.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/changelog_merge.sh
# shellcheck disable=SC1091
. "$ROOT/scripts/lib/changelog_merge.sh"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

# ── aserciones ─────────────────────────────────────────────────────────────
assert_eq() {  # assert_eq <obtenido> <esperado> <mensaje>
    if [ "$1" = "$2" ]; then
        ok "$3"
    else
        bad "$3 -- obtenido '$1', esperado '$2'"
    fi
}
assert_num() {  # assert_num <obtenido> <esperado> <mensaje>
    if [ "$1" -eq "$2" ]; then
        ok "$3"
    else
        bad "$3 -- obtenidos $1, esperados $2"
    fi
}
assert_has() {  # assert_has <fichero> <regex> <mensaje>
    if grep -qE "$2" "$1"; then
        ok "$3"
    else
        bad "$3 -- no aparece /$2/"
    fi
}
assert_empty() {  # assert_empty <valor> <mensaje>
    if [ -z "$1" ]; then
        ok "$2"
    else
        bad "$2 -- sobraba: $(printf '%s' "$1" | tr '\n' ' ')"
    fi
}

# ── helpers de fixture ─────────────────────────────────────────────────────
section_of() {  # section_of <fichero> <version> -> stdout
    awk -v h="## [$2]" '
        index($0, h) == 1 { inb = 1; next }
        inb && /^## \[/ { exit }
        inb { print }
    ' "$1"
}

SB="$(mktemp -d)" || { echo "FAIL: no se pudo crear el sandbox"; exit 1; }
# SC2329: `cleanup` solo se invoca desde el `trap` de abajo, que el analisis
# estatico no sigue.
# shellcheck disable=SC2329
cleanup() { rm -rf "$SB" >/dev/null 2>&1 || true; }
trap cleanup EXIT

# ── C1: el defecto real de INC-DEBT-074 ────────────────────────────────────
# Seccion escrita A MANO con su prosa + entrada autogenerada con los MISMOS
# commits en forma de subject pelado. Es la forma que produjo la 2.10.0 con
# las 6 entradas duplicadas.
CH="$SB/c1.md"
cat > "$CH" <<'EOF'
# Changelog

## [2.10.0] - 2026-10-05

### Features
  - feat(cli): el juez de frescura vive en el checkout, no en el artefacto — Cierra INC-DEBT-064, con su prosa larga.

### Other
  - test(cli): el guard del juez de frescura — siete relaciones
  - docs(cli): la regla de AGENTS.md 2.3.1

## [2.9.1] - 2026-10-04

### Fixes
  - fix(release): algo anterior
EOF

ENTRY="$SB/e1.md"
cat > "$ENTRY" <<'EOF'
## [2.10.0] - 2026-10-05

### Features
  - feat(cli): el juez de frescura vive en el checkout, no en el artefacto

### Other
  - docs(changelog): seccion escrita
  - docs(state): puntero reconciliado
  - test(cli): el guard del juez de frescura — siete relaciones
  - docs(cli): la regla de AGENTS.md 2.3.1
EOF

echo "=== C1: seccion pre-escrita + entrada autogenerada (el caso de 2.10.0) ==="
OUT="$(changelog_merge "$CH" 2.10.0 "$ENTRY" 2>"$SB/err1.txt")"
echo "  disposicion: $OUT"

# 1. estructura
assert_num "$(grep -cE '^## \[2\.10\.0\]' "$CH")" 1 "exactamente una cabecera 2.10.0"
assert_num "$(grep -cE '^## \[2\.9\.1\]' "$CH")" 1 "la cabecera 2.9.1 sigue ahi"
assert_has "$CH" 'fix\(release\): algo anterior' "el contenido de 2.9.1 sobrevive"

# 2. los items nuevos entran
assert_has "$CH" 'docs\(changelog\): seccion escrita' "item nuevo anadido"
assert_has "$CH" 'docs\(state\): puntero reconciliado' "segundo item nuevo anadido"

# 3. EL DEFECTO: nada se duplica
assert_num "$(grep -c 'el juez de frescura vive en el checkout' "$CH")" 1 \
    "el feat NO se duplico"
assert_num "$(grep -c 'el guard del juez de frescura' "$CH")" 1 \
    "el test NO se duplico"
assert_num "$(grep -c 'la regla de AGENTS.md 2.3.1' "$CH")" 1 \
    "el docs NO se duplico"

# 4. la prosa escrita a mano sobrevive al dedup
assert_has "$CH" 'Cierra INC-DEBT-064, con su prosa larga' \
    "la prosa escrita a mano sobrevive al dedup"

# 5. group-headers: ni vacios ni duplicados
EMPTY_G="$(section_of "$CH" 2.10.0 | awk '
  /^### /{ if (h != "" && n == 0) e++; h=$0; n=0; next }
  /^  - /{ if (h != "") n++ }
  END{ if (h != "" && n == 0) e++; print e+0 }')"
assert_num "$EMPTY_G" 0 "sin group-headers vacios"
DUP_G="$(section_of "$CH" 2.10.0 | awk '/^### /{ c[$0]++ } END{ for (k in c) if (c[k] > 1) print k }')"
assert_empty "$DUP_G" "sin group-headers duplicados"

# 6. el descarte se DECLARA
assert_has "$SB/err1.txt" 'ya declarado, se omite' \
    "los items omitidos se anuncian con su huella"
assert_num "$(grep -c 'ya declarado, se omite' "$SB/err1.txt")" 3 \
    "omitidos exactamente 3 items (los 3 ya representados)"

# 7. disposicion
assert_eq "$OUT" "disposition: merged" "disposicion 'merged' declarada"

# ── C2: todo ya declarado -> merged_nothing, y el fichero no se toca ──────
CH2="$SB/c2.md"
cp "$CH" "$CH2"
BEFORE="$(sha256sum < "$CH2" | cut -d' ' -f1)"
ENTRY2="$SB/e2.md"
printf '## [2.10.0] - 2026-10-05\n\n### Features\n  - feat(cli): el juez de frescura vive en el checkout, no en el artefacto\n' \
  > "$ENTRY2"

echo
echo "=== C2: todo ya declarado ==="
OUT2="$(changelog_merge "$CH2" 2.10.0 "$ENTRY2" 2>/dev/null)"
AFTER="$(sha256sum < "$CH2" | cut -d' ' -f1)"
assert_eq "$OUT2" "disposition: merged_nothing" "disposicion 'merged_nothing' declarada"
assert_eq "$BEFORE" "$AFTER" "el fichero no cambio byte a byte"

# ── C3: el caso original de session-30 (item nuevo en seccion existente) ──
CH3="$SB/c3.md"
cat > "$CH3" <<'EOF'
# Changelog

## [3.0.0] - 2026-10-05

### Fixes
  - fix(a): uno

## [2.1.0] - 2026-09-27

### Fixes
  - fix(b): dos
EOF
ENTRY3="$SB/e3.md"
printf '## [3.0.0] - 2026-10-05\n\n### Fixes\n  - fix(c): tres\n' > "$ENTRY3"
echo
echo "=== C3: item nuevo en seccion existente ==="
changelog_merge "$CH3" 3.0.0 "$ENTRY3" >/dev/null 2>&1
assert_has "$CH3" 'fix\(a\): uno' "el item preexistente sobrevive"
assert_has "$CH3" 'fix\(c\): tres' "el item nuevo se anade"
assert_has "$CH3" 'fix\(b\): dos' "la seccion siguiente sobrevive"
assert_num "$(grep -cE '^## \[3\.0\.0\]' "$CH3")" 1 "una sola cabecera 3.0.0"

# ── C4: la seccion destino es la ULTIMA del changelog ────────────────────
# MEDIDO: este caso encontro un bug real. El ensamblado terminaba en
# `[ "$after_line" -le "$total" ] && tail ...`; sin seccion siguiente la
# condicion es falsa, el grupo sale con 1 y el `|| { return 1; }` abortaba
# ANTES de escribir — el merge no hacia nada y no declaraba disposicion.
# Un fallo con apariencia de acierto, que solo salia cuando la seccion
# destino no tenia a nadie debajo.
CH4="$SB/c4.md"
printf '# Changelog\n\n## [4.0.0] - 2026-10-05\n\n### Fixes\n  - fix(a): uno\n' > "$CH4"
ENTRY4="$SB/e4.md"
printf '## [4.0.0] - 2026-10-05\n\n### Fixes\n  - fix(c): tres\n' > "$ENTRY4"
echo
echo "=== C4: la seccion destino es la ultima del changelog ==="
OUT4="$(changelog_merge "$CH4" 4.0.0 "$ENTRY4" 2>/dev/null)"
assert_eq "$OUT4" "disposition: merged" "el merge declara disposicion aunque no haya seccion siguiente"
assert_has "$CH4" 'fix\(a\): uno' "el item preexistente sobrevive"
assert_has "$CH4" 'fix\(c\): tres' "el item nuevo SE ANADE (antes no se anadia nada)"

# ── C5: item NO CLASIFICABLE en la ENTRADA se conserva ────────────────────
# La linea no parseable va en la ENTRADA, que es donde el merge decide. Si
# estuviera solo en la seccion no habria nada que decidir y el caso no
# mediria la direccion del fallo.
CH5="$SB/c5.md"
printf '# Changelog\n\n## [5.0.0] - 2026-10-05\n\n### Other\n  - fix(a): uno\n' > "$CH5"
ENTRY5="$SB/e5.md"
printf '## [5.0.0] - 2026-10-05\n\n### Other\n  - algo sin separador\n  - fix(x): otro\n' > "$ENTRY5"
echo
echo "=== C5: item no clasificable ==="
changelog_merge "$CH5" 5.0.0 "$ENTRY5" 2>"$SB/err5.txt" >/dev/null
assert_has "$CH5" 'algo sin separador' \
    "el item no parseable se CONSERVA (no se pierde contenido)"
assert_has "$SB/err5.txt" 'NO CLASIFICABLE' \
    "el item no clasificable se declara en vez de guardarse en silencio"

# ── C6 y C7: las otras dos disposiciones, que no se pueden confundir ──────
CH6="$SB/c6.md"
printf '# Changelog\n\n## [1.0.0] - 2026-01-01\n\n### Fixes\n  - fix(a): uno\n' > "$CH6"
ENTRY6="$SB/e6.md"
printf '## [3.0.0] - 2026-10-05\n\n### Fixes\n  - fix(n): nueva\n' > "$ENTRY6"
echo
echo "=== C6: la seccion no existe -> inserted ==="
OUT6="$(changelog_merge "$CH6" 3.0.0 "$ENTRY6" 2>/dev/null)"
assert_eq "$OUT6" "disposition: inserted" "disposicion 'inserted' declarada"
assert_num "$(grep -n '^## \[3\.0\.0\]' "$CH6" | head -1 | cut -d: -f1)" 3 \
    "la seccion nueva se inserta arriba"

CH7="$SB/c7.md"
printf '# Changelog\n' > "$CH7"
ENTRY7="$SB/e7.md"
printf '## [3.0.0] - 2026-10-05\n\n### Fixes\n  - fix(n): nueva\n' > "$ENTRY7"
echo
echo "=== C7: el changelog no tiene secciones -> appended ==="
OUT7="$(changelog_merge "$CH7" 3.0.0 "$ENTRY7" 2>/dev/null)"
assert_eq "$OUT7" "disposition: appended" "disposicion 'appended' declarada"
assert_has "$CH7" '^## \[3\.0\.0\]' "la seccion se anexa"

# ── C8: la huella clasifica igual un item y un subject de commit ──────────
# MEDIDO: quitando la vineta ANTES que los espacios, la clave salia
# "- fix(cli)" con la vineta pegada. El dedup seguia funcionando porque
# los dos lados del merge ses los dos con vineta — invisible desde
# dentro —, pero la huella ya no era la MISMA que calcula el gate 2b
# sobre el subject crudo.
echo
echo "=== C8: la huella es la MISMA para item y subject ==="
FP_ITEM="$(changelog_item_fingerprint '  - fix(cli): el juez de frescura vive aqui')"
FP_SUBJ="$(changelog_item_fingerprint 'fix(cli): el juez de frescura vive aqui')"
if [ -n "$FP_ITEM" ]; then
    ok "una linea de changelog es clasificable ($FP_ITEM)"
else
    bad "la linea de changelog NO es clasificable"
fi
assert_eq "$FP_ITEM" "$FP_SUBJ" "item y subject producen la misma huella"
if changelog_item_fingerprint 'algo sin separador' >/dev/null 2>&1; then
    bad "una linea sin ': ' deberia NO ser clasificable"
else
    ok "una linea sin ': ' se declara no clasificable"
fi

# ── C9: la huella es SUFICIENTEMENTE precisa ──────────────────────────────
# El otro lado de la deduplicacion. Un dedup que borra de mas no hace
# ruido: hace un artefacto que no declara un commit que si se publico.
# Aqui dos items de la entrada se parecen a los de la seccion y aun asi
# TIENEN que entrar, por dos motivos distintos:
#   - uno cambia de TIPO (fix -> test) con el payload identico: sin la
#     clave en la huella, `fix(a)` y `test(a)` colisionarian.
#   - uno coincide en las dos primeras palabras y diverge en la tercera:
#     una huella de 2 palabras lo borraria.
# Un dedup que se pasa de grueso falla aqui; uno que no deduplica falla
# en C1. Los dos extremos, y el medio es lo unico aceptable.
CH9="$SB/c9.md"
printf '# Changelog\n\n## [9.0.0] - 2026-10-05\n\n### Other\n  - fix(a): uno dos tres\n' > "$CH9"
ENTRY9="$SB/e9.md"
printf '## [9.0.0] - 2026-10-05\n\n### Other\n  - test(a): uno dos tres\n  - fix(a): uno dos cuatro\n' > "$ENTRY9"
echo
echo "=== C9: la huella no es demasiado gruesa ==="
changelog_merge "$CH9" 9.0.0 "$ENTRY9" >/dev/null 2>&1
assert_has "$CH9" 'test\(a\): uno dos tres' \
    "un item de OTRO TIPO con el mismo payload se anade (la clave esta en la huella)"
assert_has "$CH9" 'fix\(a\): uno dos cuatro' \
    "un item que coincide en 2 palabras y diverge en la 3 se anade (la huella mira 4)"

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — el merge se ejecuta desde el codigo real y no duplica."
    exit 0
fi
echo "RESULT: FAIL — $FAIL comprobacion(es)."
exit 1
