//! El sobre que un productor entrega a quien certifica.
//!
//! # Qué se está midiendo
//!
//! Que el sobre diga **lo que pasó** y no **lo que haría falta que pasara**.
//!
//! El riesgo de este comando no es que produzca un sobre mal formado: es que
//! produzca un sobre *completo* con campos que nadie rellenó. Un sobre con nueve
//! campos, todos presentes, donde tres los puso SDDK sin que nadie se lo pidiera,
//! es peor que no tener sobre — el certificador lee la firma del productor y no
//! hay productor detrás.
//!
//! Los seis casos de abajo atacan cada una de las tres cosas que el sobre
//! **no** puede hacer: no rellenar lo que no sabe, no publicar, y no decidir.
//!
//! **No se llama `e2e`:** corre por `run_from`, en proceso. El nombre tiene que
//! decir lo que hay.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;

use sddk_cli::{CommandOutput, run_from};
use tempfile::TempDir;

/// Un repo con un producto versionado y un artefacto real que hashear.
fn repo() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"2.1.0\"\n",
    )
    .expect("write Cargo.toml");
    fs::create_dir_all(dir.path().join("dist")).expect("mkdir dist");
    fs::write(dir.path().join("dist/material.tar.gz"), b"contenido real\n")
        .expect("write artifact");
    git(dir.path());
    dir
}

fn git(root: &Path) {
    for args in [
        vec!["init", "-q", "."],
        vec!["add", "-A"],
        vec![
            "-c",
            "user.email=a@b",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "init",
        ],
    ] {
        let code = Command::new("git")
            .args(&args)
            .current_dir(root)
            .status()
            .expect("git")
            .code()
            .unwrap_or(-1);
        assert!(code == 0, "git {args:?} salio con {code}");
    }
}

/// Los argumentos de un handoff válido, más lo que cada caso cambia.
fn handoff(root: &Path, extra: &[&str]) -> CommandOutput {
    let root = root.to_str().expect("utf-8 root");
    let mut args: Vec<String> = vec![
        "sddk".into(),
        "release".into(),
        "handoff".into(),
        "--root".into(),
        root.into(),
        "--scope".into(),
        ".".into(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    run_from_owned(args)
}

/// `run_from` con un vector, porque los casos cambian la lista de argumentos.
fn run_from_owned(args: Vec<String>) -> CommandOutput {
    // `run_from` toma un slice de `&str`; el sobre se construye con vida propia
    // y se pasa por referencia durante la llamada, que es lo único que hace falta.
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_from(refs)
}

fn json_of(out: &CommandOutput) -> serde_json::Value {
    serde_json::from_str(&out.stdout).expect("json valido")
}

#[test]
fn m1_el_digest_del_artefacto_es_el_de_los_bytes() {
    let dir = repo();
    let out = handoff(
        dir.path(),
        &[
            "--tag",
            "v2.1.0",
            "--naming",
            "v_prefixed",
            "--channel",
            "candidate",
            "--artifact",
            "bundle=dist/material.tar.gz",
            "--external-type",
            "prod.material/1",
            "--external-digest",
            "sha256:cafe",
            "--format",
            "json",
        ],
    );
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);

    let envelope = json_of(&out);
    let declarado = envelope["artifacts"][0]["digest"]
        .as_str()
        .expect("digest presente");
    let bytes = fs::read(dir.path().join("dist/material.tar.gz")).expect("leer artefacto");
    let real = format!("sha256:{:x}", Sha256::digest(&bytes));

    assert_eq!(
        declarado, real,
        "el digest del sobre es el sha256 de los bytes del artefacto: no es una \
         opinion sobre el artefacto, son estos bytes"
    );
    assert!(
        declarado.starts_with("sha256:") && declarado.len() == 71,
        "y tiene forma de digest, no un string que se parece: {declarado}"
    );
}

#[test]
fn m2_cambiar_los_bytes_cambia_el_digest() {
    // El caso que hace que M1 no sea una tautología: si el digest fuera un valor
    // fijo bien formado, M1 pasaría sin que nadie comprobara que depende del
    // contenido. Dos contenidos distintos tienen que dar dos digests distintos.
    let dir = repo();
    let antes = handoff(
        dir.path(),
        &[
            "--tag",
            "v2.1.0",
            "--channel",
            "candidate",
            "--artifact",
            "bundle=dist/material.tar.gz",
            "--external-type",
            "t/1",
            "--external-digest",
            "sha256:cafe",
            "--format",
            "json",
        ],
    );
    fs::write(dir.path().join("dist/material.tar.gz"), b"otro contenido\n").expect("rewrite");
    let despues = handoff(
        dir.path(),
        &[
            "--tag",
            "v2.1.0",
            "--channel",
            "candidate",
            "--artifact",
            "bundle=dist/material.tar.gz",
            "--external-type",
            "t/1",
            "--external-digest",
            "sha256:cafe",
            "--format",
            "json",
        ],
    );

    let a = json_of(&antes)["artifacts"][0]["digest"].clone();
    let b = json_of(&despues)["artifacts"][0]["digest"].clone();
    assert_ne!(
        a, b,
        "mismos flags, bytes distintos, mismo digest: el hash no está leyendo el \
         fichero, y entonces no está hasheando nada"
    );
}

#[test]
fn m3_un_productor_no_puede_entregar_en_stable() {
    let dir = repo();
    let out = handoff(
        dir.path(),
        &[
            "--tag",
            "v2.1.0",
            "--channel",
            "stable",
            "--external-type",
            "t/1",
            "--external-digest",
            "sha256:cafe",
        ],
    );
    assert_ne!(
        out.status, 0,
        "un `candidate_producer` entrega hasta `Candidate`: pasar de ahí es \
         trabajo de otro, y un sobre que lo permite no está delegando nada"
    );
    assert!(
        out.stderr.contains("candidate_producer") && out.stderr.contains("Candidate"),
        "y el rechazo tiene que decir el rol y su techo, no solo «no»: {}",
        out.stderr
    );
}

#[test]
fn m4_la_referencia_tiene_que_nombrar_la_version_del_sobre() {
    let dir = repo();
    let out = handoff(
        dir.path(),
        &[
            "--tag",
            "v9.9.9",
            "--channel",
            "candidate",
            "--external-type",
            "t/1",
            "--external-digest",
            "sha256:cafe",
        ],
    );
    assert_ne!(
        out.status, 0,
        "un sobre cuya referencia no nombra su propia version se contradice a si \
         mismo, y el que lo construyo fue SDDK"
    );
    let err = out.stderr.clone();
    assert!(
        err.contains("v2.1.0") && err.contains("v9.9.9"),
        "el rechazo dice los DOS lados —el esperado y el encontrado—, que es lo \
         que hace la corrección mecánica: {err}"
    );
}

#[test]
fn m5_sddk_no_inventa_la_identidad_del_productor() {
    let dir = repo();
    let sin_identidad = handoff(dir.path(), &["--tag", "v2.1.0", "--channel", "candidate"]);
    assert_ne!(
        sin_identidad.status, 0,
        "`--external-type` y `--external-digest` son OBLIGATORIOS: son hechos del \
         productor y SDDK no los tiene. Con default, el sobre se rellenaba entero \
         sin hablar con nadie y decía cosas que nadie dijo"
    );

    // Y el caso positivo: con ellos, el sobre los lleva **verbatim**, sin que
    // SDDK los interprete ni los reformule.
    let con_identidad = handoff(
        dir.path(),
        &[
            "--tag",
            "v2.1.0",
            "--channel",
            "candidate",
            "--external-type",
            "  espacio y todo  ",
            "--external-digest",
            "sha256:cafe",
            "--format",
            "json",
        ],
    );
    assert_eq!(con_identidad.status, 0, "stderr: {}", con_identidad.stderr);
    let envelope = json_of(&con_identidad);
    assert_eq!(
        envelope["external_handoff_type"], "  espacio y todo  ",
        "la identidad del productor se copia tal cual: es su vocabulario, y un \
         kernel que lo normaliza es un kernel que ha adoptado el vocabulario de un \
         productor"
    );
    assert_eq!(envelope["external_handoff_digest"], "sha256:cafe");
}

#[test]
fn m6_el_sobre_no_tiene_una_segunda_version() {
    // La versión del sobre tiene que ser LA del repositorio, la misma que ve
    // `release plan` y `release version matches`. Dos resoluciones que divergen
    // son dos autoridades, y la que acaba dentro del sobre es la que el
    // certificador leería como si la hubiera emitido SDDK.
    let dir = repo();
    let sobre = handoff(
        dir.path(),
        &[
            "--tag",
            "v2.1.0",
            "--channel",
            "candidate",
            "--external-type",
            "t/1",
            "--external-digest",
            "sha256:cafe",
            "--format",
            "json",
        ],
    );
    assert_eq!(sobre.status, 0, "stderr: {}", sobre.stderr);

    let matches = run_from([
        "sddk",
        "release",
        "version",
        "matches",
        "--root",
        dir.path().to_str().expect("utf-8"),
        "--scope",
        ".",
        "--tag",
        "v2.1.0",
    ]);

    assert_eq!(
        json_of(&sobre)["product_version"],
        "2.1.0",
        "el sobre lleva la version que el repositorio declara"
    );
    assert_eq!(
        matches.status, 0,
        "y el otro comando que resuelve una version tiene que responder: {}",
        matches.stderr
    );
    assert!(
        matches.stdout.contains("productVersion: 2.1.0"),
        "`release version matches` ve la MISMA version que el sobre. Si divergieran \
         estaríamos leyendo dos verdades distintas sobre el mismo repositorio: {}",
        matches.stdout
    );
}

#[test]
fn el_sobre_escrito_es_el_mismo_que_el_impreso() {
    let dir = repo();
    let out_path = dir.path().join("handoff.json");
    let out = handoff(
        dir.path(),
        &[
            "--tag",
            "v2.1.0-rc2",
            "--sequence",
            "2",
            "--channel",
            "candidate",
            "--artifact",
            "bundle=dist/material.tar.gz",
            "--external-type",
            "t/1",
            "--external-digest",
            "sha256:cafe",
            "--out",
            out_path.to_str().expect("utf-8"),
        ],
    );
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);

    let escrito: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).expect("leer sobre")).expect("json");
    let secuencia = escrito["sequence"].as_u64().expect("secuencia");
    assert_eq!(secuencia, 2, "un candidato rc2 lleva la secuencia 2");
    assert_eq!(escrito["reference"]["name"], "v2.1.0-rc2");

    // Y el digest publicado es el del sobre que se entrego, no el de otra cosa.
    let impreso = out
        .stdout
        .lines()
        .find_map(|l| l.strip_prefix("envelope_sha256: "))
        .expect("digest impreso")
        .to_owned();
    let recomputado = format!(
        "sha256:{:x}",
        Sha256::digest(
            fs::read_to_string(&out_path)
                .expect("leer")
                .trim_end()
                .as_bytes()
        )
    );
    assert_eq!(
        impreso, recomputado,
        "el digest que el comando imprime es el del sobre que escribió: publicar \
         una suma de comprobación de otra cosa es peor que no publicarla"
    );
}

#[test]
fn una_secuencia_cero_no_es_un_candidato() {
    let dir = repo();
    let out = handoff(
        dir.path(),
        &[
            "--tag",
            "v2.1.0-rc0",
            "--sequence",
            "0",
            "--channel",
            "candidate",
            "--external-type",
            "t/1",
            "--external-digest",
            "sha256:cafe",
        ],
    );
    assert_ne!(
        out.status, 0,
        "rc0 es un candidato sin predecesor: una pregunta que nadie hizo y que \
         nadie puede contestar. El primero es 1"
    );
}
