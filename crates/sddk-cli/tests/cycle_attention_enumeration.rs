//! E2E: `sddk cycle list` must make a cycle waiting on a human DECIDABLE.
//!
//! INC-DEBT-067 closed the loop one step further back. `cycle narrative` now
//! derives what it says instead of asserting "Cycle completed.", and it fails
//! closed for a cycle that does not exist — but the operator only reaches that
//! surface if they ALREADY KNOW the cycle id.
//!
//! Measured on the real ledger of `p-63676b11dc0ef88f` before this change:
//! **2 cycles** hold an unresolved `approval.capability.requested`
//! (`c0-t01-pointer-mutation`, `c3n-production-boundary-certification`) and
//! **0 of the 662 lines** that `cycle list` emits mention `runtime_state` or
//! `approval`. A cycle blocked on a person was byte-identical to one that
//! needed nothing; among 29 `OPEN` cycles, the blocked one could only be found
//! by already knowing its id.
//!
//! The count is 2, not 3, and the correction is worth keeping: a hand-written
//! SQL query over `events_v1` said 3, naming `cl-build-identity` as well. The
//! product was right and the query was wrong — `cl-build-identity` had already
//! been decided. The authority for "how many decisions are open" is
//! `sddk approval list`, and this enumeration now agrees with it by
//! construction rather than by coincidence.
//!
//! Both of those two had never been surfaced to anyone, including to the
//! agent that wrote the surface meant to show them.
//!
//! The property here is not "the label appears". It is the pair:
//!
//!   1. a cycle awaiting a decision is DISTINGUISHABLE from one that is not;
//!   2. a cycle whose state could not be derived SAYS SO, and is never rendered
//!      as "nothing pending" — which is the exact lie INC-DEBT-067 was about,
//!      and which the natural rendering of a failed derivation produces.

use std::path::{Path, PathBuf};
use std::process::Command;

use sddk_cli::LedgerEventInput;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

const REMOTE: &str = "https://example.test/attention-fixture";
const CAPABILITY: &str = "surface.cycle_state#cycle_supersede";

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sddk-attention-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Sandbox { dir }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn run(dir: &Path, args: &[&str]) -> (i32, String) {
    let xdg = dir.join(".xdg");
    let out = Command::new(bin())
        .args(args)
        .env_remove("SDDK_DATA_DIR")
        .env_remove("SDDK_STATE_HOME")
        .env("XDG_DATA_HOME", xdg.join("data"))
        .env("XDG_STATE_HOME", xdg.join("state"))
        .env("XDG_CACHE_HOME", xdg.join("cache"))
        .current_dir(dir)
        .output()
        .expect("sddk binary runs");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn adopt(dir: &Path) -> String {
    let (code, out) = run(
        dir,
        &[
            "adopt", "apply", "--root", ".", "--scope", ".", "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "adopt apply must succeed: {out}");
    out.lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .expect("adopt apply prints project_id")
        .trim()
        .to_string()
}

fn start_cycle(dir: &Path, name: &str) -> String {
    let (code, out) = run(
        dir,
        &[
            "cycle", "start", "--root", ".", "--scope", ".", "--name", name, "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "cycle start must succeed: {out}");
    out.lines()
        .find_map(|l| l.strip_prefix("cycle_id: "))
        .expect("cycle start prints cycle_id")
        .trim()
        .to_string()
}

fn ledger_path(dir: &Path, project_id: &str) -> PathBuf {
    dir.join(".xdg/state/sddk/projects")
        .join(project_id)
        .join("ledger.sqlite")
}

fn list(dir: &Path) -> (i32, String) {
    run(
        dir,
        &[
            "cycle", "list", "--root", ".", "--scope", ".", "--remote", REMOTE,
        ],
    )
}

/// Plants an unresolved `approval.capability.requested` for `cycle_id`,
/// through the storage API the product itself uses.
///
/// An earlier version of this fixture INSERTed the row over raw SQL. It was
/// rejected by the schema (`content_hash LIKE 'sha256:%'`), and when the hash
/// was faked to satisfy it, the event was **stored but not served**:
/// `derive_cycle_summary` reads through `list_cycle_events`, which walks a
/// hash-linked chain, so a row whose `chain_hash` was invented is invisible to
/// the very reader the test is about. The listing then showed
/// `runtime_state: unknown` for a cycle that was perfectly readable — the
/// fail-closed path masking a broken fixture.
///
/// That is worth the detour: it is the same shape as the defect this block
/// fixes. A ledger row that exists but cannot be read is not a fact, and
/// rendering it as "state unknown" is indistinguishable from a row that
/// genuinely has no derivable state. So the fixture emits a CANONICAL event
/// through `emit_canonical_event`, and if the chain ever breaks again the
/// failure is a loud one in the fixture instead of a silent lie in the output.
fn plant_pending_approval(ledger: &Path, project_id: &str, cycle_id: &str, tag: &str) {
    let storage = sddk_cli::Storage::open(ledger).expect("the sandbox ledger opens");
    let input = LedgerEventInput {
        event_id: format!("approval-planted-{tag}"),
        project_id: project_id.to_string(),
        cycle_id: Some(cycle_id.to_string()),
        frame_id: format!("frame:planted-{tag}"),
        command_id: format!("planted-{tag}"),
        actor: "tester".into(),
        actor_ref: None,
        event_type: "approval.capability.requested".into(),
        occurred_at: "2026-01-01T00:00:00Z".into(),
        state_before: None,
        state_after: None,
        // The same two fields `derive_cycle_summary` reads. A request without
        // a `request_hash` is not pending-but-undecided, it is not a request.
        payload: serde_json::json!({
            "cycle_id": cycle_id,
            "capability": CAPABILITY,
            "request_hash": format!("sha256:planted-{tag}"),
            "expires_at": "2099-01-01T00:00:00Z",
        }),
        causation_id: None,
        correlation_id: None,
    };
    storage
        .emit_canonical_event(&input)
        .expect("the approval request must be emittable, or this test measures nothing");
}

fn plant_unreadable_cycle(ledger: &Path, project_id: &str) {
    let conn = rusqlite::Connection::open(ledger).unwrap();
    conn.execute(
        "INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
         SELECT 'unreadable-cycle', p.project_id, w.workspace_id, 'OPEN', 'explore', '{}', '2026-01-01', '2026-01-01'
         FROM projects p, workspaces w
         WHERE p.project_id = w.project_id AND p.project_id = ?1",
        rusqlite::params![project_id],
    )
    .expect("the malformed row must be plantable");
}

/// Plants a cycle that is perfectly READABLE and has NO ledger events at all.
///
/// Two properties have to hold at once, and `cycle start` cannot produce
/// either combination: starting a cycle emits its own initial event, so every
/// started cycle has events.
///
/// That is not a detail. The first version of this file had no subject for the
/// "no events" case at all, and the falsifier found it: mutation M5 removes the
/// producer's entry for event-less cycles, the whole suite stayed GREEN, and
/// the mutation was counting as a detection because the script only asked
/// whether a named test still passed. A guard that cannot see the defect it
/// names is worse than no guard, because it buys the coverage on paper.
///
/// The manifest is copied from a row the product itself wrote, so the planted
/// cycle is deserializable by construction instead of by a hand-written JSON
/// literal that could quietly stop being a `CycleManifest`.
fn plant_eventless_cycle(ledger: &Path, donor: &str, planted: &str) {
    let storage = sddk_storage::Storage::open(ledger).expect("the sandbox ledger opens");
    let mut record = storage
        .get_cycle(donor)
        .expect("the donor cycle is readable, which is the point");
    record.manifest.cycle_id = planted.to_string();
    storage
        .insert_cycle(&record)
        .expect("an event-less row must be plantable");
}

/// The core property. A cycle holding an unresolved approval request must be
/// visible in the ENUMERATION — with the capability named — while a cycle with
/// nothing pending must be visibly different.
///
/// Before this change both rows were `status: OPEN` and nothing else, so the
/// operator had to already know the id to learn a person was waiting.
#[test]
fn a_cycle_awaiting_a_decision_is_distinguishable_in_the_enumeration() {
    let s = Sandbox::new("pending");
    let project_id = adopt(&s.dir);
    let blocked = start_cycle(&s.dir, "blocked-on-a-human");
    let quiet = start_cycle(&s.dir, "needs-nothing");
    let ledger = ledger_path(&s.dir, &project_id);
    plant_pending_approval(&ledger, &project_id, &blocked, "blocked");

    let (code, out) = list(&s.dir);
    assert_eq!(code, 0, "the enumeration must render: {out}");

    assert!(
        out.contains(&format!("pending_approval: {CAPABILITY}")),
        "the capability awaiting a decision must be named in the listing, or the \
         operator reads that a cycle exists but not that it is blocked on them: {out}"
    );
    assert!(
        out.contains("pending_human_decisions: 1"),
        "the listing must declare how many cycles wait on a person, so the \
         question has an answer without reading every row: {out}"
    );
    // The blocked row and the quiet row must be distinguishable. This is the
    // assertion that fails if the label is added to every row, or to none.
    assert!(
        out.contains("runtime_state: approval-waiting"),
        "the blocked cycle must carry the derived label: {out}"
    );
    let quiet_block = out
        .split(&format!("cycle: {quiet}"))
        .nth(1)
        .expect("the quiet cycle is listed")
        .split("cycle: ")
        .next()
        .unwrap_or_default();
    assert!(
        !quiet_block.contains(CAPABILITY),
        "a cycle with nothing pending must not be shown as waiting: {quiet_block}"
    );
    assert!(
        quiet_block.contains("runtime_state_known: true"),
        "and it must say its state IS known, which is what makes the other \
         rows' `unknown` mean something: {quiet_block}"
    );
}

/// Fail closed. An unreadable manifest makes the derivation fail, and the
/// natural rendering of a failure is the empty string — which reads as
/// "nothing is pending". That is the same lie as INC-DEBT-067, one layer down
/// and one level of indirection further from anyone who would notice.
///
/// A cycle the listing cannot read must be marked `unknown` and counted
/// separately, never silently folded into "nothing needs you".
#[test]
fn a_cycle_whose_state_cannot_be_derived_says_so_and_is_counted_apart() {
    let s = Sandbox::new("undetermined");
    let project_id = adopt(&s.dir);
    start_cycle(&s.dir, "readable-one");
    let ledger = ledger_path(&s.dir, &project_id);
    plant_unreadable_cycle(&ledger, &project_id);

    let (code, out) = list(&s.dir);
    assert_eq!(
        code, 0,
        "an unreadable row must not take the listing down: {out}"
    );

    assert!(
        out.contains("runtime_state: unknown"),
        "a row whose state could not be derived must SAY so: an empty runtime \
         state is the rendering of 'nothing is pending', and these two must \
         never look alike: {out}"
    );
    assert!(
        out.contains("undetermined_runtime_states: 1"),
        "the undetermined rows must be counted, and SEPARATELY from the \
         pending ones — the two failures look alike from outside and must not \
         be added together: {out}"
    );
    assert!(
        out.contains("pending_human_decisions: 0"),
        "an unreadable cycle is not a cycle that needs nothing, and must not be \
         counted as one: {out}"
    );
    assert!(
        out.contains("runtime_state_known: false"),
        "the row must be marked, not only counted: {out}"
    );
}

/// A cycle with no ledger events is QUIET and KNOWN, not UNDETERMINED.
///
/// This is the distinction the whole surface turns on, and it is invisible
/// unless a cycle with zero events exists — hence the planted subject, and
/// hence the fourth test. See `plant_eventless_cycle` for why the other three
/// could not have covered it.
///
/// The cheap way to write the producer is to answer only for the cycles it saw
/// events for. Then "this cycle needs nothing" and "this cycle is unreadable"
/// render identically, and only one of them is true. On the real ledger the
/// event-less rows are the majority: 109 cycles, 651 events, and most rows
/// carry nothing that would ever produce a label. Collapsing them into
/// "unknown" would make the counter useless and, worse, would teach the
/// operator to ignore it.
#[test]
fn a_cycle_with_no_events_is_quiet_and_known_rather_than_undetermined() {
    let s = Sandbox::new("eventless");
    let project_id = adopt(&s.dir);
    let donor = start_cycle(&s.dir, "donor-of-a-manifest");
    let ledger = ledger_path(&s.dir, &project_id);
    plant_eventless_cycle(&ledger, &donor, "eventless-cycle");

    let (code, out) = list(&s.dir);
    assert_eq!(code, 0, "the enumeration must render: {out}");

    let row = out
        .split("cycle: eventless-cycle")
        .nth(1)
        .expect("the event-less cycle is listed at all")
        .split("cycle: ")
        .next()
        .unwrap_or_default();

    assert!(
        row.contains("manifest_readable: true"),
        "the row is perfectly readable — having no events is the ONLY reason \
         its runtime state could be undetermined, so if this fails the row is \
         not the subject this test means it is: {row}"
    );
    assert!(
        row.contains("runtime_state_known: true"),
        "no events is a fact ABOUT the cycle, not a failure to read it. A row \
         we cannot derive is one we know nothing about; this one we know has \
         nothing pending, and the two must not render alike: {row}"
    );
    assert!(
        !row.contains("runtime_state: unknown"),
        "and specifically it must not borrow the wording reserved for a row the \
         enumeration could not vouch for: {row}"
    );
    assert!(
        !row.contains("pending_approval:"),
        "naming a capability here would invent a decision nobody requested: {row}"
    );
    assert!(
        out.contains("undetermined_runtime_states: 0"),
        "and it must not be counted among the undetermined rows — that counter \
         is the operator's only measure of how much of the listing is dark, \
         and padding it with quiet rows makes it lie: {out}"
    );
}

/// The declared counter must agree with the rows, or it is a claim rather than
/// evidence. This is the same cross-check the release gate 3b performs between
/// `cycles:` and the emitted rows, applied to the new declaration.
#[test]
fn the_declared_counter_agrees_with_the_rows_it_counts() {
    let s = Sandbox::new("counter");
    let project_id = adopt(&s.dir);
    let a = start_cycle(&s.dir, "counter-a");
    let b = start_cycle(&s.dir, "counter-b");
    let ledger = ledger_path(&s.dir, &project_id);
    plant_pending_approval(&ledger, &project_id, &a, "a");
    plant_pending_approval(&ledger, &project_id, &b, "b");

    let (code, out) = list(&s.dir);
    assert_eq!(code, 0, "the enumeration must render: {out}");

    let declared: usize = out
        .lines()
        .find_map(|l| l.strip_prefix("pending_human_decisions: "))
        .expect("the counter is declared")
        .trim()
        .parse()
        .expect("the counter is an integer");
    let named = out.matches("pending_approval: ").count();
    assert_eq!(
        declared, 2,
        "two planted requests must be declared as two: {out}"
    );
    assert_eq!(
        named, declared,
        "the declared counter and the named rows come from the same pass and \
         must not be able to disagree: declared {declared}, rows {named}\n{out}"
    );
    assert!(
        out.contains(&a) && out.contains(&b),
        "both blocked cycles must be named, not counted: {out}"
    );
}
