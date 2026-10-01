#!/usr/bin/env python3
"""Contrato del espejo de identidad de `scripts/migrate_project_identity.py`.

El script deriva `project_id` reimplementando `sddk_domain`. Si el espejo se
separa del Rust, el script calcula el destino equivocado de una migración
DESTRUCTIVA sobre ledgers reales, así que el espejo se fija por test igual
que el normalizador en `crates/sddk-domain/src/identity.rs`.

El corpus del propio script se generó desde el binario real vía
`sddk project resolve --remote … --format json`; aquí se comprueba que sigue
siendo coherente consigo mismo y que cubre lo que la primera versión —que se
escribió a ojo— no cubría.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "migrate_project_identity.py"


def load():
    spec = importlib.util.spec_from_file_location("migrate_project_identity", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class MirrorContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.m = load()

    def test_selfcheck_passes(self) -> None:
        """El corpus dorado se satisface entero."""
        ok, problems = self.m.selfcheck()
        self.assertTrue(ok, "\n".join(problems))

    def test_corpus_is_not_trivially_small(self) -> None:
        """Un corpus reducido volvería a no cubrir las formas divergentes."""
        self.assertGreaterEqual(len(self.m.ACCEPTED), 21)
        self.assertGreaterEqual(len(self.m.REJECTED), 8)
        self.assertEqual(len(self.m.LEGACY_IDS), 2)

    def test_scp_with_credentials_is_accepted(self) -> None:
        """`git@host:owner/repo` es la forma mas comun y la que se perdio."""
        self.assertEqual(
            self.m.normalize_remote_url("git@github.com:owner/repo.git"),
            "https://github.com/owner/repo",
        )

    def test_default_port_depends_on_the_scheme(self) -> None:
        """Un conjunto global {"443","22"} borra puertos que el Rust conserva."""
        # scp no tiene puerto por defecto: 443 se conserva como path.
        self.assertEqual(
            self.m.normalize_remote_url("git@github.com:443/owner/repo"),
            "https://github.com/443/owner/repo",
        )
        # https por defecto es 443, luego 22 se conserva.
        self.assertEqual(
            self.m.normalize_remote_url("https://github.com:22/owner/repo"),
            "https://github.com:22/owner/repo",
        )
        # ssh por defecto es 22 y https es 443: ambos se eliminan.
        self.assertEqual(
            self.m.normalize_remote_url("ssh://git@github.com:22/owner/repo"),
            "https://github.com/owner/repo",
        )
        self.assertEqual(
            self.m.normalize_remote_url("https://github.com:443/owner/repo"),
            "https://github.com/owner/repo",
        )

    def test_host_lowercasing_is_ascii_only(self) -> None:
        """`to_ascii_lowercase` no toca U+00DC; `str.lower()` si.

        La diferencia es invisible en pantalla —ambos se ven "Uber"— pero
        produce un project_id distinto, asi que se compara por codepoint.
        """
        self.assertEqual(
            [ord(c) for c in self.m._ascii_lower("\u00dcBER")],
            [0xDC, 0x62, 0x65, 0x72],
        )
        self.assertNotEqual(
            self.m._ascii_lower("\u00dcBER"),
            "\u00dcBER".lower(),
        )

    def test_empty_authority_and_empty_port_are_rejected(self) -> None:
        for remote in ("https:///owner/repo", "https://github.com:/owner/repo"):
            self.assertIsNone(self.m.normalize_remote_url(remote), remote)

    def test_ipv6_remainder_must_start_with_colon(self) -> None:
        self.assertIsNone(self.m.normalize_remote_url("https://[::1]x/owner/repo"))
        self.assertEqual(
            self.m.normalize_remote_url("https://[::1]:8080/owner/repo"),
            "https://[::1]:8080/owner/repo",
        )

    def test_legacy_ids_of_this_machine_are_still_reproduced(self) -> None:
        """Los dos ids del caso que motiva INC-DEBT-050."""
        self.assertEqual(
            self.m.stable_project_id(
                "https://github.com/Rubentxu/software-development-decision-kernel", "."
            ),
            "p-63676b11dc0ef88f",
        )
        self.assertEqual(
            self.m.stable_project_id(
                "https://github.com/rubentxu/software-development-decision-kernel", "."
            ),
            "p-995939af668a53d8",
        )

    def test_normalizer_never_raises(self) -> None:
        """Una excepcion en vez de `None` abortaria `audit` sin explicacion."""
        probes = [
            "", " ", "://", "https://", "https://:", "https://[", "https://[]",
            "https://[::1", "git@", ":", "git@:", "https://a@b@c/d/e",
            "https://host:/", "//", "https://host//", "\u00dc", "https://\u00dc/",
        ]
        for probe in probes:
            with self.subTest(probe=probe):
                try:
                    self.m.normalize_remote_url(probe)
                except Exception as exc:  # noqa: BLE001
                    self.fail(f"{probe!r} lanzo {type(exc).__name__}: {exc}")

    def test_receipt_globbing_pattern_is_not_silently_empty(self) -> None:
        """`Path(.../'p-*'...).glob('adoption.json')` devuelve 0 sin error.

        Esa forma hizo que `audit` imprimiera "selfcheck: ok" con cero
        receipts: un inventario vacio indistinguible de "no hay nada que
        migrar". Se comprueba la forma del patron, no el contenido del disco.
        """
        import inspect

        source = inspect.getsource(self.m.iter_receipts)
        self.assertIn('share.glob("projects/p-*', source)
        self.assertNotIn('.glob("adoption.json")', source)


if __name__ == "__main__":
    unittest.main()
