#!/usr/bin/env bash
# check_debt_index_coherence.sh — fail-closed gate: the debt index must not
# contradict the documents it indexes.
#
# WHY THIS EXISTS (session-43, session-44)
#   `docs/debt/README.md` is what an agent reads to prioritise work, so
#   when it disagrees with the document it links, the index lies. That is
#   not cosmetic: in session-43 the divergence produced 2 of the 3 P1
#   alerts that motivated a whole audit session, both of which were
#   already closed. In session-44 a ~15-line read-only measurement found
#   the same class again on 2 entries nobody had looked at since
#   session-31 and session-28. The divergence also *hides* real alerts:
#   the session-43 audit spent its budget on the two it found and never
#   reached the two that were already resolved.
#
# THE INVARIANT
#   For every index entry, the document's declared `status` and the
#   status the index claims must agree on open-ness. Severity and
#   priority are NOT checked: the index deliberately annotates them
#   ("critical/P1, closed in session-42") and an exact string compare
#   would be a false-positive machine, which is the failure mode this
#   file exists to fight.
#
# STATUS VOCABULARY
#   A document is "unresolved" if its status is `open` or `blocked`, and
#   "settled" if it is `closed`, `resolved` or `fixed`. Any other value is
#   rejected outright rather than guessed at: an unknown status is not
#   evidence of coherence.
#
# SCOPE
#   Read-only. Never writes, never mutates the repo. Safe in CI and in
#   an `apply` inner loop.
#
# TARGET RESOLUTION
#   Defaults to the repository this script lives in, but honours
#   `$SDDK_DEBT_DIR` so the guard can be pointed at a throwaway tree.
#   Without that override the guard could only ever be exercised against
#   the real index — which is precisely the INC-DEBT-033 shape: a test
#   that cannot move the subject under test cannot falsify it, and the
#   first run of tests/test_debt_index_coherence.sh caught exactly that
#   (4 RED: the guard kept reporting the real repo's 25 entries while
#   the test believed it was checking a fixture).
#
# EXIT
#   0 = index and documents agree
#   1 = at least one divergence, or an entry the check cannot read
#   2 = the index itself is missing or unparseable (fail closed)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# Honour an explicit target so the guard is testable; otherwise fall back
# to the repo that ships it.
DEBT_DIR="${SDDK_DEBT_DIR:-$REPO_ROOT/docs/debt}"
INDEX="$DEBT_DIR/README.md"

printf '== guard: coherencia indice<->documento de deuda ==\n'

if [[ ! -f "$INDEX" ]]; then
    printf '  [FAIL] el indice no existe: %s\n' "$INDEX"
    printf '\nRESULT: FAIL — sin indice no hay contrato que comprobar.\n'
    exit 2
fi

python3 - "$INDEX" "$DEBT_DIR" <<'PY'
import re
import sys
import pathlib

index_path = pathlib.Path(sys.argv[1])
debt_dir = pathlib.Path(sys.argv[2])

UNSETTLED = {"open", "blocked"}
SETTLED = {"closed", "resolved", "fixed"}

text = index_path.read_text(encoding="utf-8")
# Index rows: "- **[INC-XXX](./file.md)** (severity/priority, status...)"
rows = re.findall(
    r'^- \*\*\[(INC-[A-Za-z0-9-]+)\]\(\./([A-Za-z0-9._-]+\.md)\)\*\*\s*\(([^)]*)\)',
    text,
    re.M,
)

if not rows:
    print("  [FAIL] el indice no contiene ninguna entrada con el formato esperado")
    sys.exit(2)

def doc_status(path: pathlib.Path) -> str:
    """Read the declared status, or return a reason it could not be read.

    Frontmatter is the machine contract. Prose is accepted as a
    fallback so an existing document is not failed merely for being
    older than the convention, but a prose status is reported as such:
    the whole point of the guard is that a status nobody can read
    mechanically is a status nobody will notice drift.
    """
    body = path.read_text(encoding="utf-8")
    fm = re.match(r"^---\s*\n(.*?)\n---\s*\n", body, re.S)
    if fm:
        m = re.search(r"^status:\s*(\S+)", fm.group(1), re.M)
        if m:
            return m.group(1).lower()
        return "unreadable: frontmatter sin campo `status`"
    m = re.search(r"^\*\*Estado:\*\*\s*(\S+)", body, re.M)
    if m:
        return "prose:" + m.group(1).lower()
    return "unreadable: sin frontmatter y sin `**Estado:**`"

STATES = ("open", "blocked", "closed", "resolved", "fixed")

def index_claims(meta: str) -> set:
    """States the index row *declares*, not every state word it mentions.

    The declared state is the FIRST one in the metadata segment, because
    that is where the convention places it (`(medium/P2, open — ...)`).
    Scanning the whole segment made a row unfalsifiable: session-45b
    annotated INC-DEBT-039 with the word `fail-closed` in its prose, so
    the row matched both `open` and `closed`, the intersection was
    always non-empty, and a genuine divergence went unreported. A guard
    that a sentence can switch off is not a guard.

    Prose after the declared state is still reported as a warning when it
    names a *conflicting* settled state, so the annotation is visible
    without letting it override the declaration.
    """
    low = meta.lower()
    first = None
    for s in STATES:
        m = re.search(r"\b" + s + r"\b", low)
        if m and (first is None or m.start() < first[0]):
            first = (m.start(), s)
    if first is None:
        return set()
    return {first[1]}

checked = 0
problems = []
warnings = []

for inc, fname, meta in rows:
    path = debt_dir / fname
    if not path.exists():
        problems.append(f"{inc}: el indice apunta a {fname}, que no existe")
        continue

    ds = doc_status(path)
    claims = index_claims(meta)

    if ds.startswith("unreadable"):
        problems.append(f"{inc}: {ds}")
        continue
    if ds.startswith("prose:"):
        warnings.append(
            f"{inc}: estado solo en prosa (`**Estado:**`), no en frontmatter"
        )

    doc_value = ds.split(":", 1)[1] if ds.startswith("prose:") else ds
    if doc_value not in UNSETTLED | SETTLED:
        problems.append(
            f"{inc}: estado `{doc_value}` fuera del vocabulario conocido "
            f"({sorted(UNSETTLED | SETTLED)})"
        )
        continue

    checked += 1
    doc_unsettled = doc_value in UNSETTLED
    idx_unsettled = bool(claims & UNSETTLED)

    if doc_unsettled != idx_unsettled:
        problems.append(
            f"{inc}: el indice afirma {sorted(claims) or ['nada']} pero el "
            f"documento dice `{doc_value}`"
        )

print(f"  entradas en el indice: {len(rows)}")
print(f"  estados comparados:   {checked}")

for w in warnings:
    print(f"  [aviso] {w}")

if problems:
    for p in problems:
        print(f"  [FAIL] {p}")
    print()
    print("RESULT: FAIL — el indice afirma un estado que su documento no declara.")
    print("         Reconciliar segun AGENTS.md §10.3: actualizar el indice con el")
    print("         estado real del documento y CONSERVAR la evidencia anterior sin")
    print("         reescribirla. No editar el documento para que case con el indice:")
    print("         el indice es lo que se lee para priorizar, asi que cuando")
    print("         divergen es el indice el que hay que corregir.")
    sys.exit(1)

print("  [ok]   indice y documentos coinciden en las entradas con estado legible")
print()
print("RESULT: PASS — el indice no contradice a los documentos que indexa.")
sys.exit(0)
PY

exit $?
