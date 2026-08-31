#!/usr/bin/env bash
# Validación viva de referencias. Reproducible.
# Uso: validate-references.sh research/references-to-check.jsonl
# Entrada: JSONL con {reference, type, claim} por línea.
# Salida: imprime JSONL con {reference, type, http_status, found_claim, status}.

set -uo pipefail
INPUT="${1:?uso: validate-references.sh <references.jsonl>}"

while IFS= read -r line; do
  [ -z "$line" ] && continue
  ref=$(echo "$line" | python3 -c "import sys,json; print(json.loads(sys.stdin.read())['reference'])" 2>/dev/null)
  type=$(echo "$line" | python3 -c "import sys,json; print(json.loads(sys.stdin.read()).get('type','url'))" 2>/dev/null)
  claim=$(echo "$line" | python3 -c "import sys,json; print(json.loads(sys.stdin.read()).get('claim',''))" 2>/dev/null)

  case "$type" in
    crate-version)
      # Verificar versión de crate en crates.io
      crate=$(echo "$ref" | grep -oE '^[a-z0-9_-]+')
      api=$(curl -s "https://crates.io/api/v1/crates/$crate")
      max_ver=$(echo "$api" | python3 -c "import sys,json; print(json.loads(sys.stdin.read())['crate']['max_version'])" 2>/dev/null)
      echo "{\"reference\":\"$ref\",\"type\":\"$type\",\"crates_io_max_version\":\"$max_ver\",\"verified_at\":\"$(date -I)\",\"status\":\"$( [ -n "$max_ver" ] && echo VALID || echo INVALID )\"}"
      ;;
    url|doi|docs.rs|github)
      # HTTP GET + comprobar claim en el body
      status_code=$(curl -sL -o /tmp/refbody -w "%{http_code}" "$ref" 2>/dev/null)
      if [ "$status_code" = "200" ]; then
        # Buscar término clave del claim (simplificado)
        found=$(grep -qi "$(echo "$claim" | head -c 40)" /tmp/refbody 2>/dev/null && echo true || echo false)
        echo "{\"reference\":\"$ref\",\"type\":\"$type\",\"http_status\":$status_code,\"found_claim\":$found,\"verified_at\":\"$(date -I)\",\"status\":\"VALID\"}"
      else
        echo "{\"reference\":\"$ref\",\"type\":\"$type\",\"http_status\":${status_code:-0},\"found_claim\":false,\"verified_at\":\"$(date -I)\",\"status\":\"ROTTED\"}"
      fi
      ;;
    *)
      echo "{\"reference\":\"$ref\",\"type\":\"$type\",\"verified_at\":\"$(date -I)\",\"status\":\"UNKNOWN_TYPE\"}"
      ;;
  esac
done < "$INPUT"
