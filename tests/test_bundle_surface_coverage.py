#!/usr/bin/env python3
"""Gate: the four copies of the bundle surface list must agree.

`MANIFEST_SURFACES` decides what `MANIFEST.sha256` hashes, `ContentsSection`
declares one counter per surface, and TWO `tar` command lines — one in
`scripts/release.sh`, one in `.github/workflows/release.yml` — decide what
actually ships in the tarball. Nothing in the type system ties them
together.

That gap is not hypothetical. INC-DEBT-052 shipped `prompts_count = 0` in
every bundle the tool ever wrote, because the surface list named
`prompts/sddk` and the counting `match` looked for the literal `prompts`;
the arm never fired and `_ => {}` swallowed it. `count_surface_entries`
now aborts instead of writing a zero, and its own receipt recorded the
residual risk as *unimplemented*: it covers content drift but not schema
drift, when all the copies have the field and their meanings separate.

This gate closes that. It fails when the four disagree in either
direction: a surface with no counter, a counter with no surface, a
surface no tar ships, or a tar shipping a surface nothing else declares.
"""
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

COMMON = ROOT / "crates/sddk-cli/src/dev/common.rs"
BUNDLE_MANIFEST = ROOT / "crates/sddk-cli/src/dev/bundle_manifest.rs"
MANIFEST_RS = ROOT / "crates/sddk-cli/src/dev/manifest.rs"
RELEASE_SH = ROOT / "scripts/release.sh"
RELEASE_YML = ROOT / ".github/workflows/release.yml"

# Surface directory -> the `*_count` field that describes it in
# `ContentsSection`. Written out rather than derived, because the names do
# NOT correspond: the surface is `prompts/sddk` and the field is
# `prompts_count`. Deriving one from the other is what let INC-DEBT-052
# ship `prompts_count = 0` — the counting `match` looked for a literal that
# the surface list never contained, the arm never fired, and `_ => {}`
# swallowed it. An explicit table is the only form that cannot drift that
# way, and adding a surface now forces a line here.
SURFACE_TO_FIELD = {
    "agents": "agents",
    "skills": "skills",
    "prompts/sddk": "prompts",
    "assets": "assets",
    "specs": "specs",
}


def manifest_surfaces() -> list[str]:
    """Parse the `MANIFEST_SURFACES` array literal.

    The type is `[&str; 5]`, so the first `[` after the name belongs to the
    TYPE, not to the list. Anchor on the `=` and take the brackets after it.
    """
    text = COMMON.read_text(encoding="utf-8")
    start = text.index("MANIFEST_SURFACES: [&str;")
    equals = text.index("=", start)
    open_bracket = text.index("[", equals)
    close_bracket = text.index("]", open_bracket)
    body = text[open_bracket + 1 : close_bracket]
    return re.findall(r'"([^"]+)"', body)


def contents_fields() -> set[str]:
    """Surface names that have a `*_count` field in `ContentsSection`."""
    text = BUNDLE_MANIFEST.read_text(encoding="utf-8")
    start = text.index("pub struct ContentsSection")
    end = text.index("\n}", start)
    body = text[start:end]
    return {m.removesuffix("_count") for m in re.findall(r"pub (\w+_count):", body)}


def counting_arms() -> set[str]:
    """Surfaces the `count_surface_entries` match knows how to increment."""
    text = MANIFEST_RS.read_text(encoding="utf-8")
    start = text.index("fn count_surface_entries")
    end = text.index("other => anyhow::bail!", start)
    return set(re.findall(r'"([^"]+)" => &mut counts\.', text[start:end]))


def tar_surfaces(script: Path) -> set[str]:
    """Directory arguments of the `tar` command that builds the bundle tarball.

    Two things make this harder than it looks, and both bit the first
    version of this gate:

    1. `release.sh` writes the tarball as a backslash-continued command, so
       the line naming `MANIFEST.sha256` is a CONTINUATION and the `tar`
       verb is on the previous line. Selecting "the line with tar and
       MANIFEST.sha256" therefore finds only the header comment
       (`# 5. Bundle - tar agents/ ... MANIFEST.sha256`) and parses a prose
       sentence as a command.
    2. `tar`'s verb letters carry no leading dash, so `czf` reads as a
       member directory unless it is named explicitly.

    So: strip comment lines, join continuations, then take the members.
    """
    raw = script.read_text(encoding="utf-8")

    # Drop whole-line comments. They are the source of the prose that
    # mentions the same directories, which is exactly the ambiguity here.
    code = "\n".join(
        line for line in raw.splitlines() if not line.lstrip().startswith("#")
    )

    # Join backslash continuations into single logical commands.
    logical = re.sub(r"\\\s*\n", " ", code)

    candidates = [
        line
        for line in logical.splitlines()
        if re.search(r"\btar\b", line) and "MANIFEST.sha256" in line
    ]
    if len(candidates) != 1:
        raise AssertionError(
            f"{script.relative_to(ROOT)}: expected exactly 1 tar command naming "
            f"MANIFEST.sha256, found {len(candidates)}:\n  " + "\n  ".join(candidates)
        )
    line = candidates[0]

    tar_verbs = {"tar", "czf", "cf", "xz", "cz", "c"}
    # Flags that consume the NEXT token as a value. Their value is an
    # expression or a pattern, never a member of the archive.
    value_flags = {
        "--transform",
        "--xform",
        "--exclude",
        "--exclude-from",
        "--directory",
        "--file",
    }

    surfaces: set[str] = set()
    skip_next = False
    for token in line.split():
        if skip_next:
            skip_next = False
            continue
        if token in value_flags:
            skip_next = True
            continue
        if token == "-C":
            skip_next = True
            continue
        if token.startswith("-"):
            continue
        if token in tar_verbs or token in {"MANIFEST.sha256", "BUNDLE.toml"}:
            continue
        if token.endswith((".tar.gz", ".tgz")):
            continue  # the archive being written, not a member of it
        if "$" in token:
            # A shell expansion, quoted or not: the staging dir, the output
            # path, or anything else the script computes.
            continue
        surfaces.add(token)
    return surfaces


class BundleSurfaceCoverage(unittest.TestCase):
    def setUp(self):
        self.surfaces = set(manifest_surfaces())
        self.assertTrue(self.surfaces, "MANIFEST_SURFACES parsed as empty")

    def test_manifiest_surfaces_is_not_empty_and_has_no_duplicates(self):
        listed = manifest_surfaces()
        self.assertEqual(
            len(listed),
            len(self.surfaces),
            f"MANIFEST_SURFACES has duplicates: {listed}",
        )

    def test_every_surface_has_a_counter_field(self):
        fields = contents_fields()
        missing = {s for s in self.surfaces if s not in SURFACE_TO_FIELD}
        self.assertEqual(
            missing,
            set(),
            f"MANIFEST_SURFACES declares {missing} but SURFACE_TO_FIELD in this "
            f"gate has no entry for it. Add the line, then add the field and its "
            f"match arm. Guessing the field name from the surface name is what "
            f"broke INC-DEBT-052",
        )
        absent = {
            f"{s}_count"
            for s, f in SURFACE_TO_FIELD.items()
            if s in self.surfaces and f not in fields
        }
        self.assertEqual(
            absent,
            set(),
            f"ContentsSection has no field for {absent}; count_surface_entries "
            f"would ABORT (fail-closed), or worse, silently ship 0",
        )

    def test_every_counter_field_has_a_surface(self):
        fields = contents_fields()
        mapped = {f for f in SURFACE_TO_FIELD.values()}
        orphan = fields - mapped
        self.assertEqual(
            orphan,
            set(),
            f"ContentsSection declares counters for {orphan}, which no surface "
            f"in SURFACE_TO_FIELD describes. A counter that can only be 0 is a "
            f"field that lies about the artifact",
        )

    def test_surfaces_and_the_table_agree_in_both_directions(self):
        self.assertEqual(
            self.surfaces,
            set(SURFACE_TO_FIELD),
            "MANIFEST_SURFACES and this gate's SURFACE_TO_FIELD name different "
            "surfaces. The gate's table is stale",
        )

    def test_every_surface_has_a_counting_arm(self):
        arms = counting_arms()
        missing = self.surfaces - arms
        self.assertEqual(
            missing,
            set(),
            f"count_surface_entries has no match arm for {missing}; it would "
            f"hit the `other => bail!` branch and abort bundle generation",
        )

    def test_release_script_tar_matches_the_surface_list(self):
        self.assertEqual(
            tar_surfaces(RELEASE_SH),
            self.surfaces,
            "scripts/release.sh's tar does not ship exactly MANIFEST_SURFACES. "
            "A surface in one and not the other is a bundle that omits it while "
            "its manifest claims to cover it",
        )

    def test_workflow_tar_matches_the_surface_list(self):
        self.assertEqual(
            tar_surfaces(RELEASE_YML),
            self.surfaces,
            ".github/workflows/release.yml's tar does not ship exactly "
            "MANIFEST_SURFACES: the cloud release and the local one would "
            "publish different bundles from the same commit",
        )

    def test_both_tars_agree_with_each_other(self):
        self.assertEqual(
            tar_surfaces(RELEASE_SH),
            tar_surfaces(RELEASE_YML),
            "release.sh and release.yml ship different directories, so a "
            "locally tested release is not the one the cloud publishes",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
