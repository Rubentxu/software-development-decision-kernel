//! E2E: `sddk project pin` (W2c, agente-secretless D2).
//!
//! Contrato:
//! - `project pin` escribe `.sddk/project-pin.json` y `resolve` pasa a reportar
//!   `identity_source: pinned` con el project_id fijado.
//! - Con pin presente, un remote con otro nombre (fork de identidad) NO cambia
//!   el project_id resuelto.
//! - Repinar a otro project_id falla sin `unpin` previo; `unpin` restaura el
//!   derive por remote.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    // Sigue el target-dir global como el resto de e2e del workspace.
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sddk-pin-e2e-{}-{}", name, std::process::id()));
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

fn run(dir: &std::path::Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(bin())
        .args(args)
        .env_remove("SDDK_PROJECT_ID")
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

#[test]
fn pin_overrides_remote_drift() {
    let s = Sandbox::new("override");

    // Resolve inicial por remote (puede no haber git: usamos --remote explícito).
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@EXAMPLE.com:Org/Repo.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "resolve inicial: {out}");
    let pinned_id = out
        .lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .expect("project_id line")
        .trim()
        .to_string();
    assert!(pinned_id.starts_with("p-"), "{out}");

    // Pin al id resuelto.
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "pin",
            "--root",
            ".",
            "--project-id",
            &pinned_id,
            "--reason",
            "test",
        ],
    );
    assert_eq!(code, 0, "pin: {out}");
    assert!(s.dir.join(".sddk/project-pin.json").exists());

    // Resolve ahora reporta pinned y el MISMO id incluso con remote distinto.
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@other.example:Totally/Different.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "resolve tras pin: {out}");
    assert!(
        out.contains("identity_source: pinned"),
        "debe reportar pinned: {out}"
    );
    assert!(
        out.contains(&format!("project_id: {pinned_id}")),
        "debe mantener el id pineado: {out}"
    );

    // Y con un remote que derivaría a otro id: sigue el pin.
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "pin",
            "--root",
            ".",
            "--project-id",
            "p-other0000000000000000000000000000000000000000000000000000000000.00",
        ],
    );
    assert_ne!(code, 0, "repin a otro id debe fallar: {out}");
}

#[test]
fn unpin_restores_remote_derivation() {
    let s = Sandbox::new("unpin");

    // Sin pin: derive por remote A.
    let (code, out_a) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@example.com:org/a.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "{out_a}");
    let id_a = out_a
        .lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .unwrap()
        .trim()
        .to_string();

    // Sin pin, remote B da OTRO id (precondición del fork).
    let (code, out_b) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@example.com:org/b.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "{out_b}");
    let id_b = out_b
        .lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .unwrap()
        .trim()
        .to_string();
    assert_ne!(id_a, id_b, "remotes distintos deben derivar ids distintos");

    // Pin a id_a; remote B ya no mueve la identidad.
    let (code, out) = run(
        &s.dir,
        &["project", "pin", "--root", ".", "--project-id", &id_a],
    );
    assert_eq!(code, 0, "{out}");
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@example.com:org/b.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains(&format!("project_id: {id_a}")), "{out}");

    // Unpin: vuelve el derive (remote B da id_b otra vez).
    let (code, out) = run(&s.dir, &["project", "unpin", "--root", "."]);
    assert_eq!(code, 0, "{out}");
    assert!(!s.dir.join(".sddk/project-pin.json").exists());
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@example.com:org/b.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains(&format!("project_id: {id_b}")), "{out}");
}
