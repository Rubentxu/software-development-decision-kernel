//! R1–R4: `sddk vault graph` must not show one cycle and imply it is the only one.
//!
//! INC-DEBT-060 F63's family, third surface. Measured in session-69h with two
//! vaults: an **acyclic** one where `vault graph` looks perfectly correct, and a
//! one with **two distinct cycles** where `sample_cycle` returns one of them and
//! never says how many there are. `find_sample_cycle` returns the first cycle it
//! finds and stops, and `GraphView` has no count field at all.
//!
//! These are **RED today** (R4 excepted, see below) at the CLI level, over real
//! vaults indexed with `vault index`.
//!
//! R4 is a **characterization test that passes today**, and saying otherwise would
//! be false: it pins the acyclic output so the fix cannot quietly change it. That
//! is the case the first fixture accidentally exercised and declared healthy.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

struct Sandbox {
    root: PathBuf,
    vault: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("sddk-vault-graph-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let vault = root.join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        Sandbox { root, vault }
    }

    fn write(&self, id: &str, body: &str) {
        std::fs::write(
            self.vault.join(format!("{id}.md")),
            format!("---\nid: {id}\ntype: term\nstatus: active\n---\n# {id}\n\n{body}\n"),
        )
        .unwrap();
    }

    /// One cycle: A -> B -> C -> A.
    fn one_cycle(name: &str) -> Self {
        let s = Sandbox::new(name);
        s.write("CYC-A", "links [[CYC-B]]");
        s.write("CYC-B", "links [[CYC-C]]");
        s.write("CYC-C", "links [[CYC-A]]");
        s
    }

    /// Two disjoint cycles: A -> B -> C -> A and X -> Y -> X. The whole point is
    /// that the output looks identical whichever of the two it shows.
    fn two_cycles(name: &str) -> Self {
        let s = Sandbox::new(name);
        s.write("CYC-A", "links [[CYC-B]]");
        s.write("CYC-B", "links [[CYC-C]]");
        s.write("CYC-C", "links [[CYC-A]]");
        s.write("CYC-X", "links [[CYC-Y]]");
        s.write("CYC-Y", "links [[CYC-X]]");
        s
    }

    /// A chain with no links at all: acyclic, so the topological order exists and
    /// must keep existing.
    fn acyclic(name: &str) -> Self {
        let s = Sandbox::new(name);
        for id in ["N-000", "N-001", "N-002"] {
            s.write(id, "no links");
        }
        s
    }

    fn graph(&self, format: &str) -> (i32, String) {
        let out = Command::new(bin())
            .args([
                "vault",
                "graph",
                "--vault",
                self.vault.to_str().unwrap(),
                "--format",
                format,
                "--root",
                self.root.to_str().unwrap(),
                "--scope",
                ".",
                "--fallback-seed",
                "00000000-0000-0000-0000-000000000001",
            ])
            .env_remove("SDDK_DATA_DIR")
            .env_remove("SDDK_STATE_HOME")
            .env("XDG_DATA_HOME", self.root.join(".xdg/data"))
            .env("XDG_STATE_HOME", self.root.join(".xdg/state"))
            .env("XDG_CACHE_HOME", self.root.join(".xdg/cache"))
            .current_dir(&self.root)
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

    fn json(&self) -> serde_json::Value {
        let (code, out) = self.graph("json");
        assert_eq!(code, 0, "`vault graph --format json` must exit 0: {out}");
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("valid JSON ({e}): {out}"))
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// RED today: two cycles exist and the output names one of them. A vault with one
/// cycle and a vault with two hundred produce the same shape, so `sample_cycle`
/// owes its reader the fact that it is a sample of more than one.
#[test]
fn r1_two_cycles_declare_that_there_is_more_than_one() {
    let s = Sandbox::two_cycles("two-cycles");

    let v = s.json();

    assert_eq!(v["cyclic"], true, "{v}");
    assert!(
        v["sample_cycle"].is_array(),
        "the fixture must produce a sample cycle, or this measures nothing: {v}"
    );
    assert_eq!(
        v["multiple_cycles"], true,
        "this vault has TWO disjoint cycles and the output must say so. Today \
         `sample_cycle` names one of them and never says how many exist, so a \
         graph with one cycle and one with two hundred look identical. Got: {v}"
    );
}

/// RED today, and it is the other half of O1: **one** cycle must say it is one.
/// Declaring only «more than one» when there is more than one would leave the
/// single-cycle case still lying by omission.
#[test]
fn r2_one_cycle_declares_exactly_one() {
    let s = Sandbox::one_cycle("one-cycle");

    let v = s.json();

    assert_eq!(v["cyclic"], true, "{v}");
    assert_eq!(
        v["cycle_count"], 1,
        "this vault has exactly one cycle and must say so, not merely 'cyclic: \
         true'. A boolean alone cannot distinguish one from many, and that is \
         the whole reason the field exists. Got: {v}"
    );
    assert_eq!(
        v["multiple_cycles"], false,
        "one cycle is not multiple cycles: {v}"
    );
}

/// RED today: when the graph has cycles there IS no topological order, and the
/// output simply stops printing the line. A reader sees `cyclic: true` and a
/// missing field with nothing connecting the two, which reads like "not
/// computed" rather than "does not exist".
#[test]
fn r3_absent_topological_order_explains_itself() {
    let s = Sandbox::two_cycles("absent-order");

    let (code, out) = s.graph("text");

    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("cyclic") && out.to_lowercase().contains("true"),
        "the fixture must be cyclic: {out}"
    );
    assert!(
        out.to_lowercase().contains("topological_order")
            && (out.to_lowercase().contains("absent") || out.to_lowercase().contains("cyclic")),
        "an absent topological order must SAY that it is absent and why. Today \
         the line is simply not printed, which reads like 'not computed' rather \
         than 'does not exist because the graph is cyclic'. Got: {out}"
    );
}

/// Characterization: **passes today**, and is here to pin the acyclic output so
/// the fix cannot change it. Labelling it RED would be false — a test that
/// asserts current behaviour passes, and calling that a red would be the same
/// overclaiming this whole session keeps correcting.
#[test]
fn r4_acyclic_output_is_unchanged() {
    let s = Sandbox::acyclic("acyclic");

    let v = s.json();

    assert_eq!(v["cyclic"], false, "{v}");
    assert_eq!(v["node_count"], 3, "{v}");
    assert_eq!(v["edge_count"], 0, "{v}");
    let order = v["topological_order"]
        .as_array()
        .unwrap_or_else(|| panic!("an acyclic graph has a topological order: {v}"));
    assert_eq!(
        order.len(),
        3,
        "the order must cover every node, which is what makes it complete: {v}"
    );
    assert!(
        v.get("sample_cycle").is_none_or(|s| s.is_null()),
        "an acyclic graph has no sample cycle: {v}"
    );
    let _ = Path::new(".");
}
