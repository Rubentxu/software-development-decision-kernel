#!/usr/bin/env python3
"""Informe: los alias de proyecto ocultan historia repartida (INC-DEBT-061).

Cada alias `from_id -> to_id` afirma en su campo `reason` que el receipt y el
histórico están en el `to_id`. Eso es cierto cuando la historia se *movió*. Es
falso cuando se *repartió*, y la diferencia es la que este script mide: una
redirección puede renombrar, no puede unir.

NO modifica nada. Lee el storage real y sale distinto de cero cuando algún
alias aporta historia en el lado que aparta, para que quien decida tenga el
número delante y no un recuerdo.

Deliberadamente NO es un gate de `release.sh`: escribir en la tabla de alias
desde un gate de release sería el mismo defecto que la incidencia describe, y
además el gate tendría que conocer el estado de una máquina concreta.
"""
from __future__ import annotations

import json
import sqlite3
import sys
from pathlib import Path

# Las cinco tablas que el storage declara append-only, más `cycles`, que es lo
# que un usuario llama «trabajo» cuando dice que un proyecto tiene historia.
APPEND_ONLY = (
    "events_v1",
    "attempts_v1",
    "workflow_runs_v1",
    "node_runs_v1",
    "workflow_run_events_v1",
    "backlog_item_events_v1",
)


def state_base() -> Path:
    """`SDDK_STATE_HOME` manda sobre `XDG_STATE_HOME`, igual que el runtime.

    Mismo orden que `sddk_engine::state_base` (INC-DEBT-037). Si este script
    mirara un sitio distinto del que mira el binario, estaría midiendo otra
    máquina sin decir nada — que es el fallo que ya corrigió `roots()` en
    `migrate_project_identity.py`.
    """
    import os

    return Path(
        os.environ.get("SDDK_STATE_HOME")
        or os.environ.get("XDG_STATE_HOME")
        or Path.home() / ".local/state"
    ) / "sddk"


def side_rows(project_id: str, projects: Path) -> dict[str, int]:
    """Filas de un lado. Ausente = 0, y se distingue de 0 leido.

    Un ledger inexistente y un ledger vacío son estados distintos: el primero
    es una cáscara de re-adopción, el segundo es un proyecto sin trabajo. La
    tabla de alias delega la diferencia al que lee, y este informe la nombra.

    La forma del dict es SIEMPRE la misma —`_ledger` más un contador por tabla
    más `events` y `cycles`—, incluso cuando no hay ledger. La primera versión
    devolvía `{_ledger: -1}` en ese caso y el informe reventaba con `KeyError`
    justo en la fila más importante: la que no tiene nada que enseñar.
    """
    empty: dict[str, int] = {"_ledger": -1, "events": 0, "cycles": 0}
    for table in APPEND_ONLY:
        empty[table] = 0

    path = projects / project_id / "ledger.sqlite"
    if not path.exists():
        return empty
    try:
        con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    except sqlite3.Error:
        return empty
    try:
        have = {r[0] for r in con.execute(
            "select name from sqlite_master where type='table'"
        )}
        out = dict(empty)
        out["_ledger"] = 1
        for table in APPEND_ONLY:
            if table in have:
                out[table] = con.execute(f'select count(*) from "{table}"').fetchone()[0]
        out["events"] = out.get("events_v1", 0)
        out["cycles"] = (
            con.execute(
                "select count(*) from cycles where project_id = ?", (project_id,)
            ).fetchone()[0]
            if "cycles" in have
            else 0
        )
        return out
    finally:
        con.close()


def main() -> int:
    base = state_base()
    table_path = base / "project-aliases.json"
    projects = base / "projects"

    if not table_path.exists():
        print(f"no hay tabla de alias en {table_path}")
        print("  una maquina que nunca declaro un alias es el caso normal, no un fallo")
        return 0

    try:
        entries = json.loads(table_path.read_text(encoding="utf-8"))["aliases"]
    except (OSError, ValueError, KeyError) as exc:
        # Fail-closed. Un fichero ilegible NO es una tabla vacía: sin esto el
        # informe diría «ningún alias esconde historia» precisamente porque no
        # pudo leer ninguno.
        print(f"NO SE PUEDE LEER LA TABLA DE ALIASES: {exc}", file=sys.stderr)
        print("  esto NO es lo mismo que 'no hay aliases'.", file=sys.stderr)
        return 2

    print(f"tabla: {table_path}")
    print(f"alias declaradas: {len(entries)}")
    print()
    print(f"{'from_id':<20} {'to_id':<20} {'ev_from':>8} {'cy_from':>8} {'ev_to':>8}  veredicto")
    print("-" * 92)

    hiding = []
    for entry in entries:
        src, dst = entry["from_id"], entry["to_id"]
        a = side_rows(src, projects)
        b = side_rows(dst, projects)
        if a["_ledger"] < 0:
            verdict = "sin ledger en el lado apartado (cascara de re-adopcion)"
        elif a["events"] > 0 or a["cycles"] > 0:
            verdict = "APARTA HISTORIA — el alias no puede renombrar un reparto"
            hiding.append((src, dst, a, b))
        else:
            verdict = "limpio: la historia esta entera en el destino"
        print(f"{src:<20} {dst:<20} {a['events']:>8} {a['cycles']:>8} "
              f"{b.get('events', 0):>8}  {verdict}")

    print()
    if not hiding:
        print("ningun alias aparta historia: la tabla afirma lo que hace")
        return 0

    total_ev = sum(a["events"] for _, _, a, _ in hiding)
    total_cy = sum(a["cycles"] for _, _, a, _ in hiding)
    print(f"ALIASES QUE APARTAN HISTORIA: {len(hiding)} de {len(entries)}")
    print(f"  {total_ev} eventos y {total_cy} ciclos quedan fuera de toda ruta")
    print("  canonica de lectura. Un pin no los salva: el alias se aplica")
    print("  DESPUES del pin, y ese es justo el input que existe para corregir.")
    print()
    print("El campo `reason` de estos alias dice que el historico esta en el `to_id`.")
    print("Comprueba si el storage lo confirma antes de citarlo:")
    for src, dst, a, _ in hiding:
        entry = next(e for e in entries if e["from_id"] == src)
        claims = "historico estan en" in entry.get("reason", "")
        print(f"  {src} -> {dst}: {a['events']} ev, {a['cycles']} ciclos · "
              f"el reason {'AFIRMA' if claims else 'no afirma'} que la historia esta en el to")
    return 1


if __name__ == "__main__":
    sys.exit(main())
