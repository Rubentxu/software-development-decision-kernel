#!/usr/bin/env python3
"""Gate: every copy of "what ships in the bundle" must agree with the manifest.

`MANIFEST.sURFACES` decides what `MANIFEST.sha256` hashes, `ContentsSection`
declares one counter per surface, and the two production paths decide what
actually reaches the tarball. Nothing in the type system ties them together.

That gap is not hypothetical. INC-DEBT-052 shipped `prompts_count = 0` in
every bundle the tool ever wrote, because the surface list named
`prompts/sddk` and the counting `match` looked for the literal `prompts`;
the arm never fired and `_ => {}` swallowed it. `count_surface_entries`
now aborts instead of writing a zero, and its own receipt recorded the
residual risk as *unimplemented*: it covers content drift but not schema
drift, when all the copies have the field and their meanings separate.

This gate closes that. It fails when the copies disagree in either
direction: a surface with no counter, a counter with no surface, a surface
no tar ships, or a tar shipping a surface nothing else declares.

Session-65h added a fifth copy, and the two defects it then measured were
both invisible to the list comparison this gate was built on:

  (a) `release.sh`'s `tar` named `specs` and `docs/impeccable-reference`
      while the `cp -r` staging never copied them. The tar aborted on a
      non-existent member — a release that cannot complete at all.
  (b) `cp -r <surface>` copies whatever is on disk, INCLUDING files that
      `.gitignore` excludes and the manifest does not list. Measured on
      2.5.3: `agents/.atl/.skill-registry.cache.json` and
      `assets/agent-models.yaml.bak` shipped as bundle content, so
      `manifest_sha256` in BUNDLE.toml did not describe the tarball.

Both had the same cause: the staging was a hand-maintained list, a fifth
copy of the contract, independent of the authority. It was replaced with a
staging DERIVED FROM `MANIFEST.sha256`, so the two production paths no
longer state the same fact in the same way:

  - `release.yml` (cloud) names the surface directories explicitly. It runs
    in a fresh checkout, so gitignored debris cannot exist there; the
    explicit list is safe and readable.
  - `release.sh` (local) reads the manifest, because it runs in a working
    tree where debris does exist.

So the gate no longer compares two lists for `release.sh`. It asserts the
structural property instead: the local path takes its contents from the
manifest and ships the whole staged tree, with no surface list of its own
to drift.
"""
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

COMMON = ROOT / "crates/sddk-cli/src/dev/common.rs"
BUNDLE_MANIFEST = ROOT / "crates/sddk-cli/src/dev/bundle_manifest.rs"
MANIFEST_RS = ROOT / "crates/sddk-cli/src/dev/manifest.rs"
RELEASE_SH = ROOT / "scripts/release.sh"
RELEASE_YML = ROOT / ".github/workflows/release.yml"
MANIFEST_SHA = ROOT / "MANIFEST.sha256"

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
    # A subdirectory of `docs/`, under a field with no path in it. Shipping
    # `docs/` wholesale would drag `docs/history/` into the bundle; shipping
    # nothing would leave `impeccable-primary.md` — which DOES ship — citing
    # two files the bundle does not carry.
    "docs/impeccable-reference": "impeccable_reference",
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


def strip_comments(raw: str) -> str:
    """Drop whole-line comments and join backslash continuations.

    Comments are the source of prose that mentions the same directories as
    the commands, which is exactly the ambiguity when parsing. Continuations
    must be joined before selecting a command, or only its first physical
    line is seen.
    """
    code = "\n".join(
        line for line in raw.splitlines() if not line.lstrip().startswith("#")
    )
    return re.sub(r"\\\s*\n", " ", code)


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
    """
    logical = strip_comments(script.read_text(encoding="utf-8"))

    # Select the ONE `tar` that CREATES the framework bundle tarball.
    #
    # Selecting on the output filename does not work: `release.sh` names it
    # through a variable (`tar czf "$BUNDLE_TARBALL" ...`) while `release.yml`
    # writes the literal. Selecting on "the line containing .tar.gz" is worse
    # still — in release.yml that is 19 lines, because the filename is passed
    # around by `cp`, `sha256sum` and the asset upload.
    #
    # What both share is that this specific tar packs the FRAMEWORK BUNDLE,
    # and it packs it in one of exactly two shapes:
    #   - release.sh: the whole staged tree, by its wrapper directory name;
    #   - release.yml: the surface list, ending in MANIFEST.sha256.
    # So the anchor is the bundle content, not the archive name. `tar xzf`
    # (extraction) and `tar tzf` (listing) are excluded by requiring a
    # create verb: `c`, not `x` or `t`.
    candidates = [
        line
        for line in logical.splitlines()
        if re.search(r"\btar\s+[a-z]*c[a-z]*\b", line)
        and (
            "software-development-decision-kernel" in line
            or "MANIFEST.sha256" in line
        )
    ]
    if len(candidates) != 1:
        raise AssertionError(
            f"{script.relative_to(ROOT)}: expected exactly 1 tar command creating "
            f"the framework bundle, found {len(candidates)}:\n  "
            + "\n  ".join(candidates)
        )
    line = candidates[0]
    return _member_dirs(line)


def _member_dirs(line: str) -> set[str]:
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


def release_script_stages_from_the_manifest() -> bool:
    """True if the local release path derives its staging from the manifest.

    The property that matters, and the reason `release.sh` stopped naming
    surfaces: whatever the manifest lists is what ships, and nothing else
    can. Read the code rather than a list, so the gate cannot be satisfied
    by re-adding a list that happens to agree today.
    """
    logical = strip_comments(RELEASE_SH.read_text(encoding="utf-8"))
    return bool(
        re.search(
            r"awk\s+'\{print\s+\$2\}'\s+MANIFEST\.sha256\s*\|\s*xargs\b[^\n]*cp\b",
            logical,
        )
    )


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

    def test_release_script_stages_from_the_manifest(self):
        self.assertTrue(
            release_script_stages_from_the_manifest(),
            "scripts/release.sh no longer derives its staging from "
            "MANIFEST.sha256. A hand-written surface list is a fifth copy of "
            "the contract: it shipped a tar naming surfaces the staging never "
            "copied, and copied gitignored files the manifest does not list "
            "(INC-DEBT-056, session-65h)",
        )

    def test_release_script_ships_the_whole_staged_tree(self):
        """The tar must pack the staged tree, not a re-listed subset.

        The staging is now the authority. A `tar` that names directories
        again would reintroduce exactly the drift the staging removed: two
        lists, agreeing only until someone adds a surface to one.
        """
        members = tar_surfaces(RELEASE_SH)
        # Exact equality, not containment. Falsified: a first version asserted
        # `"agents" not in members` and `len(members) == 1`, which a mutation
        # naming `software-development-decision-kernel/agents` satisfied —
        # the subset was still one member, just not the expected one. A
        # prefix match is how a re-listed tar hides.
        self.assertEqual(
            members,
            {"software-development-decision-kernel"},
            "scripts/release.sh's tar must pack the staged tree by its wrapper "
            "directory. Naming paths inside it re-lists the contents and "
            "reintroduces the drift the manifest-derived staging removed",
        )

    def test_staged_tree_matches_the_manifest_exactly(self):
        """Execute the staging and compare its output to the manifest.

        Structural assertions read the code; this one runs it. It is the only
        test here that can catch a file present in the working tree, matched
        by no surface list and listed by no manifest — the defect (b) above,
        which no amount of list-comparison can see, because the file is
        correctly absent from every list.
        """
        manifest_paths = {
            line.split("  ", 1)[1]
            for line in MANIFEST_SHA.read_text(encoding="utf-8").splitlines()
            if line.strip()
        }
        self.assertTrue(manifest_paths, "MANIFEST.sha256 parsed as empty")

        with tempfile.TemporaryDirectory() as tmp:
            stage = Path(tmp) / "bundle-stage"
            stage.mkdir()
            result = _stage_from_manifest(stage)
            self.assertEqual(
                result.returncode,
                0,
                f"staging from the manifest failed: {result.stderr.strip()}",
            )
            staged = {
                str(p.relative_to(stage)) for p in stage.rglob("*") if p.is_file()
            }

        self.assertEqual(
            staged - manifest_paths,
            set(),
            "the staged bundle contains files the manifest does not list. If "
            "this is a gitignored artefact in the working tree, it would ship "
            "in the tarball while `manifest_sha256` in BUNDLE.toml claims to "
            "cover the contents",
        )
        self.assertEqual(
            manifest_paths - staged,
            set(),
            "the manifest lists files the staging did not produce, so the "
            "published tarball would omit content its own manifest claims",
        )

    def test_manifest_covers_every_tracked_file_under_the_surfaces(self):
        """`MANIFEST.sha256` must match what git tracks, in both directions.

        The staging test above derives BOTH sides from the manifest, so it is
        tautological against a hand-edited manifest: deleting an entry
        removes it from the expected and the actual set alike, and the gate
        stays green while the file silently stops shipping. Falsified exactly
        that way (mutation M8).

        So this test anchors on an authority the manifest cannot define for
        itself — the tracked tree. `sddk dev manifest` builds the manifest
        from `git ls-files`, so the two must agree; when they do not, the
        manifest has been edited by hand or is stale.
        """
        result = subprocess.run(
            ["git", "ls-files", "-z", "--", *sorted(self.surfaces)],
            cwd=ROOT,
            capture_output=True,
        )
        self.assertEqual(
            result.returncode,
            0,
            f"git ls-files failed: {result.stderr.decode(errors='replace').strip()}",
        )
        tracked = {p for p in result.stdout.decode().split("\0") if p}
        manifest_paths = _manifest_paths()

        self.assertEqual(
            manifest_paths - tracked,
            set(),
            "MANIFEST.sha256 lists files git does not track under the "
            "surfaces. The published bundle would contain content with no "
            "source in the repository",
        )
        self.assertEqual(
            tracked - manifest_paths,
            set(),
            "MANIFEST.sha256 omits tracked files under the surfaces, so they "
            "would silently stop shipping while the manifest claims to cover "
            "the bundle. Re-run `sddk dev manifest --root .`",
        )

    def test_untracked_files_under_a_surface_do_not_reach_the_bundle(self):
        """A gitignored artefact in the tree must not be published.

        The measured instance of INC-DEBT-056 (b): `cp -r <surface>` copied
        `agents/.atl/.skill-registry.cache.json` and
        `assets/agent-models.yaml.bak` — both matched by `.gitignore` and
        neither listed in the manifest — into the published tarball, so
        `manifest_sha256` in BUNDLE.toml did not describe its own artifact.

        This drops a canary into a real surface, runs the actual staging
        pipeline, and asserts the canary did not come along. That is the
        only shape of this test that can fail: no list of surfaces can
        mention a file that is correctly absent from every list.
        """
        canary = ROOT / "assets/.bundle-staging-canary"
        self.assertFalse(canary.exists(), f"leftover canary at {canary}")
        canary.write_text("canary\n", encoding="utf-8")
        try:
            self.assertFalse(
                _is_ignored(canary),
                "the canary is gitignored, so this test no longer proves "
                "anything about untracked files",
            )
            self.assertNotIn(
                "assets/.bundle-staging-canary",
                _manifest_paths(),
                "the canary must be absent from the manifest, or this test is "
                "not exercising the untracked path",
            )
            with tempfile.TemporaryDirectory() as tmp:
                stage = Path(tmp) / "bundle-stage"
                stage.mkdir()
                rc = _stage_from_manifest(stage)
                self.assertEqual(rc.returncode, 0, rc.stderr.strip())
                staged = {
                    str(p.relative_to(stage)) for p in stage.rglob("*") if p.is_file()
                }
            self.assertIn(
                "assets/agent-models.yaml",
                staged,
                "control: a real surface file must be staged, or the canary's "
                "absence proves nothing",
            )
            self.assertNotIn(
                "assets/.bundle-staging-canary",
                staged,
                "an untracked file under a surface reached the staging tree. "
                "It would ship in the tarball while the manifest — and the "
                "`manifest_sha256` derived from it — claims to cover the "
                "contents (INC-DEBT-056 b)",
            )
        finally:
            canary.unlink()

    def test_workflow_tar_matches_the_surface_list(self):
        self.assertEqual(
            tar_surfaces(RELEASE_YML),
            self.surfaces,
            ".github/workflows/release.yml's tar does not ship exactly "
            "MANIFEST_SURFACES: the cloud release and the local one would "
            "publish different bundles from the same commit",
        )

    def test_no_surface_ships_that_the_manifest_does_not_cover(self):
        """The cloud path's explicit list must stay inside the manifest.

        `release.yml` packs a fresh checkout, so it cannot pick up working-tree
        debris — but it still states the surface list a second time, and a
        directory it names must exist in the manifest for the two to describe
        the same bundle.
        """
        manifest_paths = {
            line.split("  ", 1)[1]
            for line in MANIFEST_SHA.read_text(encoding="utf-8").splitlines()
            if line.strip()
        }
        uncovered = {
            s for s in tar_surfaces(RELEASE_YML)
            if not any(p.startswith(f"{s}/") for p in manifest_paths)
        }
        self.assertEqual(
            uncovered,
            set(),
            f"release.yml ships {uncovered}, which MANIFEST.sha256 does not "
            f"cover: those files would be published without a recorded digest",
        )


def _manifest_paths() -> set[str]:
    """The paths `MANIFEST.sha256` declares, in the file's own order-free form."""
    return {
        line.split("  ", 1)[1]
        for line in MANIFEST_SHA.read_text(encoding="utf-8").splitlines()
        if line.strip()
    }


def _shell_list(paths: set[str]) -> str:
    """A single-quoted shell word per path, for the staging subprocess."""
    return " ".join("'" + p.replace("'", "'\\''") + "'" for p in sorted(paths))


def _stage_from_manifest(stage: Path) -> subprocess.CompletedProcess:
    """Run the release script's staging pipeline, for real.

    The pipeline is duplicated here rather than shelled out to `release.sh`,
    which would build and publish. Duplicating it is a risk — the test could
    pass against a staging that the script no longer performs — and that risk
    is paid for by `test_release_script_stages_from_the_manifest`, which
    asserts the script still contains this exact pipeline. The two tests fail
    together if the staging is ever changed on one side only.
    """
    return subprocess.run(
        [
            "bash",
            "-c",
            f"awk '{{print $2}}' {MANIFEST_SHA} "
            f"| xargs -d '\\n' cp --parents -t \"$1\"",
            "_",
            str(stage),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )


def _is_ignored(path: Path) -> bool:
    result = subprocess.run(
        ["git", "check-ignore", "-q", str(path)],
        cwd=ROOT,
        capture_output=True,
    )
    return result.returncode == 0


if __name__ == "__main__":
    unittest.main(verbosity=2)
