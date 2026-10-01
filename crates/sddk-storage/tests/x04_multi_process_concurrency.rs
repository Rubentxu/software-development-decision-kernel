//! C3l.5 / AIW-S8 X04 — multi-process lease concurrency on a durable ledger.
//!
//! # What boundary this crosses
//!
//! The pre-existing `aiw_s8_x04_two_cli_concurrency.rs` proves **intra-process**
//! lease semantics with `Arc<InMemoryLeaseStore>` and a `MockClock`. Two `Arc`
//! clones are not two processes: they share a mutex and a heap. Nothing here
//! can observe what happens when two *independent* OS processes contend.
//!
//! This file crosses that boundary:
//!
//! - `SqliteLeaseStore` on a **file**, reopened per operation, so no shared
//!   memory exists between contenders;
//! - ≥2 **real PIDs**, asserted via `std::process::id()`, never assumed;
//! - a **real clock** for the expiry path (`MockClock` would make G5 vacuous).
//!
//! `boundary_class = PROCESS / SQLITE_DURABLE`.
//!
//! # Gates
//!
//! G1 first-writer · G2 monotonic fencing · G3 loser does not mutate ·
//! G4 crash of the winner · G5 expiry/reacquire · G6 reopen ·
//! G7 no `database is locked` under contention · G8 no state divergence.

use std::process::{Command, Stdio};

use sddk_domain::ports::{LeaseError, LeaseStore};
use sddk_storage::agent_lease_store::{SqliteLeaseStore, peek};

const DB: &str = "agent_leases.sqlite";
const CYCLE: &str = "x04-cycle";
const OWNER_A: &str = "cli-a";
const OWNER_B: &str = "cli-b";

/// A contender running in **this** process.
fn store(path: &std::path::Path) -> SqliteLeaseStore {
    SqliteLeaseStore::open(path).expect("open lease store")
}

/// Spawns a genuinely separate OS process that races for the lease and prints
/// one machine-readable line: `ACQUIRED <token>` or `CONFLICT <owner> <token>`.
///
/// The child re-executes this very test binary, so the code under contention is
/// the same code the parent would have run.
fn spawn_contender(path: &std::path::Path, owner: &str, now_ms: i64, expires_ms: i64) -> String {
    let out = Command::new(std::env::current_exe().expect("current exe"))
        .args([
            "--exact",
            "x04_child_contender_body",
            "--nocapture",
            "--ignored",
        ])
        .env("SDDK_X04_DB", path)
        .env("SDDK_X04_OWNER", owner)
        .env("SDDK_X04_NOW", now_ms.to_string())
        .env("SDDK_X04_EXPIRES", expires_ms.to_string())
        .env("SDDK_X04_CHILD", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("spawn contender");
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// The child's body. `#[ignore]`d so it never runs in the ordinary suite.
#[test]
#[ignore = "child process body; spawned explicitly by x04_child_contender_body"]
fn x04_child_contender_body() {
    if std::env::var("SDDK_X04_CHILD").as_deref() != Ok("1") {
        return;
    }
    let path = std::path::PathBuf::from(std::env::var("SDDK_X04_DB").expect("SDDK_X04_DB"));
    let owner = std::env::var("SDDK_X04_OWNER").expect("SDDK_X04_OWNER");
    let now: i64 = std::env::var("SDDK_X04_NOW")
        .expect("SDDK_X04_NOW")
        .parse()
        .expect("now");
    let expires: i64 = std::env::var("SDDK_X04_EXPIRES")
        .expect("SDDK_X04_EXPIRES")
        .parse()
        .expect("expires");
    let pid = std::process::id();
    match store(&path).acquire(CYCLE, &owner, now, expires) {
        Ok(rec) => println!("CHILD pid={pid} ACQUIRED {}", rec.fencing_token),
        Err(LeaseError::Conflict {
            owner,
            fencing_token,
        }) => {
            println!("CHILD pid={pid} CONFLICT {owner} {fencing_token}")
        }
        Err(e) => println!("CHILD pid={pid} ERROR {e}"),
    }
}

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = dir.path().join(DB);
    (dir, p)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .expect("clock")
}

// ── G1/G3: first-writer wins, and the loser does not mutate ────────────────

/// Two independent processes race. Exactly one wins, the loser is told who
/// holds it, and the durable state still shows only the winner.
#[test]
fn g1_two_real_processes_race_exactly_one_wins_and_loser_does_not_mutate() {
    let (_d, path) = temp_db();
    let now = now_ms();

    // This process takes it first so the child's race is decided, not racy.
    let winner = store(&path)
        .acquire(CYCLE, OWNER_A, now, now + 60_000)
        .expect("parent acquires");
    assert_eq!(winner.fencing_token, 1);

    let child_out = spawn_contender(&path, OWNER_B, now, now + 60_000);
    assert!(
        child_out.contains(&format!("CONFLICT {OWNER_A} 1")),
        "child must be told the winner holds token 1; got: {child_out}"
    );
    assert!(
        !child_out.contains("ACQUIRED"),
        "the loser must not acquire: {child_out}"
    );

    // G3: the durable state was not touched by the loser.
    let persisted = peek(&path, CYCLE).expect("peek").expect("row exists");
    assert_eq!(persisted.owner, OWNER_A, "loser must not change the owner");
    assert_eq!(
        persisted.fencing_token, 1,
        "loser must not advance the fencing token"
    );
}

/// A child process really is a different PID. Without this, "multi-process"
/// would be an unfalsifiable label.
#[test]
fn g1_child_is_genuinely_a_second_pid() {
    let (_d, path) = temp_db();
    let now = now_ms();
    store(&path)
        .acquire(CYCLE, OWNER_A, now, now + 60_000)
        .expect("parent acquires");
    let out = spawn_contender(&path, OWNER_B, now, now + 60_000);
    let child_pid = child_pid_of(&out);
    assert!(
        !child_pid.is_empty(),
        "the contender must report its PID: {out}"
    );
    assert_ne!(
        child_pid,
        std::process::id().to_string(),
        "the contender must be a different OS process"
    );
}

// ── G2/G6: monotonic fencing across a reopen ──────────────────────────────

/// Release + reacquire across a **fresh store instance** (as a new process
/// would) must advance the token, never reuse it.
#[test]
fn g2_fencing_is_monotonic_across_reopen() {
    let (_d, path) = temp_db();
    let now = now_ms();

    let first = store(&path)
        .acquire(CYCLE, OWNER_A, now, now + 60_000)
        .expect("acquire 1");
    assert_eq!(first.fencing_token, 1);

    // G6: every call to `store()` opens a brand-new connection, so the second
    // acquisition is already made by a *fresh* store — as a new process would.
    let reopened = store(&path);
    let second = reopened
        .acquire(CYCLE, OWNER_A, now + 1, now + 60_000)
        .expect("reacquire same owner");
    assert!(
        second.fencing_token > first.fencing_token,
        "reacquire must advance the token ({} -> {})",
        first.fencing_token,
        second.fencing_token
    );

    // A DIFFERENT owner cannot take over while A's lease is still live.
    // (Written the other way round this test asserted B could steal a live
    // lease, which the store correctly refuses — the first run of this test
    // failed on its own bad premise, not on a defect.)
    let out = spawn_contender(&path, OWNER_B, now + 2, now + 60_000);
    assert!(
        out.contains(&format!("CONFLICT {OWNER_A} {}", second.fencing_token)),
        "a live lease blocks a different owner: {out}"
    );

    // Hand over properly: A releases, then B takes over with a greater token.
    assert!(
        store(&path)
            .release(CYCLE, OWNER_A, second.fencing_token)
            .expect("a releases")
    );
    let third = store(&path)
        .acquire(CYCLE, OWNER_B, now + 3, now + 60_000)
        .expect("b takes over after release");
    assert!(
        third.fencing_token > second.fencing_token,
        "a new owner must get a strictly greater token ({} -> {})",
        second.fencing_token,
        third.fencing_token
    );
}

// ── G4: crash of the winner ───────────────────────────────────────────────

/// A winner that dies WITHOUT releasing must still hold the lease, so nobody
/// else can take it before expiry. `expire_lease` simulates the crash by
/// simply never being called — the durable row is the proof.
#[test]
fn g4_crashed_winner_holds_the_lease_until_expiry() {
    let (_d, path) = temp_db();
    let now = now_ms();

    // "Winner" takes the lease and vanishes (no release, store dropped).
    let held = store(&path)
        .acquire(CYCLE, OWNER_A, now, now + 60_000)
        .expect("winner acquires");
    drop(held); // crash: no release

    // A separate process must be refused while the lease is live.
    let out = spawn_contender(&path, OWNER_B, now + 1, now + 60_000);
    assert!(
        out.contains(&format!("CONFLICT {OWNER_A} 1")),
        "a crashed winner still fences others out: {out}"
    );

    let persisted = peek(&path, CYCLE)
        .expect("peek")
        .expect("row survives crash");
    assert_eq!(persisted.owner, OWNER_A);
    assert_eq!(
        persisted.fencing_token, 1,
        "a crash must not let anyone reuse the token"
    );
}

// ── G5: expiry / reacquire, on a real clock ───────────────────────────────

/// Once expired, another process may take it, with a strictly greater token.
#[test]
fn g5_expired_lease_can_be_reacquired_with_a_greater_token() {
    let (_d, path) = temp_db();
    let now = now_ms();

    // A lease that is ALREADY expired when written (short window, past time).
    let dead = store(&path)
        .acquire(CYCLE, OWNER_A, now - 10_000, now - 5_000)
        .expect("acquire already-expired lease");
    assert_eq!(dead.fencing_token, 1);

    let out = spawn_contender(&path, OWNER_B, now, now + 60_000);
    assert!(
        out.contains("ACQUIRED 2"),
        "an expired lease must be acquirable with the next token: {out}"
    );
    let persisted = peek(&path, CYCLE).expect("peek").expect("row exists");
    assert_eq!(persisted.owner, OWNER_B);
    assert_eq!(persisted.fencing_token, 2);
}

// ── Release compare-and-swap (G3 for the release path) ───────────────────

/// A stale holder cannot release the current owner's lease. Without the token
/// check this is the classic fencing hole.
#[test]
fn g3_stale_token_cannot_release_the_current_owners_lease() {
    let (_d, path) = temp_db();
    let now = now_ms();

    let first = store(&path)
        .acquire(CYCLE, OWNER_A, now, now + 60_000)
        .expect("a acquires");
    let second = store(&path)
        .acquire(CYCLE, OWNER_A, now + 1, now + 60_000)
        .expect("a reacquires with a new token");

    // The OLD token is stale and must not free the CURRENT lease.
    let released_stale = store(&path)
        .release(CYCLE, OWNER_A, first.fencing_token)
        .expect("release with stale token");
    assert!(
        !released_stale,
        "a stale fencing token must not release the lease"
    );
    let still_held = peek(&path, CYCLE).expect("peek").expect("still held");
    assert_eq!(still_held.owner, OWNER_A);
    assert_eq!(still_held.fencing_token, second.fencing_token);

    // The current token does release it — the holder is cleared but the ROW
    // and its token survive, because fencing is monotonic per cycle.
    assert!(
        store(&path)
            .release(CYCLE, OWNER_A, second.fencing_token)
            .expect("release with current token")
    );
    let freed = peek(&path, CYCLE).expect("peek").expect("row must survive");
    assert_eq!(freed.owner, "", "release must clear the holder");
    assert_eq!(
        freed.fencing_token, second.fencing_token,
        "release must NOT reset the fencing counter"
    );

    // …which is what makes the next acquire strictly greater, not 1 again.
    let after = store(&path)
        .acquire(CYCLE, OWNER_B, now + 2, now + 60_000)
        .expect("next owner acquires");
    assert_eq!(
        after.fencing_token,
        second.fencing_token + 1,
        "a released lease must not restart the fencing counter at 1"
    );
}

// ── G7: WAL / busy timeout under contention ──────────────────────────────

/// Many independent processes hammering the same cycle must never surface
/// `database is locked`; exactly one wins each round.
#[test]
fn g7_concurrent_processes_never_surface_database_locked() {
    let (_d, path) = temp_db();
    let now = now_ms();

    let mut children = Vec::new();
    for i in 0..6 {
        children.push(
            Command::new(std::env::current_exe().expect("exe"))
                .args([
                    "--exact",
                    "x04_child_contender_body",
                    "--nocapture",
                    "--ignored",
                ])
                .env("SDDK_X04_DB", &path)
                .env("SDDK_X04_OWNER", format!("cli-{i}"))
                .env("SDDK_X04_NOW", now.to_string())
                .env("SDDK_X04_EXPIRES", (now + 60_000).to_string())
                .env("SDDK_X04_CHILD", "1")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn"),
        );
    }

    let mut acquired = 0;
    let mut pids = std::collections::BTreeSet::new();
    for child in children {
        let out = child.wait_with_output().expect("child output");
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        assert!(
            !text.contains("database is locked"),
            "busy/lock leaked to the caller under contention: {text}"
        );
        assert!(
            !text.contains("ERROR"),
            "an unexpected error surfaced under contention: {text}"
        );
        if let Some(rest) = text.lines().find_map(|l| l.split_once("CHILD pid=")) {
            let pid = rest.1.split(' ').next().expect("pid");
            pids.insert(pid.to_string());
        }
        if text.contains("ACQUIRED") {
            acquired += 1;
        }
    }

    assert_eq!(acquired, 1, "exactly one process may win the lease");
    assert!(
        pids.len() >= 2,
        "expected at least 2 distinct PIDs, saw {pids:?}"
    );
}

// ── G8: no state divergence ───────────────────────────────────────────────

/// Whatever each process believes it won, the durable file reports the same
/// single holder with the same token.
#[test]
fn g8_durable_state_is_identical_to_what_the_winner_believes() {
    let (_d, path) = temp_db();
    let now = now_ms();

    let mine = store(&path)
        .acquire(CYCLE, OWNER_A, now, now + 60_000)
        .expect("acquire");
    let out = spawn_contender(&path, OWNER_B, now, now + 60_000);

    let persisted = peek(&path, CYCLE).expect("peek").expect("row exists");
    assert_eq!(persisted.owner, mine.owner);
    assert_eq!(persisted.fencing_token, mine.fencing_token);
    assert!(out.contains(&format!("CONFLICT {} {}", mine.owner, mine.fencing_token)));
    assert!(out.contains(&format!("pid={}", child_pid_of(&out))));
}

/// Extracts the child PID from a contender's stdout.
fn child_pid_of(out: &str) -> String {
    out.lines()
        .find_map(|l| {
            l.strip_prefix("CHILD pid=")
                .map(|r| r.split(' ').next().unwrap_or(""))
        })
        .expect("child pid")
        .to_string()
}
