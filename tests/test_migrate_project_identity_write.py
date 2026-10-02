#!/usr/bin/env python3
"""Contrato de la ESCRITURA de `scripts/migrate_project_identity.py`.

POR QUE EXISTE (session-66)
---------------------------
`test_migrate_project_identity_mirror.py` fija la DERIVACION: que el espejo en
Python reproduzca el `project_id` del Rust. No dice nada de la escritura. Y la
escritura tenia tres defectos que ninguno de los guardas existentes detectaba,
porque los tres vivian en el camino que solo se ejecuta al final, contra el
storage real, y ese camino no se puede ejercitar sin escribir de verdad:

1. `UPDATE` SIN `WHERE`. `sqlite_impact` contaba las filas que CASABAN con el
   id viejo; la escritura se llevaba la TABLA ENTERA. En los ledgers de esta
   maquina eso no era hipotetico:
     - `p-63676b11dc0ef88f` tiene 79 ciclos centinela `__spine_import__`
     - `p-7c4aff45199a2069` tiene 10 ciclos de OTRO proyecto
       (`p-490921be0aac9b69`), con su historia de eventos y gate receipts
   Un `UPDATE` sin `WHERE` habria reasignado la identidad de los dos, y el plan
   -que es lo que el operador revisa- no lo mencionaba, porque para el plan
   esas filas no existen.

2. El directorio de estado NO se renombraba. El codigo hacia
   `dst = src.replace(old, new)` y abria `sqlite3.connect(dst)`: sobre un
   directorio inexistente fallaba, o sobre una base recien creada y vacia
   perdia el ledger entero. El plan solo declaraba `share_dirs_to_rename`.

3. Solo se comprobaba que el destino de share no existiera; el de estado no.

LA PROPIEDAD
------------
Las filas escritas son EXACTAMENTE las filas que el plan conto. No es una
revision: `owned_predicate` construye el predicado y lo usan tanto el recuento
como la escritura, asi que no pueden divergir. Y nada que no sea del proyecto se
toca: ni los centinelas, ni las filas de otro proyecto que conviven en el mismo
ledger.

Todos los fixtures son temporales. Este test no lee ni escribe el storage real.
"""

from __future__ import annotations

import importlib.util
import sqlite3
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "migrate_project_identity.py"

OLD = "p-0ld0000000000001"
NEW = "p-new000000000001"
FOREIGN = "p-f0reign00000001"
SENTINEL = "__spine_import__"


def load():
    spec = importlib.util.spec_from_file_location("migrate_project_identity_write", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


DDL = """
CREATE TABLE projects      (project_id TEXT, workspace_id TEXT);
CREATE TABLE workspaces    (project_id TEXT, workspace_id TEXT);
CREATE TABLE cycles        (cycle_id TEXT, project_id TEXT, status TEXT);
CREATE TABLE events_v1     (cycle_id TEXT, project_id TEXT, event_type TEXT);
CREATE TABLE gate_receipts (cycle_id TEXT, project_id TEXT, verdict TEXT);
"""


def make_fixture(root: Path):
    """Storage temporal con los TRES tipos de fila que conviven en un ledger real.

    - propias    -> deben reescribirse
    - centinela  -> `__spine_import__`: no es un project_id, no se toca
    - ajena     -> otro proyecto en el mismo ledger: no se toca
    """
    share = root / "share"
    state = root / "state"
    old_share = share / "projects" / OLD
    old_state = state / "projects" / OLD
    old_share.mkdir(parents=True)
    old_state.mkdir(parents=True)

    db = old_state / "ledger.sqlite"
    conn = sqlite3.connect(db)
    conn.executescript(DDL)
    conn.executemany("INSERT INTO projects VALUES (?,?)",
                     [(OLD, "ws-own"), (SENTINEL, "ws-sentinel"), (FOREIGN, "ws-foreign")])
    conn.executemany("INSERT INTO workspaces VALUES (?,?)",
                     [(OLD, "ws-own"), (SENTINEL, "ws-sentinel"), (FOREIGN, "ws-foreign")])
    conn.executemany("INSERT INTO cycles VALUES (?,?,?)", [
        (f"{OLD}/m1", OLD, "CLOSED"),
        (f"{OLD}/m2", OLD, "OPEN"),
        ("SENT-CYCLE", SENTINEL, "OPEN"),
        (f"{FOREIGN}/m9", FOREIGN, "CLOSED"),
    ])
    conn.executemany("INSERT INTO events_v1 VALUES (?,?,?)", [
        (f"{OLD}/m1", OLD, "workflow.phase.entered"),
        (f"{OLD}/m2", OLD, "workflow.phase.exited"),
        ("SENT-CYCLE", SENTINEL, "workflow.phase.entered"),
        (f"{FOREIGN}/m9", FOREIGN, "workflow.transition.succeeded"),
    ])
    conn.executemany("INSERT INTO gate_receipts VALUES (?,?,?)", [
        (f"{OLD}/m1", OLD, "PASS"),
        ("SENT-CYCLE", SENTINEL, "PASS"),
        (f"{FOREIGN}/m9", FOREIGN, "PASS"),
    ])
    conn.commit()
    conn.close()
    return share, state, db


def build_migration(module, share: Path, state: Path, db: Path):
    return {
        "old_project_id": OLD,
        "new_project_id": NEW,
        "remotes": ["git@example.test:o/r.git"],
        "receipts": [str(share / "projects" / OLD)],
        "share_dirs_to_rename": [str(share / "projects" / OLD)],
        "state_dirs_to_rename": [str(state / "projects" / OLD)],
        "state_dbs": [{
            "path": str(db),
            "impact": module.sqlite_impact(db, OLD),
            "foreign": module.foreign_values(db, OLD),
        }],
    }


def rows(db_path: Path, sql: str, params: tuple = ()) -> list:
    conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    try:
        return conn.execute(sql, params).fetchall()
    finally:
        conn.close()


class WriteContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.m = load()

    def setUp(self) -> None:
        # Cada test parte de un fixture NUEVO y decide cuando aplicarlo. Un
        # setUp que aplicase la migracion obligaria a estos tests a deshacer
        # una escritura para poder observar el estado previo.
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.share, self.state, self.db = make_fixture(self.root)
        self.migration = build_migration(self.m, self.share, self.state, self.db)
        self.silent = lambda s: None
        self.new_db = Path(str(self.db).replace(OLD, NEW))

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def apply(self, migration=None):
        return self.m.migrate_one(migration or self.migration, out=self.silent)

    # ── lo que el apply TIENE que hacer ──────────────────────────────

    def test_own_rows_are_rewritten(self) -> None:
        """Las filas propias pasan de OLD a NEW conservando el sufijo."""
        self.assertEqual(self.apply(), [])
        self.assertEqual(
            sorted(r[0] for r in rows(self.new_db, "SELECT cycle_id FROM cycles")),
            sorted([f"{NEW}/m1", f"{NEW}/m2", "SENT-CYCLE", f"{FOREIGN}/m9"]),
        )
        self.assertEqual(
            sorted(rows(self.new_db, "SELECT project_id, workspace_id FROM projects")),
            sorted([
                (NEW, "ws-own"),
                (SENTINEL, "ws-sentinel"),
                (FOREIGN, "ws-foreign"),
            ]),
        )

    def test_state_directory_is_renamed_and_ledger_survives(self) -> None:
        """El ledger se muda de directorio; no se pierde ni se duplica.

        REGRESION del defecto 2: el codigo abria `sqlite3.connect(dst)` sin
        haber renombrado el directorio. Sobre un directorio inexistente eso
        falla; si el destino existiera, creaba una base VACIA y el ledger real
        se quedaba atras, con su historia, sin copia.
        """
        self.assertEqual(self.apply(), [])
        self.assertFalse((self.state / "projects" / OLD).exists())
        self.assertTrue((self.state / "projects" / NEW).is_dir())
        self.assertTrue(self.new_db.exists())
        # Contenido real, no una base recien creada.
        self.assertEqual(rows(self.new_db, "SELECT COUNT(*) FROM events_v1")[0][0], 4)
        self.assertEqual(rows(self.new_db, "SELECT COUNT(*) FROM gate_receipts")[0][0], 3)

    # ── lo que el apply NO tiene que hacer (las regresiones) ─────────

    def test_sentinel_rows_survive(self) -> None:
        """`__spine_import__` no es un project_id: no se reescribe.

        REGRESION del defecto 1. En el ledger real de este repo son 79 ciclos.
        """
        self.assertEqual(self.apply(), [])
        for table, where in (
            ("cycles", "cycle_id='SENT-CYCLE'"),
            ("events_v1", "cycle_id='SENT-CYCLE'"),
            ("gate_receipts", "cycle_id='SENT-CYCLE'"),
        ):
            self.assertEqual(
                rows(self.new_db, f"SELECT project_id FROM {table} WHERE {where}"),
                [(SENTINEL,)],
                f"{table}: el centinela no puede adoptarse como project_id real",
            )
        self.assertEqual(
            rows(self.new_db, "SELECT project_id, workspace_id FROM projects WHERE project_id=?", (SENTINEL,)),
            [(SENTINEL, "ws-sentinel")],
        )

    def test_foreign_project_rows_survive(self) -> None:
        """Un ledger puede convivir con otro proyecto: no se reescribe.

        REGRESION del defecto 1. En el storage real, el ledger de
        `p-7c4aff45199a2069` contiene 10 ciclos de `p-490921be0aac9b69`.
        """
        self.assertEqual(self.apply(), [])
        self.assertEqual(
            rows(self.new_db, "SELECT project_id,cycle_id FROM cycles WHERE cycle_id LIKE ?",
                 (f"{FOREIGN}/%",)),
            [(FOREIGN, f"{FOREIGN}/m9")],
        )
        for table in ("events_v1", "gate_receipts"):
            self.assertEqual(
                rows(self.new_db, f"SELECT project_id FROM {table} WHERE cycle_id LIKE ?",
                     (f"{FOREIGN}/%",)),
                [(FOREIGN,)],
                f"{table}: la historia de {FOREIGN} no puede cambiar de identidad",
            )
        self.assertEqual(
            rows(self.new_db, "SELECT project_id, workspace_id FROM workspaces WHERE project_id=?", (FOREIGN,)),
            [(FOREIGN, "ws-foreign")],
        )

    def test_foreign_values_are_reported_not_guessed(self) -> None:
        """Lo que no se toca se DECLARA, para que el plan no vuelva a ocultarlo."""
        foreign = self.m.foreign_values(self.db, OLD)
        self.assertIn(f"cycles.project_id={SENTINEL}", foreign)
        self.assertIn(f"cycles.project_id={FOREIGN}", foreign)
        self.assertIn(f"projects.project_id={FOREIGN}", foreign)
        # El filtro no incluye lo que SI es nuestro.
        self.assertNotIn(f"cycles.project_id={OLD}", foreign)

    # ── fail-closed ─────────────────────────────────────────────────

    def test_aborts_when_state_dir_is_missing(self) -> None:
        """Si el directorio no esta, el plan esta obsoleto: no se escribe."""
        (self.state / "projects" / OLD).rename(self.state / "projects" / "p-otro")
        problems = self.apply()
        self.assertTrue(problems)
        self.assertIn("obsoleto", problems[0])

    def test_aborts_when_state_destination_already_exists(self) -> None:
        """No se fusiona nunca: dos ledgers con la misma identidad no se mezclan."""
        (self.state / "projects" / NEW).mkdir()
        problems = self.apply()
        self.assertTrue(problems)
        self.assertIn("no se fusiona", problems[0])

    def test_aborts_when_share_destination_already_exists(self) -> None:
        """Lo mismo para share: el receipt de otro proyecto no se sobrescribe."""
        (self.share / "projects" / NEW).mkdir()
        problems = self.apply()
        self.assertTrue(problems)
        self.assertIn("no se fusiona", problems[0])
        # Y el estado tampoco se toco: share se comprueba antes.
        self.assertTrue((self.state / "projects" / OLD).is_dir())

    def test_count_mismatch_aborts_and_leaves_the_ledger_intact(self) -> None:
        """Si se escriben mas filas de las contadas, se revierte entero.

        La comparacion `changed != count` es la unica prueba de que la
        escritura no salio del alcance revisado. Se falsifica inflando el
        recuento del plan: el migrate TIENE que detectarlo y revertir.
        """
        m = self.migration
        m["state_dbs"][0]["impact"]["cycles.project_id"] += 1
        problems = self.apply(m)
        self.assertTrue(problems, "un recuento inflado tiene que abortar")
        self.assertIn("se escribieron", problems[0])
        # El rollback tiene que ser real: el ledger sigue como estaba.
        self.assertEqual(
            sorted(r[0] for r in rows(self.db, "SELECT cycle_id FROM cycles")),
            sorted([f"{OLD}/m1", f"{OLD}/m2", "SENT-CYCLE", f"{FOREIGN}/m9"]),
        )

    # ── la invariante estructural, no una coincidencia de test ───────

    def test_rewrite_sql_always_carries_the_owned_predicate(self) -> None:
        """El predicado de escritura es el MISMO objeto que el de recuento.

        Si esto se cumple, las filas escritas no pueden apartarse de las
        contadas aunque alguien edite el SQL alrededor: el `WHERE` no es una
        constante escrita a mano, es la funcion.
        """
        for table in ("cycles", "events_v1", "projects"):
            for col in ("project_id", "cycle_id", "workspace_id"):
                sql = self.m.rewrite_sql(table, col)
                self.assertIn("WHERE", sql)
                self.assertIn(self.m.owned_predicate(col), sql)
                # Conserva el sufijo: `p-old/x` -> `p-new/x`, no `p-new`.
                self.assertIn(f"substr(\"{col}\", ?)", sql)
                self.assertNotIn(f'SET "{col}" = ?"', sql)

    def test_count_excludes_foreign_and_sentinel_rows(self) -> None:
        """Propio + ajeno = total. Ni una fila queda fuera de las dos cuentas.

        Es la propiedad que hace que `count_mismatch` signifique algo: si una
        fila no estuviera ni en el recuento del plan ni en la lista de
        ajenas, la comparacion `changed != count` no podria detectarla.
        """
        impact = self.m.sqlite_impact(self.db, OLD)
        foreign = self.m.foreign_values(self.db, OLD)
        checked = 0
        for table in ("projects", "workspaces", "cycles", "events_v1", "gate_receipts"):
            # Solo columnas que EXISTEN. Un identificador entre comillas dobles
            # que no corresponde a ninguna columna no da error en SQLite: cae
            # a literal de cadena, y `WHERE "cycle_id" IS NOT NULL` se
            # convierte en `WHERE 'cycle_id' IS NOT NULL`, que es cierto en
            # todas las filas. Por eso el PRAGMA y no una lista a mano.
            present = {r[1] for r in rows(self.db, f'PRAGMA table_info("{table}")')}
            for col in ("project_id", "workspace_id", "cycle_id"):
                if col not in present:
                    continue
                total = rows(
                    self.db,
                    f'SELECT COUNT(*) FROM "{table}" WHERE "{col}" IS NOT NULL',
                )[0][0]
                if total == 0:
                    continue
                owned = impact.get(f"{table}.{col}", 0)
                alien = sum(n for k, n in foreign.items() if k.startswith(f"{table}.{col}="))
                self.assertEqual(
                    owned + alien, total,
                    f"{table}.{col}: {owned} propias + {alien} ajenas != {total} totales",
                )
                checked += 1
        self.assertGreaterEqual(checked, 9)


def main() -> int:
    import sys
    result = unittest.main(module=__name__, argv=[sys.argv[0]], exit=False).result
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
