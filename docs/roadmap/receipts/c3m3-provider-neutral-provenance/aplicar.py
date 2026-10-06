#!/usr/bin/env python3
"""Reemplazo ESTRICTO de una ocurrencia, para falsificacion de guards.

Por que existe y por que no es un `sed`: una mutacion que no se aplica es una
falsacion MUERTA, y el modo habitual de que eso pase sin que nadie lo note es
que el patron ya no casa con el fuente y el script dice "0 replacements",
 sale con 0, y el shell lo cuenta como exito.

Aqui el contrato es al reves:
  0  -> exactamente una ocurrencia reemplazada
  2  -> 0 ocurrencias (la mutacion NO se aplico)
  3  -> mas de una ocurrencia (el patron es ambiguo)

Y el falsador trata el 2 como SKIP, nunca como PASS.
"""
import sys


def main() -> int:
    if len(sys.argv) != 4:
        print("uso: aplicar.py <fichero> <viejo> <nuevo>", file=sys.stderr)
        return 64
    path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
    with open(path, encoding="utf-8") as fh:
        texto = fh.read()
    n = texto.count(old)
    if n == 0:
        print(f"NO APLICADA: 0 ocurrencias en {path}", file=sys.stderr)
        return 2
    if n > 1:
        print(f"NO APLICADA: {n} ocurrencias (patron ambiguo) en {path}", file=sys.stderr)
        return 3
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(texto.replace(old, new, 1))
    return 0


if __name__ == "__main__":
    sys.exit(main())