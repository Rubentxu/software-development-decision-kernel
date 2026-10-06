#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Autofalsacion del guard de pureza del nucleo.
#
# ## QUE MIDE Y POR QUE NO BASTA CON EL GUARD
#
# `crates/sddk-domain/tests/kernel_purity_fitness.rs` tiene doce controles, y
# todos estan en verde sobre un corpus limpio. Eso es exactamente el estado en el
# que un guard puede estar desconectado sin que nadie lo note: si la ley se
# cumple, no hay nada que ver, y un guard que solo dice «no hay nada que ver» es
# indistinguible de uno que ha dejado de mirar.
#
# ## LA REGLA DE INTEGRIDAD, Y POR QUE NO SE PUEDE COPIAR TAL CUAL
#
# El falsador hermano de los diagnosticos de release mide sus mutaciones sobre
# un script de shell, y por eso puede exigir `bash -n` antes de juzgar: un parche
# que rompe la sintaxis hace caer los DIEZ casos, y eso no es una deteccion, es
# que el fichero no arranca. Aqui el objeto de la mutacion es un fichero Rust, y
# el equivalente de `bash -n` no existe como comprobacion suelta.
#
# El equivalente correcto es **el recuento de los otros controles**: si el parche
# hace caer el control que se le atribuye Y ademas deja pasar al resto, la
# deteccion es real y es de ese control. Si se caen todos, el parche era
# degenerado —no compila— y no se cuenta como nada. Un parche degenerado es
# `SKIP`, con su motivo, nunca `PASS`.
#
# Se aplica la misma regla de las dos formas: una mutacion que NO cambia el
# fichero es `SKIP` (no esta midiendo nada) y una mutacion que sobrevive es
# `FAIL` (el control no tiene dientes).
# ---------------------------------------------------------------------------
set -uo pipefail

cd "$(git rev-parse --show-toplevel)" || exit 1

GUARD="crates/sddk-domain/tests/kernel_purity_fitness.rs"
OUT="$(mktemp)"
BACKUP="$(mktemp)"
trap 'cp "$BACKUP" "$GUARD" 2>/dev/null; rm -f "$OUT" "$BACKUP"' EXIT

PASS=0
FAIL=0
SKIP=0

cp "$GUARD" "$BACKUP"

# Cuenta como ha quedado cada control: `<ok> <nombre>` o `<ko> <nombre>`.
snapshot() {
    cargo test -p sddk-domain --test kernel_purity_fitness --no-fail-fast > "$OUT" 2>&1
    : > "$OUT.controls"
    while read -r estado nombre; do
        [ -n "${nombre:-}" ] && printf '%s %s\n' "$estado" "$nombre" >> "$OUT.controls"
    done < <(sed -nE 's/^test ([a-z0-9_]+) \.\.\. (ok|FAILED)$/\2 \1/p' "$OUT")
}

control_estado() {
    awk -v n="$1" '$2 == n { print $1 }' "$OUT.controls"
}

# `grep -c` devuelve 1 cuando cuenta cero, y un `|| echo 0` al final anade OTRO
# cero: la funcion devolvia dos lineas y `[ "0
# 0" -lt 12 ]` no es una comparacion entera, luego el abortion de la base rota
# NUNCA se disparo. MEDIDO: una base con cero controles en pie dejo correr las once
# mutaciones y las declaro todas supervivientes, que es exactamente lo contrario
# de lo que decia. Un contador que no cuenta es peor que no tener contador.
total_ok() {
    awk '$1 == "ok" { n++ } END { print n + 0 }' "$OUT.controls"
}

mutar() {
    local etiqueta="$1" esperado="$2" que="$3" pycode="$4"
    local sha_antes sha_mut sha_restore ok_otros ko_esperado ko_total

    sha_antes="$(sha256sum "$GUARD" | cut -d' ' -f1)"
    cp "$BACKUP" "$GUARD"

    if ! MUT_FILE="$GUARD" python3 -c "$pycode" 2>/dev/null; then
        printf '  [SKIP] %s — el parche lanzo error\n' "$etiqueta"
        SKIP=$((SKIP + 1))
        return
    fi
    sha_mut="$(sha256sum "$GUARD" | cut -d' ' -f1)"
    if [ "$sha_antes" = "$sha_mut" ]; then
        printf '  [SKIP] %s — el parche NO cambio el fichero: no esta midiendo nada\n' "$etiqueta"
        SKIP=$((SKIP + 1))
        return
    fi

    snapshot
    ok_otros="$(total_ok)"
    ko_esperado="$(control_estado "$esperado")"
    ko_total="$(awk '$1 == "FAILED" { n++ } END { print n + 0 }' "$OUT.controls")"

    # restaurar ANTES de juzgar, para que un fallo del falsador no deje el arbol
    # mutado.
    cp "$BACKUP" "$GUARD"
    sha_restore="$(sha256sum "$GUARD" | cut -d' ' -f1)"
    if [ "$sha_restore" != "$sha_antes" ]; then
        printf '  [FATAL] %s — la restauracion no fue byte-identica\n' "$etiqueta"
        exit 1
    fi

    # INTEGRIDAD DEL INSTRUMENTO, y aqui hace falta una distincion mas que en el
    # falsador hermano. Si el parche rompe la compilacion, el control **no
    # aparece** en el informe: no sale ni `ok` ni `FAILED`. MEDIDO, y no fue
    # hipotetico: la primera version de M11 inserto un `.filter()` dentro de un
    # closure, que no compila, y el falsador lo conto como "el control SIGUE
    # VERDE" — declarando una mutacion sana como si el guard fosse ciego, que es
    # justo la mentira que este falsador existe para no decir.
    if [ -z "$ko_esperado" ]; then
        printf '  [SKIP] %s — el control %s no aparece en el informe: el parche rompio el \
fichero y no se ejecuto nada. No es una deteccion ni una supervivencia\n' "$etiqueta" "$esperado"
        SKIP=$((SKIP + 1))
        return
    fi
    if [ "$ko_esperado" = "ok" ]; then
        printf '  [FAIL] %s — %s SIGUE VERDE. %s\n' "$etiqueta" "$esperado" "$que"
        FAIL=$((FAIL + 1))
        return
    fi
    # Integridad: el control caido tiene que ser el suyo, no un efecto colateral
    # de un fichero que ya no compila.
    if [ "$ok_otros" -lt 8 ]; then
        printf '  [SKIP] %s — el control cayo pero solo quedaron %s controles en pie: el parche \
rompio el fichero, mide que no arranca y no la propiedad\n' "$etiqueta" "$ok_otros"
        SKIP=$((SKIP + 1))
        return
    fi
    printf '  [ok]   %s — cae %s (%s control(es) siguen en pie, %s caido(s) en total). %s\n' \
        "$etiqueta" "$esperado" "$ok_otros" "$ko_total" "$que"
    PASS=$((PASS + 1))
}

printf '== autofalsacion del guard de pureza del nucleo ==\n'
snapshot
printf '  base: %s control(es) en pie\n' "$(total_ok)"
if [ "$(total_ok)" -lt 12 ]; then
    printf 'RESULT: ABORTADO — la base no esta verde, y un falsador sobre una base rota no \
mide nada.\n'
    exit 1
fi
printf '\n'

mutar "M1 el guard deja de normalizar la caja" un_nombre_con_mayuscula_tambien_infringe \
    "sin normalizar la caja, Cargo.toml deja de infraccionar y el guard pasa en verde sobre el fichero exacto que tiene que vigilar." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "let haystack = source.to_lowercase();"
nuevo = "let haystack = source.to_string();"
assert viejo in s, "no hay normalizacion de caja donde esta mutacion la supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

mutar "M2 ninguna zona se exime" la_zona_de_test_no_infringe_y_prueba_el_rechazo \
    "si no se exime nada, el guard marca los 41 sitios legitimos del corpus y entrena a su lector a ignorarlo." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "fn test_zones(source: &str) -> Result<Vec<Zone>, String> {"
nuevo = "fn test_zones(source: &str) -> Result<Vec<Zone>, String> {\n    if !source.is_empty() { return Ok(Vec::new()); }"
assert viejo in s, "no hay zona declarada donde esta mutacion la supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

mutar "M3 la zona se busca sin mirar si el item tiene cuerpo" un_cfg_test_sobre_un_use_no_exime_lo_que_viene_despues \
    "un cfg(test) sobre un use no abre modulo: sin mirar el terminador, el guard adopta la llave de la primera funcion de produccion posterior y exime su cuerpo entero. Eso es ceguera, no ruido." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "                b\x27{\x27 | b\x27;\x27 | b\x27}\x27 => {"
nuevo = "                b\x27{\x27 => {"
assert viejo in s, "el terminador de item no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

mutar "M4 las cadenas crudas dejan de contar como cadenas" una_cadena_cruda_con_llaves_no_cierra_la_zona \
    "una cadena cruda con un numero impar de comillas, si no se enmascara, deja una cadena abierta que se come el resto del fichero: la zona nunca cierra y el guard se queda ciego." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "                && bytes[j] == b\x27\"\x27\n"
nuevo = "                && false\n                && bytes[j] == b\x27\"\x27\n"
assert viejo in s, "la rama de cadena cruda no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

mutar "M5 las cadenas normales dejan de contar como cadenas" una_cadena_normal_y_un_literal_de_caracter_no_cuentan_como_llaves \
    "379 lineas del corpus tienen llaves dentro de cadenas. Sin enmascararlas, el conteo se desincroniza a la primera." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "        if b == b\x27\"\x27 {\n            let mut j = i + 1;"
nuevo = "        if b == b\x27\"\x27 && false {\n            let mut j = i + 1;"
assert viejo in s, "la rama de cadena normal no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

mutar "M6 los literales de caracter dejan de contar como literales" una_cadena_normal_y_un_literal_de_caracter_no_cuentan_como_llaves \
    "un literal de una sola llave es una llave de verdad para un conteo ingenuo, y desplaza todo el resto." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "        if b == b\x27\\\x27\x27\n            && let Some(len) = char_literal_len(bytes, i)"
nuevo = "        if b == b\x27\\\x27\x27 && false\n            && let Some(len) = char_literal_len(bytes, i)"
assert viejo in s, "la rama de literal de caracter no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

mutar "M7 los comentarios de bloque dejan de contar como comentarios" un_comentario_con_llaves_no_cierra_la_zona \
    "un comentario de bloque con llaves cierra la zona antes de tiempo, y lo que viene despues queda sin vigilar." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "        if b == b\x27/\x27 && i + 1 < bytes.len() && bytes[i + 1] == b\x27*\x27 {"
nuevo = "        if b == b\x27/\x27 && i + 1 < bytes.len() && bytes[i + 1] == b\x27*\x27 && false {"
assert viejo in s, "la rama de comentario de bloque no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

mutar "M8 una zona que no cierra se presume limpia" el_guard_falla_cerrado_ante_una_zona_indeterminable \
    "eximir lo que no se ha podido leer entero no es ley, es no vigilar: una llave sin cerrar declararia medio fichero limpio." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "        if depth != 0 {\n            return Err(format!("
nuevo = "        if depth != 0 {\n            cursor = i + 1;\n            continue;\n        }\n        if false {\n            return Err(format!("
assert viejo in s, "el fallo cerrado no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

mutar "M9 el limite de palabra vuelve a ser una subcadena" el_escaner_de_palabras_no_es_una_subcadena \
    "sin limite de palabra, la palabra rust aparece dentro de trusted y la palabra pip dentro de pipeline: los tres rojos falsos medidos que hicieron que este escaner existiera." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);"
nuevo = "        let before_ok = true;"
assert viejo in s, "el limite izquierdo no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
viejo2 = "        let after_ok = after == bytes.len() || !is_word_byte(bytes[after]);"
nuevo2 = "        let after_ok = true;"
assert viejo2 in s, "el limite derecho no tiene la forma que esta mutacion supone"
s = s.replace(viejo2, nuevo2, 1)
open(p, "w").write(s)
'

mutar "M10 el vocabulario se encoge" el_vocabulario_no_se_ha_vaciado \
    "un hits vacio puede pasar porque la lista se encogio, no porque el nucleo este limpio. Se trunca en vez de vaciarse porque vaciarla rompe ademas todos los controles que indexan la lista, y ahi lo que cae no es una deteccion sino un efecto colateral." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "const FORBIDDEN: &[&str] = &["
idx = s.index(viejo)
cuerpo = s.index("];", idx)
entradas = s[idx + len(viejo):cuerpo].split("\n")
# Se conservan las cinco primeras entradas, que son las que los controles usan
# por indice, y se descarta el resto.
nuevo_cuerpo = "\n".join(entradas[:6])
s = s[:idx + len(viejo)] + nuevo_cuerpo + s[cuerpo:]
open(p, "w").write(s)
'

mutar "M11 el guard vuelve a mirar un solo modulo" el_guard_cubre_todos_los_modulos_del_dominio \
    "la extension a 46 modulos podria seguir siendo un include_str de uno solo, y todos los demas controles pasarian igual porque operan sobre cadenas sinteticas." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = """    let files = fs::read_dir(domain_src())"""
nuevo = """    let mut files = fs::read_dir(domain_src())"""
assert viejo in s, "el recuento de ficheros no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
viejo2 = """        .count();
    assert!(
        files >= 46,"""
nuevo2 = """        .count();
    files = 0;
    assert!(
        files >= 46,"""
assert viejo2 in s, "el cierre del recuento no tiene la forma que esta mutacion supone"
s = s.replace(viejo2, nuevo2, 1)
open(p, "w").write(s)
'

# --- resumen -----------------------------------------------------------------

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d SKIP=%d\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" -eq 0 ] && [ "$SKIP" -eq 0 ]; then
    printf 'RESULT: PASS — las %d comprobaciones tienen dientes, cada una por la suya.\n' "$PASS"
    exit 0
fi
if [ "$FAIL" -ne 0 ]; then
    printf 'RESULT: FAIL — hay comprobaciones que no vigilan lo que declaran.\n'
    exit 1
fi
printf 'RESULT: DECLARADO — %s mutacion(es) no se aplicaron y no cuentan como deteccion.\n' "$SKIP"
exit 1