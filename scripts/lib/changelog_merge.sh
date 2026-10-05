# shellcheck shell=bash
# changelog_merge.sh — UNA implementacion del merge de CHANGELOG, que
# ejecutan tanto el release como su test.
#
# POR QUE ESTA EN UNA LIBRERIA Y NO DENTRO DEL SCRIPT
# -------------------------------------------------
# `tests/test_changelog_merge.sh` **pegaba literalmente** el bloque del
# merge de `release-bump.sh` ("copied verbatim from release-bump.sh").
# Consecuencia medida: cambiar el merge en el release **no movia el test**,
# luego el unico guard de ese bloque daba verde contra la copia antigua.
# Es la clase "una copia del codigo no vigila el codigo", y aqui es peor
# que en otros casos porque **el objeto exclusivo del guard es justamente
# ese bloque**: no hay otra asercion que lo cubra. (INC-DEBT-074)
#
# Aqui la funcion se EJECUTA desde los dos lados. El test no lee el merge,
# lo llama.
#
# EL DEFECTO QUE ESTA CORREGIDO
# -----------------------------
# El merge hacia `tail -n +2 "$ENTRY_FILE"` al final de la seccion
# existente **sin comparar** con lo que la seccion ya declaraba. Y el
# orden del flujo lo hace inevitable: el preflight pide HEAD =
# `chore(release): bump version`, luego la seccion del artefacto que se
# publica tiene que existir ANTES del bump. Escribirla antes es la norma.
#
# MEDIDO en 2.10.0: la seccion quedo con las 6 entradas escritas dos veces
# —las de la prosa y el bloque autogenerado con el subject pelado— y hubo
# que recortarlas a mano, que no es repetible.
#
# UN SOLO CONCEPTO, UNA SOLA REGLA
# --------------------------------
# "¿Es el mismo commit?" lo decide `changelog_item_fingerprint`, y la usa
# TAMBIEN el gate 2b (`tests/test_changelog_coverage.sh`). Dos copias de
# esa definicion divergen en cuanto se toca una, y la divergencia no se ve
# hasta que los datos la golpean. El merge y el gate piden cosas distintas
# de la MISMA clasificacion —el uno salta, el otro exige— y por eso
# compartir la regla no es compartir el veredicto.
#
# DIRECCION DEL FALLO
# -------------------
# Cuando un item no se puede clasificar, el merge lo **CONSERVA**. Un item
# duplicado en el changelog es ruido; un item perdido es un artefacto que
# no describe lo que publica, que es la propiedad que este repo defiende
# desde INC-DEBT-047. Ante la duda, el merge no descarta.
#
# LO QUE ESTA ESTRUCTURA NO HACE
# -------------------------------
# Reemite la seccion como una secuencia de grupos `### X` con sus
# miembros. Cualquier linea que caiga dentro de un grupo se conserva tal
# cual —no se descarta nada que no sea un item de la entrada—, y las
# lineas en blanco finales de cada grupo se recortan. Lo que NO se
# conserva es el texto que va ANTES del primer `### ` de la seccion
# (preambulo): el changelog de este repo no lo usa, y borrarlo en
# silencio seria justo el fallo que este fichero evita. Si algun dia
# aparece, el merge tiene que declararlo en vez de tragarselo.

# ── NOTA SOBRE LA LIMPIEZA DE TEMPORALES ───────────────────────────────────
# En este entorno `rm` es un shim que delega en `mavis-trash`, y ESA SALIDA
# va a STDOUT. En una funcion cuyo stdout es un contrato legible por
# maquina (`disposition: ...`), un `rm` sin redirigir lo corrompe: el
# llamante recibe lineas de sistema mezcladas con las suyas, y un
# `case "$out" in *merged_nothing*)` deja de casar. MEDIDO: sin esto, la
# disposicion salia precedida de "mavis-trash: moved to trash: ...".
#
# Por eso TODA limpieza aqui va con `>/dev/null 2>&1`. No es cosmetica:
# es la diferencia entre un stdout que es un contrato y uno que no lo es.
changelog_drop() {
    rm -f "$@" >/dev/null 2>&1 || true
}

# Normalizar: minusculas y espacios colapsados. MISMAS operaciones, y en el
# mismo orden, que usaba el gate 2b antes de extraerse aqui: un refactor
# que cambiase el resultado haria que el gate que ya pasa dejara de pasar
# por un motivo que nadie podria leer en el diff.
changelog_norm() {
    tr '[:upper:]' '[:lower:]' | tr -s '[:space:]' ' '
}

# changelog_item_fingerprint <line>
#
# Imprime "<type(scope)>|<4 primeras palabras del payload>" o devuelve 1
# si la linea no es un item parseable. Acepta tanto la forma de item del
# changelog ("  - fix(a): uno") como la de subject de commit
# ("fix(a): uno"), porque las dos tienen que clasificarse igual.
#
# Devolver 1 —y no imprimir nada— es lo que permite que el llamante
# decida la DIRECCION del fallo. El merge conserva lo no clasificable; el
# gate 2b lo cuenta como no cubierto, que es lo correcto: un commit cuyo
# subject no se puede partir no se ha demostrado que este en la seccion.
changelog_item_fingerprint() {
    local line="${1:-}"
    # El ORDEN importa y se rompio una vez. Una linea de changelog llega
    # como "  - fix(cli): ..." (espacios, vineta, espacios) y un subject de
    # commit como "fix(cli): ...". MEDIDO: quitando la vineta ANTES que los
    # espacios, `${line#- }` no casa con "  - fix..." y la clave salia
    # "- fix(cli)" con la vineta pegada. El dedup seguia funcionando porque
    # los dos lados del merge ses los dos con vineta — y por eso el defecto
    # era invisible desde dentro —, pero la huella ya no era la MISMA que
    # calcula el gate 2b sobre el subject crudo, luego la promesa de "una
    # sola regla para que es el mismo commit" era falsa.
    line="${line#"${line%%[![:space:]]*}"}"   # espacios iniciales
    line="${line#- }"                          # la vineta
    line="${line#"${line%%[![:space:]]*}"}"   # espacios tras la vineta

    # `%%:*` y `#*: ` parten por el PRIMER separador. Sin ": " no hay
    # payload y la linea no es un item: se declara no clasificable en vez
    # de inventar una huella.
    case "$line" in
        *": "*) ;;
        *) return 1 ;;
    esac

    local key payload fingerprint
    key="${line%%:*}"
    payload="$(printf '%s' "${line#*: }" | changelog_norm)"
    fingerprint="$(printf '%s' "$payload" | cut -d' ' -f1-4)"

    if [ -z "$key" ] || [ -z "$fingerprint" ]; then
        return 1
    fi
    printf '%s|%s' "$key" "$fingerprint"
}

# changelog_section_fingerprints <changelog> <version>
#
# Imprime una huella por linea, de los items que la seccion de <version>
# ya declara. La seccion va desde su cabecera hasta justo antes de la
# siguiente `## [`, o hasta EOF.
changelog_section_fingerprints() {
    local changelog="$1" version="$2"
    local section
    section="$(awk -v h="## [$version]" '
        index($0, h) == 1 { inb = 1; next }
        inb && /^## \[/ { exit }
        inb { print }
    ' "$changelog")"

    local line fp
    while IFS= read -r line; do
        case "$line" in
            "  - "*) ;;
            *) continue ;;
        esac
        if fp="$(changelog_item_fingerprint "$line")"; then
            printf '%s\n' "$fp"
        fi
    done <<< "$section"
}

# changelog_filter_entry <entry_file> <fingerprints_file>
#
# Escribe en stdout el bloque de entrada SIN la cabecera `## [`, con los
# items ya representados eliminados y los group-headers que se quedan sin
# items debajo eliminados tambien.
#
# Cada item descartado se anuncia por stderr CON SU HUELLA, porque una
# cuenta ("1 omitido") no dice cual se omite, y un descarte silencioso en
# un artefacto publicado es indistinguible de que no hubiera pasado.
changelog_filter_entry() {
    local entry_file="$1" fingerprints_file="$2"
    local line fp group="" pending=""

    # Que se impriman los items y que se anuncien los omitidos.
    #
    # OJO: aqui NO se decide si el grupo tiene items o no. Esa decision es
    # de `changelog_group_merge`, que es el que EMITE. MEDIDO: decidirla
    # tambien aqui era un autor duplicado de la misma propiedad, y una
    # mutacion que quitara solo esta comprobacion no hacia caer NINGUN
    # guard — el awk laCubria igual. Con un solo autor, la propiedad se
    # puede falsar. (AGENTS.md 2.7: una autoridad canonica por concepto.)
    flush_group() {
        if [ -n "$group" ]; then
            printf '%s\n' "$group"
            [ -n "$pending" ] && printf '%s' "$pending"
            printf '\n'
        fi
        group=""
        pending=""
    }

    local -a lines=()
    while IFS= read -r line; do
        lines+=("$line")
    done < "$entry_file"

    local i
    for i in "${!lines[@]}"; do
        line="${lines[$i]}"
        case "$line" in
            "  - "*)
                if fp="$(changelog_item_fingerprint "$line")"; then
                    if grep -qxF -- "$fp" "$fingerprints_file"; then
                        printf 'changelog_merge: ya declarado, se omite %s\n' "$fp" >&2
                        continue
                    fi
                else
                    printf 'changelog_merge: NO CLASIFICABLE, se conserva -> %s\n' \
                        "$(printf '%s' "$line" | cut -c1-60)" >&2
                fi
                pending+="$line"$'\n'
                ;;
            "### "*)
                flush_group
                group="$line"
                ;;
            *)
                # Lineas en blanco o de otro tipo: no son grupo ni item, y
                # no alteran la agrupacion.
                ;;
        esac
    done
    flush_group
}

# changelog_group_merge <existing_block_file> <incoming_block_file>
#
# Une dos bloques de grupos `### X` con sus items. Si el grupo entrante ya
# existe en el bloque destino, SUS ITEMS ENTRAN EN ESE GRUPO en vez de
# crear un segundo grupo con el mismo nombre.
#
# Esto NO es cosmetica. Sin el, una seccion pre-escrita a la que el bump
# le anade commits no representados sale con DOS cabeceras `### Other`: una
# con la prosa escrita a mano y otra con los items sueltos. Publicar eso
# parece un fallo del merge, y lo es — de la misma clase que el que se
# esta corrigiendo, un mismo concepto declarado dos veces.
#
# Formato de entrada: lineas `### X` abren grupo, todo lo demas hasta el
# siguiente `### X` es miembro del grupo. Las lineas en blanco finales de
# cada grupo se recortan; el resto de lineas se conservan tal cual.
changelog_group_merge() {
    local existing="$1" incoming="$2"
    awk '
    FNR == NR {
        if ($0 ~ /^### /) { g = ++ng; hdr[g] = $0; cnt[g] = 0 }
        else if (g > 0) { cnt[g]++; line[g, cnt[g]] = $0 }
        next
    }
    {
        if ($0 ~ /^### /) {
            g = ++ng
            hdr[g] = $0
            cnt[g] = 0
            for (j = 1; j < g; j++) {
                if (hdr[j] == hdr[g]) { g = j; break }
            }
        } else if (g > 0) {
            cnt[g]++
            line[g, cnt[g]] = $0
        }
    }
    END {
        for (j = 1; j <= ng; j++) {
            n = cnt[j]
            # Recorta las lineas en blanco del final del grupo.
            while (n > 0 && line[j, n] ~ /^[[:space:]]*$/) n--
            if (n == 0) continue
            printf "%s\n", hdr[j]
            for (m = 1; m <= n; m++) printf "%s\n", line[j, m]
            printf "\n"
        }
    }
    ' "$existing" "$incoming"
}

# changelog_merge <changelog> <version> <entry_file>
#
# Aplica <entry_file> a <changelog> con las CUATRO disposiciones
# posibles. Imprime en stdout una linea de disposicion y deja el detalle
# de lo omitido y de lo no clasificable en stderr. Devuelve 0 siempre que
# el fichero quede escrito, y 1 si no se pudo.
#
# Las cuatro se declaran y no se confunden, porque el llamante tiene que
# saber cual se tomo:
#   merged_nothing -> la seccion existia y TODOS los items ya estaban
#   merged         -> la seccion existia y se le anadieron items nuevos
#   inserted       -> la seccion no existia y se creo (nada que dedup)
#   appended       -> el changelog no tenia ninguna seccion
changelog_merge() {
    local changelog="$1" version="$2" entry_file="$3"
    local tmp fingerprints filtered existing_block

    if [ -z "$(grep -nE "^## \[$version\]" "$changelog" 2>/dev/null | head -1)" ]; then
        # Sin seccion previa no hay nada que comparar: se inserta o se
        # anexa entera. Este camino no puede duplicar, y por eso no se le
        # aplica el filtro.
        if grep -qE '^## \[' "$changelog" 2>/dev/null; then
            local first
            first="$(grep -n -m1 '^## \[' "$changelog" | cut -d: -f1)"
            tmp="$(mktemp)" || return 1
            {
                head -n "$((first - 1))" "$changelog"
                cat "$entry_file"
                tail -n "+$first" "$changelog"
            } > "$tmp" || { changelog_drop "$tmp"; return 1; }
            mv "$tmp" "$changelog" || { changelog_drop "$tmp"; return 1; }
            echo "disposition: inserted"
        else
            cat "$entry_file" >> "$changelog" 2>/dev/null || return 1
            echo "disposition: appended"
        fi
        return 0
    fi

    local exist_line after_line total
    exist_line="$(grep -nE "^## \[$version\]" "$changelog" | head -1 | cut -d: -f1)"
    after_line="$(grep -nE '^## \[' "$changelog" | awk -F: -v s="$exist_line" '$1 > s {print $1; exit}')"
    total="$(wc -l < "$changelog")"
    [ -z "$after_line" ] && after_line="$((total + 1))"

    tmp="$(mktemp)" || return 1
    fingerprints="$(mktemp)" || { changelog_drop "$tmp"; return 1; }
    filtered="$(mktemp)" || { changelog_drop "$tmp" "$fingerprints"; return 1; }
    existing_block="$(mktemp)" || {
        changelog_drop "$tmp" "$fingerprints" "$filtered"; return 1; }

    changelog_section_fingerprints "$changelog" "$version" > "$fingerprints"

    # El bloque de la seccion, sin su cabecera.
    awk -v s="$exist_line" -v e="$after_line" 'NR > s && NR < e' "$changelog" \
        > "$existing_block"

    if ! changelog_filter_entry "$entry_file" "$fingerprints" > "$filtered"; then
        changelog_drop "$tmp" "$fingerprints" "$filtered" "$existing_block"
        return 1
    fi

    local merged_body
    merged_body="$(mktemp)" || {
        changelog_drop "$tmp" "$fingerprints" "$filtered" "$existing_block"; return 1; }
    changelog_group_merge "$existing_block" "$filtered" > "$merged_body"

    # `if` y NO `[ ... ] && tail`. MEDIDO: con la forma `&&`, cuando la
    # seccion destino es la ULTIMA del changelog no hay seccion siguiente,
    # la condicion es falsa, el grupo entero sale con estado 1 y el
    # `|| { ...; return 1; }` de al lado aborta ANTES de escribir: el
    # merge no hacia nada y no declaraba disposicion. Un fallo que se
    # parece a un acierto, y que solo aparece cuando la seccion destino
    # no tiene a nadie debajo. El `if` con condicion falsa devuelve 0, que
    # es lo que el grupo debe devolver cuando todo ha ido bien.
    {
        head -n "$exist_line" "$changelog"
        echo
        cat "$merged_body"
        if [ "$after_line" -le "$total" ]; then
            tail -n "+$after_line" "$changelog"
        fi
    } > "$tmp" || {
        changelog_drop "$tmp" "$fingerprints" "$filtered" "$existing_block" "$merged_body"
        return 1
    }

    local disposition
    # La disposicion la decide si SOBREVIVIO ALGUN ITEM, no si el cuerpo
    # merged tiene contenido: el cuerpo merged SIEMPRE contiene los grupos
    # que ya estaban, luego preguntarle eso daria `merged` siempre y la
    # cuarta disposicion seria inalcanzable. MEDIDO antes de corregirlo.
    if grep -q '^  - ' "$filtered"; then
        disposition="merged"
    else
        disposition="merged_nothing"
    fi

    mv "$tmp" "$changelog" || {
        changelog_drop "$tmp" "$fingerprints" "$filtered" "$existing_block" "$merged_body"
        return 1
    }
    changelog_drop "$fingerprints" "$filtered" "$existing_block" "$merged_body"
    echo "disposition: $disposition"
    return 0
}
