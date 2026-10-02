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


class ClassificationContract(unittest.TestCase):
    """La clase de una migracion se DECLARA; no depende de quien la mire.

    Decidido por el operador en session-66 sobre el contenido real de los
    ledgers: se migran los renombrados limpios y nada mas. Si la clasificacion
    se degrada a `clean_rename` por defecto, un `ledger_merge` pasaria a
    ejecutarse sin que nadie lo autorizara — que es justo el defecto que
    corrigio `migrate_one`.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.m = load()

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.share = self.root / "share"
        self.state = self.root / "state"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def _make(self, pid, cycles):
        d = self.state / "projects" / pid
        d.mkdir(parents=True)
        conn = sqlite3.connect(d / "ledger.sqlite")
        conn.execute("CREATE TABLE cycles (cycle_id TEXT, project_id TEXT)")
        conn.executemany("INSERT INTO cycles VALUES (?,?)",
                         [(f"{pid}/{c}", pid) for c in cycles])
        conn.commit()
        conn.close()
        return d / "ledger.sqlite"

    def test_absent_destination_is_a_clean_rename(self) -> None:
        self.assertEqual(
            self.m.classify(OLD, NEW, self.share, self.state)["class"], "clean_rename"
        )

    def test_destination_without_cycles_is_an_empty_shell(self) -> None:
        (self.share / "projects" / NEW).mkdir(parents=True)
        (self.share / "projects" / NEW / "workspaces").mkdir()
        self._make(OLD, ["m1", "m2"])
        self._make(NEW, [])
        got = self.m.classify(OLD, NEW, self.share, self.state)
        self.assertEqual(got["class"], "empty_shell")
        self.assertEqual(got["old_cycles"], 2)
        self.assertEqual(got["new_cycles"], 0)

    def test_cycles_on_both_sides_is_a_ledger_merge(self) -> None:
        (self.share / "projects" / NEW).mkdir(parents=True)
        self._make(OLD, ["m1", "m2"])
        self._make(NEW, ["m3"])
        got = self.m.classify(OLD, NEW, self.share, self.state)
        self.assertEqual(got["class"], "ledger_merge")
        self.assertEqual(got["name_collisions"], [])

    def test_merge_records_the_name_collision_not_just_the_count(self) -> None:
        """El id crudo nunca coincide entre proyectos distintos.

        `p-a/x` y `p-b/x` son distintos aunque `x` sea el mismo ciclo. Lo que
        choca al re-apuntar el id viejo al nuevo es el NOMBRE corto, asi que
        la colision tiene que nombrarse, no solo contarse.
        """
        (self.share / "projects" / NEW).mkdir(parents=True)
        self._make(OLD, ["r6-workers-probe-wiring", "m1"])
        self._make(NEW, ["r6-workers-probe-wiring", "m2"])
        got = self.m.classify(OLD, NEW, self.share, self.state)
        self.assertEqual(got["class"], "ledger_merge")
        self.assertEqual(got["name_collisions"], ["r6-workers-probe-wiring"])

    def test_cycle_names_strips_the_project_prefix(self) -> None:
        db = self._make(OLD, ["a", "b/c"])
        self.assertEqual(self.m.cycle_names(db), {"a", "b/c"})

    def test_apply_only_touches_clean_renames(self) -> None:
        """Un `ledger_merge` marcado como tal NO se ejecuta."""
        (self.share / "projects" / NEW).mkdir(parents=True)
        db_old = self._make(OLD, ["m1"])
        self._make(NEW, ["m2"])
        m = build_migration(self.m, self.share, self.state, db_old)
        m["class"] = "ledger_merge"
        # `migrate_one` se niega a fusionar porque el destino existe: con lo
        # que ya tiene, la unica defensa que le queda es no llegar a llamarse.
        self.assertTrue(self.m.migrate_one(m, out=lambda s: None))
        self.assertTrue((self.state / "projects" / OLD).is_dir())
        self.assertTrue((self.state / "projects" / NEW).is_dir())

    def test_select_migratable_holds_everything_but_clean_renames(self) -> None:
        """El filtro de `cmd_apply` es una FUNCION, y esta es su prueba.

        Sin esto, degradar el filtro a «migra todo» pasaria la suite: el filtro
        estaria dentro del `for` de `cmd_apply` y ningun test lo alcanzaria.
        """
        def mig(klass):
            return {"class": klass, "old_project_id": OLD, "new_project_id": NEW}

        clean, held = self.m.select_migratable(
            [mig("clean_rename"), mig("empty_shell"), mig("ledger_merge")]
        )
        self.assertEqual([m["class"] for m in clean], ["clean_rename"])
        self.assertEqual([m["class"] for m in held], ["empty_shell", "ledger_merge"])

    def test_select_migratable_never_silently_promotes_a_merge(self) -> None:
        """Una clase desconocida se RETIENE, no se migra por defecto.

        El valor por defecto de `class` es `clean_rename` para no romper planes
        viejos, pero una clase que no sea la conocida se queda intacta: perder un
        proyecto por una etiqueta nueva es reversible; migrar el equivocado, no.
        """
        clean, held = self.m.select_migratable([
            {"class": "clean_rename"},
            {"class": "lo_que_se_me_ocurra"},
            {"class": "ledger_merge"},
        ])
        self.assertEqual(len(clean), 1)
        self.assertEqual(len(held), 2)


class AppendOnlyContract(unittest.TestCase):
    """El fact log es inmutable: la identidad del proyecto queda sellada.

    session-66, al ejecutar de verdad el `apply`. `events_v1` tiene triggers
    `BEFORE UPDATE` que hacen `RAISE(ABORT, 'events_v1 are append-only')`, y
    `EventEnvelopeV1::compute_content_hash` solo anula `content_hash`,
    `sequence` y `recorded_at` — `project_id`, `stream_id` y `cycle_id` entran
    en el hash. Reescribir esas columnas no es "relajar un trigger": deja
    `verify_stream_chain` fallando con `hash_drift`.

    Los 7 proyectos que el operador autorizo migrar tienen los 7 filas de
    eventos. La migracion no es que fueradangerosa: es que no existe.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.m = load()

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.share = self.root / "share"
        self.state = self.root / "state"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def _ledger(self, pid, events=0, cycles=()):
        d = self.state / "projects" / pid
        d.mkdir(parents=True)
        conn = sqlite3.connect(d / "ledger.sqlite")
        conn.executescript("""
        CREATE TABLE cycles (cycle_id TEXT, project_id TEXT);
        CREATE TABLE events_v1 (
            sequence INTEGER, project_id TEXT, cycle_id TEXT,
            content_hash TEXT);
        CREATE TRIGGER events_v1_no_update BEFORE UPDATE ON events_v1
        BEGIN SELECT RAISE(ABORT, 'events_v1 are append-only'); END;
        """)
        conn.executemany("INSERT INTO cycles VALUES (?,?)",
                         [(f"{pid}/{c}", pid) for c in cycles])
        conn.executemany("INSERT INTO events_v1 VALUES (?,?,?,?)",
                         [(i, pid, f"{pid}/m1", f"sha256:{i}") for i in range(events)])
        conn.commit()
        conn.close()
        return d / "ledger.sqlite"

    def test_a_real_trigger_turns_into_a_problem_not_a_traceback(self) -> None:
        """El rechazo del storage se INFORMA; no se propaga como excepcion.

        Sin esto, un `ledger_merge` de los 6 habria aplicado los 5 anteriores y
        el unico informe seria un traceback a mitad del bucle.
        """
        self._ledger(OLD, events=3, cycles=["m1"])
        (self.share / "projects" / OLD).mkdir(parents=True)
        m = build_migration(self.m, self.share, self.state,
                            self.state / "projects" / OLD / "ledger.sqlite")
        problems = self.m.migrate_one(m, out=lambda s: None)
        self.assertTrue(problems)
        self.assertIn("append-only", problems[0])
        self.assertIn("reverted", problems[0])
        # Y nada se movio.
        self.assertTrue((self.state / "projects" / OLD).is_dir())

    def test_classification_blocks_before_even_trying(self) -> None:
        """Un proyecto con fact log se declara, no se descubre al reventar."""
        self._ledger(OLD, events=28, cycles=["m1", "m2"])
        got = self.m.classify(OLD, NEW, self.share, self.state)
        self.assertEqual(got["class"], "blocked_append_only")
        self.assertEqual(got["append_only"], {"events_v1": 28})
        self.assertNotEqual(got["class"], "clean_rename")

    def test_append_only_beats_a_free_destination(self) -> None:
        """Aunque el destino este libre, el fact log sigue mandando.

        Si el orden de las comprobaciones se invirtiera, este proyecto pasaria
        por `clean_rename` y el `apply` reventaria contra el trigger.
        """
        self._ledger(OLD, events=1, cycles=["m1"])
        self.assertEqual(
            self.m.classify(OLD, NEW, self.share, self.state)["class"],
            "blocked_append_only",
        )

    def test_a_project_without_fact_log_is_still_migratable(self) -> None:
        """El bloqueo es por tener historia, no por existir: no anula la clase buena."""
        self._ledger(OLD, events=0, cycles=["m1"])
        self.assertEqual(
            self.m.classify(OLD, NEW, self.share, self.state)["class"], "clean_rename"
        )

    def test_append_only_rows_ignores_another_projects_events(self) -> None:
        """`append_only_rows` cuenta las MIA, no las de otro ledger."""
        db = self._ledger(OLD, events=4, cycles=["m1"])
        conn = sqlite3.connect(db)
        conn.executemany("INSERT INTO events_v1 VALUES (?,?,?,?)",
                         [(i, FOREIGN, f"{FOREIGN}/x", f"sha256:f{i}") for i in range(5)])
        conn.commit()
        conn.close()
        self.assertEqual(self.m.append_only_rows(db, OLD), {"events_v1": 4})


def main() -> int:
    import sys
    result = unittest.main(module=__name__, argv=[sys.argv[0]], exit=False).result
    return 0 if result.wasSuccessful() else 1

if __name__ == "__main__":
    raise SystemExit(main())
